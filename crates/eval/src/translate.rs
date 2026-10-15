//! `te-eval --task translate`: the translation corpus through each model, into each
//! target, exactly as AI Translate runs it (`explain::run` with `Task::Translate`: the same
//! prompts, grammar, output filter, retry and guards), plus a few answers per target
//! without the grammar to see how often the model would write the wrong script on its own.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use te_core::engine::{Engine, Sampling};
use te_core::explain::{self, Event, Request, Task};
use te_core::language::{self, AllowedScripts, Language};
use te_core::meaning::{Fact, Report};
use te_core::output::StreamFilter;
use te_core::text::{self, SelectionKind};
use te_core::{prompt, translate};

use crate::{
    Args, Running, common_words, median, open_dictionary, prepare_model, report, start_engine,
};

/// The targets the corpus is written for (`docs/SOURCES_PLAN.md`, S4).
pub const DEFAULT_TARGETS: &str = "bn,es,hi,ar,zh";

#[derive(Debug, Deserialize)]
pub struct Item {
    pub id: String,
    pub domain: String,
    pub text: String,
    /// The sentence around a single word.
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub item: String,
    pub domain: String,
    pub target: String,
    pub kind: SelectionKind,
    pub ttft_ms: Option<u64>,
    pub total_ms: u64,
    pub tokens_per_second: Option<f64>,
    pub predicted_tokens: Option<u64>,
    /// Tokens the answer took per English-sized source word (`prompt::source_words`).
    pub tokens_per_source_word: Option<f64>,
    /// The ceiling the app set (`prompt::max_tokens_for_*`).
    pub budget: u32,
    /// The guard caught the first answer (wrong script, or handed back untranslated) and
    /// asked again.
    pub retried: bool,
    /// The request failed (both answers in the wrong language, or an engine error).
    pub error: Option<String>,
    pub missing_numbers: Vec<String>,
    pub added_numbers: Vec<String>,
    /// Names and acronyms (checked only when source and target share a script).
    pub missing_terms: Vec<String>,
    pub output: String,
}

impl Row {
    /// The answer used (nearly) the whole token budget: probably cut short.
    pub fn hit_budget(&self) -> bool {
        self.predicted_tokens
            .is_some_and(|n| n + 2 >= u64::from(self.budget))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    pub model: String,
    pub target: String,
    pub rows: usize,
    pub ttft_ms_median: u64,
    pub total_ms_median: u64,
    pub tokens_per_second_mean: f64,
    /// Over passages (a word's answer has a fixed-size explanation).
    pub tokens_per_source_word_mean: f64,
    pub tokens_per_source_word_max: f64,
    pub hit_budget: usize,
    pub retried: usize,
    pub failed: usize,
    pub rows_with_missing_numbers: usize,
    pub rows_with_added_numbers: usize,
    pub rows_with_missing_terms: usize,
    pub wrong_script_without_grammar: usize,
    pub trials_without_grammar: usize,
}

/// "bn,es" → the guard's languages; an unknown or web-only code is an error.
pub fn parse_targets(list: &str) -> Result<Vec<&'static Language>, String> {
    list.split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(|c| translate::target(c).ok_or_else(|| format!("{c} is not an AI Translate target")))
        .collect()
}

pub fn parse_corpus(text: &str) -> Vec<Item> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("translation corpus line"))
        .collect()
}

/// Numbers (and dates) apart from names and acronyms.
pub fn split_facts(facts: &[Fact]) -> (Vec<String>, Vec<String>) {
    let (numbers, terms): (Vec<&Fact>, Vec<&Fact>) =
        facts.iter().partition(|f| matches!(f, Fact::Number(_)));
    let text = |v: Vec<&Fact>| v.into_iter().map(|f| f.text().to_string()).collect();
    (text(numbers), text(terms))
}

