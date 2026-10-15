//! One "explain this" or "translate this" request from start to finish: clean the
//! selection, decide what it is, pick the output language and allowed scripts, stream the
//! rewrite or translation part by part through the output filter, enforce the language
//! guard (with one stricter retry), and finish with the reading grade and the meaning
//! check.
//!
//! The UI receives a stream of `Event`s. Text arrives per part: `Delta`s while streaming,
//! then `PartDone` with the cleaned final text of that part (which replaces the streamed
//! text), or `RestartPart` when a part is being redone.

use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::engine::{Engine, EngineError, Sampling, Timings};
use crate::language::{self, AllowedScripts, Language};
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

/// What the model does with the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Task {
    /// Rewrite it in plain language at a reading level (a word: what it means here).
    Explain {
        level: Level,
        output: OutputChoice,
        /// Custom system prompt for this level (Settings → Model → Prompts).
        custom_prompt: Option<String>,
    },
    /// Translate it into this language (`translate::target`). The language guard allows
    /// only the target's script (and Latin, for names), an answer handed back untranslated
    /// is retried like drift, and the meaning guard checks numbers and dates.
    Translate { to: &'static Language },
}

impl Default for Task {
    fn default() -> Self {
        Task::Explain {
            level: Level::Plain,
            output: OutputChoice::SameAsText,
            custom_prompt: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub text: String,
    /// The sentence or paragraph around a selected word, when the app could read it.
    pub context: Option<String>,
    pub task: Task,
    pub sampling: Sampling,
    /// Long text is rewritten in parts of about this many words.
    pub part_words: usize,
    /// Capitalised words in the text that are ordinary words (from the dictionary, see
    /// `meaning::single_word_names`), lowercase; the meaning check won't treat them as
    /// names.
    pub common_words: Vec<String>,
}

impl Request {
    pub fn new(text: impl Into<String>) -> Self {
        Request {
            text: text.into(),
            context: None,
            task: Task::default(),
            sampling: Sampling::default(),
            part_words: 180,
            common_words: Vec::new(),
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

    let (out_lang, allowed) = match &req.task {
        Task::Explain { output, .. } => {
            let out_lang = match output {
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
            let allowed = AllowedScripts::for_request(
                &format!("{source}\n{}", req.context.as_deref().unwrap_or("")),
                out_lang,
            );
            (out_lang, allowed)
        }
        Task::Translate { to } => (Some(*to), AllowedScripts::for_translation(to)),
    };
    let translate_to: Option<&Language> = match &req.task {
        Task::Translate { to } => Some(to),
        Task::Explain { .. } => None,
    };
    let out = out_lang.map_or(OutputLanguage::SameAsText, OutputLanguage::Named);
    let grammar = engine.supports_grammar().then(|| allowed.grammar());

    // The reading-grade formula is calibrated for English only, and a translation has no
    // grade to compare.
    let english = translate_to.is_none()
        && kind == SelectionKind::Passage
        && language::detect(&source).is_some_and(|l| l.code == "en");
    let parts: Vec<String> = match kind {
        SelectionKind::Passage => text::chunk(&source, req.part_words),
        _ => vec![source.clone()],
    };
    emit(Event::Started {
        kind,
        source: source.clone(),
        truncated,
        grade_before: english
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
        let passage = kind == SelectionKind::Passage;
        let word = text::bare_word(part);
        let context = req.context.as_deref();
        let mut sampling = req.sampling.clone();
        let messages = match (&req.task, passage) {
            (
                Task::Explain {
                    level,
                    custom_prompt,
                    ..
                },
                true,
            ) => {
                sampling.max_tokens =
                    prompt::max_tokens_for_rewrite(text::word_count(part), *level);
                prompt::rewrite(
                    part,
                    *level,
                    out,
                    custom_prompt.as_deref(),
                    Some((i + 1, parts.len())),
                )
            }
            (Task::Explain { .. }, false) => {
                sampling.max_tokens = 160;
                prompt::word_meaning(word, context, out)
            }
            (Task::Translate { to }, true) => {
                sampling.max_tokens = prompt::max_tokens_for_translation(part, to);
                prompt::translate(part, to, Some((i + 1, parts.len())))
            }
            (Task::Translate { to }, false) => {
                sampling.max_tokens = prompt::max_tokens_for_word_translation(to);
                prompt::translate_word(word, context, to)
            }
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
            let drifted = allowed.violations(&cleaned).is_some();
            // A translation handed back in the source language is retried the same way.
            let untranslated =
                translate_to.is_some_and(|to| language::untranslated(part, &cleaned, to));
            if !drifted && !untranslated {
                final_text = Some(cleaned);
                break;
            }
            tracing::warn!(
                part = i,
                attempt,
                drifted,
                untranslated,
                "answer is not in the language asked for"
            );
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
    let common = |w: &str| req.common_words.iter().any(|c| c == w);
    let (grade_after, report) = match (kind, translate_to) {
        (SelectionKind::Passage, None) => (
            english
                .then(|| readability::grade(&text).map(|r| r.grade))
                .flatten(),
            meaning::check_with(&source, &text, &common),
        ),
        (SelectionKind::Passage, Some(to)) => (
            None,
            meaning::check_translation(&source, &text, to, &common),
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
