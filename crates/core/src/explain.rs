//! One "explain this" request from start to finish: clean the selection, decide what it
//! is, pick the output language and allowed scripts, stream the rewrite part by part
//! through the output filter, enforce the language guard (with one stricter retry), and
//! finish with the reading grade and the meaning check.
//!
//! The UI receives a stream of `Event`s. Text arrives per part: `Delta`s while streaming,
//! then `PartDone` with the cleaned final text of that part (which replaces the streamed
//! text), or `RestartPart` when a part is being redone.

use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::engine::{Engine, EngineError, Sampling, Timings};
use crate::language::{self, AllowedScripts};
use crate::meaning;
use crate::output::StreamFilter;
use crate::prompt::{self, Level, Message, OutputLanguage};
use crate::readability;
use crate::text::{self, SelectionKind};

/// Which language to explain in.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "mode", content = "code", rename_all = "snake_case")]
pub enum OutputChoice {
    /// The selection's own language (detected).
    #[default]
    SameAsText,
    /// A fixed language code from `language::LANGUAGES`.
    Fixed(String),
}

#[derive(Debug, Clone)]
pub struct Request {
    pub text: String,
    /// The sentence or paragraph around a selected word, when the app could read it.
    pub context: Option<String>,
    pub level: Level,
    pub output: OutputChoice,
    /// Custom system prompt for this level (Settings → Model → Prompts).
    pub custom_prompt: Option<String>,
    pub sampling: Sampling,
    /// Long text is rewritten in parts of about this many words.
    pub part_words: usize,
}

impl Request {
    pub fn new(text: impl Into<String>) -> Self {
        Request {
            text: text.into(),
            context: None,
            level: Level::Plain,
            output: OutputChoice::SameAsText,
            custom_prompt: None,
            sampling: Sampling::default(),
            part_words: 180,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Started {
        kind: SelectionKind,
        /// The cleaned (and possibly shortened) text being explained.
        source: String,
        truncated: bool,
        grade_before: Option<f32>,
        parts: usize,
        /// The language the answer is written in, when known (English name).
        language: Option<String>,
    },
    /// The model is being loaded into memory (first request, or after idle unload).
    Loading,
    Delta {
        part: usize,
        text: String,
    },
    PartDone {
        part: usize,
        text: String,
    },
    RestartPart {
        part: usize,
    },
    Done {
        text: String,
        grade_after: Option<f32>,
        report: meaning::Report,
        timings: Option<Timings>,
        elapsed_ms: u64,
    },
    Error {
        message: String,
        detail: Option<String>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ExplainError {
    #[error("nothing is selected")]
    Empty,
    #[error("the model answered in another language twice; try another model")]
    LanguageDrift,
    #[error(transparent)]
    Engine(#[from] EngineError),
}

/// Runs a request, sending events to `emit`. Errors are also sent as `Event::Error`.
pub async fn run(
    engine: &mut Engine,
    req: &Request,
    emit: &mut (dyn FnMut(Event) + Send),
) -> Result<(), ExplainError> {
    let result = run_inner(engine, req, emit).await;
    if let Err(e) = &result {
        let detail = match e {
            ExplainError::Engine(EngineError::Exited { log_tail, .. })
            | ExplainError::Engine(EngineError::StartTimeout { log_tail, .. }) => {
                Some(log_tail.clone())
            }
            _ => None,
        };
        emit(Event::Error {
            message: e.to_string(),
            detail,
        });
    }
    result
}

async fn run_inner(
    engine: &mut Engine,
    req: &Request,
    emit: &mut (dyn FnMut(Event) + Send),
) -> Result<(), ExplainError> {
    let started = Instant::now();
    let cleaned = text::clean(&req.text);
    let (source, truncated) = text::truncate(&cleaned, text::MAX_CHARS);
    let source = source.to_string();
    let kind = text::classify(&source);
    if kind == SelectionKind::Empty {
        return Err(ExplainError::Empty);
    }

    let out_lang = match &req.output {
        OutputChoice::Fixed(code) => language::language(code),
        OutputChoice::SameAsText => {
            let probe = req
                .context
                .as_deref()
                .filter(|_| kind != SelectionKind::Passage)
                .unwrap_or(&source);
            language::detect(probe)
        }
    };
    let out = out_lang.map_or(OutputLanguage::SameAsText, OutputLanguage::Named);
    let allowed = AllowedScripts::for_request(
        &format!("{source}\n{}", req.context.as_deref().unwrap_or("")),
        out_lang,
    );
    let grammar = engine.supports_grammar().then(|| allowed.grammar());

    let parts: Vec<String> = match kind {
        SelectionKind::Passage => text::chunk(&source, req.part_words),
        _ => vec![source.clone()],
    };
    emit(Event::Started {
        kind,
        source: source.clone(),
        truncated,
        grade_before: (kind == SelectionKind::Passage)
            .then(|| readability::grade(&source).map(|r| r.grade))
            .flatten(),
        parts: parts.len(),
        language: out_lang.map(|l| l.name.to_string()),
    });

    if !engine.is_loaded() && engine.supports_grammar() {
        emit(Event::Loading);
    }

    let mut finished_parts = Vec::with_capacity(parts.len());
    let mut timings = None;
    for (i, part) in parts.iter().enumerate() {
        let messages = match kind {
            SelectionKind::Passage => prompt::rewrite(
                part,
                req.level,
                out,
                req.custom_prompt.as_deref(),
                Some((i + 1, parts.len())),
            ),
            _ => prompt::word_meaning(text::bare_word(part), req.context.as_deref(), out),
        };
        let mut sampling = req.sampling.clone();
        sampling.max_tokens = match kind {
            SelectionKind::Passage => {
                prompt::max_tokens_for_rewrite(text::word_count(part), req.level)
            }
            _ => 160,
        };

        let mut final_text = None;
        for attempt in 0..2 {
            let msgs: Vec<Message> = if attempt == 0 {
                messages.clone()
            } else {
                emit(Event::RestartPart { part: i });
                let mut m = messages.clone();
                m.insert(1, prompt::language_reminder(out));
                m
            };
            let mut filter = StreamFilter::new();
            let completion = engine
                .chat(&msgs, &sampling, grammar.as_deref(), |delta| {
                    let shown = filter.push(delta);
                    if !shown.is_empty() {
                        emit(Event::Delta {
                            part: i,
                            text: shown,
                        });
                    }
                })
                .await?;
            timings = completion.timings.or(timings);
            let cleaned = filter.finish();
            if allowed.violations(&cleaned).is_none() {
                final_text = Some(cleaned);
                break;
            }
            tracing::warn!(part = i, attempt, "answer drifted into another script");
        }
        let Some(text) = final_text else {
            return Err(ExplainError::LanguageDrift);
        };
        emit(Event::PartDone {
            part: i,
            text: text.clone(),
        });
        finished_parts.push(text);
    }

    let text = finished_parts.join("\n\n");
    let (grade_after, report) = match kind {
        SelectionKind::Passage => (
            readability::grade(&text).map(|r| r.grade),
            meaning::check(&source, &text),
        ),
        _ => (None, meaning::Report::default()),
    };
    emit(Event::Done {
        text,
        grade_after,
        report,
        timings,
        elapsed_ms: started.elapsed().as_millis() as u64,
    });
    Ok(())
}