pub async fn run(args: &Args, corpus_text: &str) {
    let items = parse_corpus(corpus_text);
    let mut summaries = Vec::new();
    for id in &args.models {
        let (model, path) = prepare_model(args, id).await;
        let Running {
            mut engine, stop, ..
        } = start_engine(args, model, &path).await;
        let dictionary = open_dictionary(args);
        let mut rows = Vec::new();
        let mut drift: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for &to in &args.targets {
            for item in &items {
                let common = common_words(dictionary.as_ref(), &text::clean(&item.text));
                let row = run_one(&mut engine, item, to, common).await;
                eprintln!(
                    "[{}] {} -> {}: {} ms, {:.1} tok/s, {:?} tokens/word{}{}{}\n  > {}",
                    model.id,
                    item.id,
                    to.code,
                    row.total_ms,
                    row.tokens_per_second.unwrap_or(0.0),
                    row.tokens_per_source_word,
                    if row.retried { ", retried" } else { "" },
                    if row.missing_numbers.is_empty() {
                        String::new()
                    } else {
                        format!(", missing {:?}", row.missing_numbers)
                    },
                    row.error
                        .as_ref()
                        .map(|e| format!(", failed: {e}"))
                        .unwrap_or_default(),
                    row.output.replace('\n', "\n  > "),
                );
                rows.push(row);
            }
            let trials: Vec<&Item> = items.iter().take(args.drift_subset).collect();
            let mut wrong = 0;
            for item in &trials {
                if wrong_script_without_grammar(&mut engine, item, to).await {
                    wrong += 1;
                }
            }
            drift.insert(to.code.to_string(), (wrong, trials.len()));
        }
        stop.store(true, Ordering::SeqCst);
        engine.unload().await;
        std::fs::write(
            args.out.join(format!("{}-translate-rows.json", model.id)),
            serde_json::to_vec_pretty(&rows).unwrap(),
        )
        .unwrap();
        summaries.extend(summarize(model.id, &rows, &drift));
    }
    report(args, markdown(&summaries));
    std::fs::write(
        args.out.join("translate-summary.json"),
        serde_json::to_vec_pretty(&summaries).unwrap(),
    )
    .unwrap();
}

/// One item into one target, through the app's own pipeline.
async fn run_one(
    engine: &mut Engine,
    item: &Item,
    to: &'static Language,
    common_words: Vec<String>,
) -> Row {
    let mut req = Request::new(item.text.clone());
    req.context = item.context.clone();
    req.task = Task::Translate { to };
    req.common_words = common_words;
    req.sampling = Sampling {
        temperature: 0.3,
        top_p: 0.9,
        max_tokens: 512,
    };
    let source = text::clean(&item.text);
    let kind = text::classify(&source);
    let budget = match kind {
        SelectionKind::Passage => prompt::max_tokens_for_translation(&source, to),
        _ => prompt::max_tokens_for_word_translation(to),
    };

    let events = Arc::new(Mutex::new(Vec::<(Duration, Event)>::new()));
    let sink = events.clone();
    let start = Instant::now();
    let mut emit = move |e: Event| sink.lock().unwrap().push((start.elapsed(), e));
    let result = explain::run(engine, &req, &mut emit).await;
    let total_ms = start.elapsed().as_millis() as u64;
    let events = events.lock().unwrap().clone();

    let ttft_ms = events
        .iter()
        .find(|(_, e)| matches!(e, Event::Delta { .. } | Event::PartDone { .. }))
        .map(|(t, _)| t.as_millis() as u64);
    let retried = events
        .iter()
        .any(|(_, e)| matches!(e, Event::RestartPart { .. }));
    let (output, report, timings) = events
        .iter()
        .find_map(|(_, e)| match e {
            Event::Done {
                text,
                report,
                timings,
                ..
            } => Some((text.clone(), report.clone(), timings.clone())),
            _ => None,
        })
        .unwrap_or_else(|| (String::new(), Report::default(), None));
    let (missing_numbers, mut missing_terms) = split_facts(&report.missing);
    let (added_numbers, added_terms) = split_facts(&report.added);
    missing_terms.extend(added_terms);
    let timings = timings.unwrap_or_default();
    let words = prompt::source_words(&source).max(1) as f64;
    Row {
        item: item.id.clone(),
        domain: item.domain.clone(),
        target: to.code.to_string(),
        kind,
        ttft_ms,
        total_ms,
        tokens_per_second: timings.predicted_per_second,
        predicted_tokens: timings.predicted_n,
        tokens_per_source_word: timings.predicted_n.map(|n| n as f64 / words),
        budget,
        retried,
        error: result.err().map(|e| e.to_string()),
        missing_numbers,
        added_numbers,
        missing_terms,
        output,
    }
}

/// Asks once without the grammar: does the answer use a script the guard would block, or
/// hand the text back untranslated?
async fn wrong_script_without_grammar(
    engine: &mut Engine,
    item: &Item,
    to: &'static Language,
) -> bool {
    let source = text::clean(&item.text);
    let passage = text::classify(&source) == SelectionKind::Passage;
    let (messages, max_tokens) = if passage {
        (
            prompt::translate(&source, to, None),
            prompt::max_tokens_for_translation(&source, to),
        )
    } else {
        (
            prompt::translate_word(text::bare_word(&source), item.context.as_deref(), to),
            prompt::max_tokens_for_word_translation(to),
        )
    };
    let sampling = Sampling {
        temperature: 0.3,
        top_p: 0.9,
        max_tokens,
    };
    let mut filter = StreamFilter::new();
    let Ok(_) = engine
        .chat(&messages, &sampling, None, |d| {
            filter.push(d);
        })
        .await
    else {
        return false;
    };
    let output = filter.finish();
    AllowedScripts::for_translation(to)
        .violations(&output)
        .is_some()
        || language::untranslated(&source, &output, to)
}

/// One summary per target, in the order the targets were run.
pub fn summarize(
    model: &str,
    rows: &[Row],
    drift: &BTreeMap<String, (usize, usize)>,
) -> Vec<Summary> {
    let mut targets: Vec<&str> = Vec::new();
    for r in rows {
        if !targets.contains(&r.target.as_str()) {
            targets.push(&r.target);
        }
    }
    targets
        .into_iter()
        .map(|target| {
            let rows: Vec<&Row> = rows.iter().filter(|r| r.target == target).collect();
            let tps: Vec<f64> = rows.iter().filter_map(|r| r.tokens_per_second).collect();
            let per_word: Vec<f64> = rows
                .iter()
                .filter(|r| r.kind == SelectionKind::Passage)
                .filter_map(|r| r.tokens_per_source_word)
                .collect();
            let (wrong, trials) = drift.get(target).copied().unwrap_or_default();
            Summary {
                model: model.to_string(),
                target: target.to_string(),
                rows: rows.len(),
                ttft_ms_median: median(rows.iter().filter_map(|r| r.ttft_ms).collect()),
                total_ms_median: median(rows.iter().map(|r| r.total_ms).collect()),
                tokens_per_second_mean: mean(&tps),
                tokens_per_source_word_mean: mean(&per_word),
                tokens_per_source_word_max: per_word.iter().copied().fold(0.0, f64::max),
                hit_budget: rows.iter().filter(|r| r.hit_budget()).count(),
                retried: rows.iter().filter(|r| r.retried).count(),
                failed: rows.iter().filter(|r| r.error.is_some()).count(),
                rows_with_missing_numbers: rows
                    .iter()
                    .filter(|r| !r.missing_numbers.is_empty())
                    .count(),
                rows_with_added_numbers: rows
                    .iter()
                    .filter(|r| !r.added_numbers.is_empty())
                    .count(),
                rows_with_missing_terms: rows
                    .iter()
                    .filter(|r| !r.missing_terms.is_empty())
                    .count(),
                wrong_script_without_grammar: wrong,
                trials_without_grammar: trials,
            }
        })
        .collect()
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

pub fn markdown(summaries: &[Summary]) -> String {
    let mut s = String::from(
        "\n## Translation evaluation\n\n| Model | Into | First token (median) | Whole answer (median) | Speed | Tokens per source word (mean / max) | Cut at the budget | Numbers missing | Numbers added | Names missing | Retried by the guard | Failed | Wrong language without grammar |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for m in summaries {
        let n = m.rows;
        s.push_str(&format!(
            "| {} | {} | {:.1} s | {:.1} s | {:.1} tok/s | {:.1} / {:.1} | {}/{n} | {}/{n} | {}/{n} | {}/{n} | {}/{n} | {}/{n} | {}/{} |\n",
            m.model,
            m.target,
            m.ttft_ms_median as f64 / 1000.0,
            m.total_ms_median as f64 / 1000.0,
            m.tokens_per_second_mean,
            m.tokens_per_source_word_mean,
            m.tokens_per_source_word_max,
            m.hit_budget,
            m.rows_with_missing_numbers,
            m.rows_with_added_numbers,
            m.rows_with_missing_terms,
            m.retried,
            m.failed,
            m.wrong_script_without_grammar,
            m.trials_without_grammar,
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(target: &str, kind: SelectionKind) -> Row {
        Row {
            item: "x".into(),
            domain: "d".into(),
            target: target.into(),
            kind,
            ttft_ms: Some(1000),
            total_ms: 3000,
            tokens_per_second: Some(10.0),
            predicted_tokens: Some(60),
            tokens_per_source_word: Some(4.0),
            budget: 200,
            retried: false,
            error: None,
            missing_numbers: vec![],
            added_numbers: vec![],
            missing_terms: vec![],
            output: String::new(),
        }
    }

    #[test]
    fn targets_must_be_guarded_languages() {
        let codes: Vec<&str> = parse_targets(DEFAULT_TARGETS)
            .unwrap()
            .iter()
            .map(|l| l.code)
            .collect();
        assert_eq!(codes, vec!["bn", "es", "hi", "ar", "zh"]);
        assert_eq!(parse_targets(" zh-CN , es,").unwrap().len(), 2);
        assert!(parse_targets("bn,th").unwrap_err().contains("th"));
    }

    #[test]
    fn the_corpus_parses_and_covers_numbers_names_and_words() {
        let items = parse_corpus(include_str!("../../../eval/translate.jsonl"));
        assert!(items.len() >= 12, "{}", items.len());
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        for (i, id) in ids.iter().enumerate() {
            assert!(!ids[i + 1..].contains(id), "duplicate {id}");
        }
        let words: Vec<&Item> = items
            .iter()
            .filter(|i| text::classify(&i.text) != SelectionKind::Passage)
            .collect();
        assert!(words.len() >= 3);
        assert!(
            words.iter().all(|w| w.context.is_some()),
            "words come with their sentence"
        );
        let passages = items
            .iter()
            .filter(|i| text::classify(&i.text) == SelectionKind::Passage);
        assert!(
            passages
                .clone()
                .all(|i| i.text.chars().any(|c| c.is_ascii_digit())),
            "every passage has numbers"
        );
        // Every passage is English (the corpus is English into the targets).
        assert!(
            passages
                .clone()
                .all(|i| language::detect(&i.text).map(|l| l.code) == Some("en"))
        );
        // Dates in two notations.
        assert!(items.iter().any(|i| i.text.contains("31/12/2025")));
        assert!(items.iter().any(|i| i.text.contains("1 March 2026")));
    }

    #[test]
    fn facts_are_split_into_numbers_and_names() {
        let (numbers, terms) = split_facts(&[
            Fact::Number("5%".into()),
            Fact::Name("Geneva".into()),
            Fact::Acronym("WHO".into()),
        ]);
        assert_eq!(numbers, vec!["5%"]);
        assert_eq!(terms, vec!["Geneva", "WHO"]);
    }

    #[test]
    fn summaries_count_per_target() {
        let mut a = row("bn", SelectionKind::Passage);
        a.missing_numbers = vec!["5%".into()];
        a.retried = true;
        let mut b = row("bn", SelectionKind::Word);
        b.tokens_per_source_word = Some(50.0);
        b.predicted_tokens = Some(199);
        let mut c = row("es", SelectionKind::Passage);
        c.error = Some("drift".into());
        c.tokens_per_source_word = Some(2.0);
        let drift = BTreeMap::from([("bn".to_string(), (1, 4))]);
        let s = summarize("m", &[a, b, c], &drift);
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].target.as_str(), s[0].rows), ("bn", 2));
        assert_eq!(s[0].rows_with_missing_numbers, 1);
        assert_eq!(s[0].retried, 1);
        assert_eq!(s[0].hit_budget, 1);
        // Words don't count towards tokens per source word.
        assert_eq!(s[0].tokens_per_source_word_mean, 4.0);
        assert_eq!(
            (
                s[0].wrong_script_without_grammar,
                s[0].trials_without_grammar
            ),
            (1, 4)
        );
        assert_eq!(s[1].failed, 1);
        assert_eq!(s[1].trials_without_grammar, 0);
        let md = markdown(&s);
        assert!(md.contains("| m | bn | 1.0 s | 3.0 s | 10.0 tok/s | 4.0 / 4.0 | 1/2 | 1/2 | 0/2 | 0/2 | 1/2 | 0/2 | 1/4 |"), "{md}");
    }
}
