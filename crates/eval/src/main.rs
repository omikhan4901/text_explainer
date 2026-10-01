//! `te-eval`: runs the evaluation corpus through models exactly as the app would (same
//! text repair, prompts, grammar, output filter and checks) and measures what matters
//! for choosing the default model:
//!
//! - load time, time to first token, generation speed and peak memory;
//! - reading-grade drop per level;
//! - meaning faithfulness (facts dropped or invented, from the meaning guard);
//! - chatter (preambles the output filter had to remove);
//! - language drift with the grammar turned off (how much the guard is needed).
//!
//! Usage:
//!   te-eval --server PATH --models-dir DIR --model ID[,ID] --corpus FILE --out DIR
//!           [--levels plain,simpler] [--drift-subset N] [--download]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use te_core::download::{Cancel, Download, download};
use te_core::engine::{Backend, Engine, Sampling, ServerConfig};
use te_core::language::{self, AllowedScripts};
use te_core::output::StreamFilter;
use te_core::prompt::{self, Level, OutputLanguage};
use te_core::{meaning, models, readability, text};

#[derive(Deserialize)]
struct Passage {
    id: String,
    domain: String,
    lang: String,
    text: String,
}

#[derive(Serialize)]
struct Row {
    passage: String,
    domain: String,
    lang: String,
    level: Level,
    ttft_ms: Option<u64>,
    total_ms: u64,
    tokens_per_second: Option<f64>,
    predicted_tokens: Option<u64>,
    grade_before: Option<f32>,
    grade_after: Option<f32>,
    missing: Vec<String>,
    added: Vec<String>,
    chatter: bool,
    drift: bool,
    output: String,
}

#[derive(Serialize)]
struct Summary {
    model: String,
    file_bytes: u64,
    load_ms: u64,
    peak_rss_bytes: u64,
    first_request_ms: u64,
    ttft_ms_median: u64,
    total_ms_median: u64,
    tokens_per_second_mean: f64,
    grade_drop_mean: BTreeMap<String, f64>,
    rows: usize,
    rows_with_missing_facts: usize,
    rows_with_added_facts: usize,
    rows_with_chatter: usize,
    drift_with_grammar: usize,
    drift_without_grammar: usize,
    drift_trials: usize,
}

struct Args {
    server: PathBuf,
    models_dir: PathBuf,
    models: Vec<String>,
    corpus: PathBuf,
    out: PathBuf,
    levels: Vec<Level>,
    drift_subset: usize,
    download: bool,
}

fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let get = |name: &str| -> Option<String> {
        argv.iter()
            .position(|a| a == name)
            .and_then(|i| argv.get(i + 1))
            .cloned()
    };
    let need = |name: &str| get(name).unwrap_or_else(|| panic!("missing {name}"));
    let levels = get("--levels")
        .unwrap_or_else(|| "plain,simpler".into())
        .split(',')
        .map(|l| match l.trim() {
            "simpler" => Level::Simpler,
            "clearer" => Level::Clearer,
            _ => Level::Plain,
        })
        .collect();
    Args {
        server: need("--server").into(),
        models_dir: need("--models-dir").into(),
        models: need("--model")
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),
        corpus: need("--corpus").into(),
        out: need("--out").into(),
        levels,
        drift_subset: get("--drift-subset")
            .and_then(|n| n.parse().ok())
            .unwrap_or(12),
        download: argv.iter().any(|a| a == "--download"),
    }
}

/// Peak resident memory of a process (Linux `/proc`), sampled every 200 ms.
fn watch_rss(pid: u32, stop: Arc<AtomicBool>, peak: Arc<AtomicU64>) {
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            if let Ok(status) = std::fs::read_to_string(format!("/proc/{pid}/status"))
                && let Some(kb) = status
                    .lines()
                    .find(|l| l.starts_with("VmRSS:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u64>().ok())
            {
                peak.fetch_max(kb * 1024, Ordering::SeqCst);
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
}

fn median(mut v: Vec<u64>) -> u64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[v.len() / 2]
}

#[tokio::main]
async fn main() {
    let args = parse_args();
    std::fs::create_dir_all(&args.out).expect("output folder");
    let corpus: Vec<Passage> = std::fs::read_to_string(&args.corpus)
        .expect("corpus")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("corpus line"))
        .collect();
    let mut summaries = Vec::new();
    for id in &args.models {
        let model = models::by_id(id).unwrap_or_else(|| panic!("unknown model {id}"));
        let path = args.models_dir.join(model.file_name);
        if args.download && !path.exists() {
            eprintln!("downloading {} ({} bytes)", model.name, model.size_bytes);
            download(
                Download {
                    url: model.url,
                    dest: &path,
                    sha256: Some(model.sha256),
                    size: Some(model.size_bytes),
                },
                &Cancel::new(),
                |_| {},
            )
            .await
            .expect("model download");
        }
        let summary = evaluate(&args, model, &path, &corpus).await;
        summaries.push(summary);
    }
    let md = markdown(&summaries);
    println!("{md}");
    if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
        let _ = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, md.as_bytes()));
    }
    std::fs::write(
        args.out.join("summary.json"),
        serde_json::to_vec_pretty(&summaries).unwrap(),
    )
    .unwrap();
}

async fn evaluate(
    args: &Args,
    model: &models::CatalogModel,
    path: &Path,
    corpus: &[Passage],
) -> Summary {
    let mut engine = Engine::new(Backend::Local(ServerConfig {
        exe: args.server.clone(),
        model: path.to_path_buf(),
        ctx: 4096,
        threads: None,
        log: Some(args.out.join(format!("{}-server.log", model.id))),
        extra_args: vec![],
    }));
    engine.set_ready_timeout(Duration::from_secs(600));
    let started = Instant::now();
    if let Err(e) = engine.ensure_ready().await {
        panic!("{} failed to start: {e}", model.id);
    }
    let load_ms = started.elapsed().as_millis() as u64;
    let stop = Arc::new(AtomicBool::new(false));
    let peak = Arc::new(AtomicU64::new(0));
    if let Some(pid) = engine.server_pid() {
        watch_rss(pid, stop.clone(), peak.clone());
    }

    let mut rows = Vec::new();
    let mut first_request_ms = None;
    for passage in corpus {
        for &level in &args.levels {
            let row = run_one(&mut engine, passage, level, true).await;
            first_request_ms.get_or_insert(row.total_ms);
            eprintln!(
                "[{}] {} {:?}: {} ms, ttft {:?} ms, {:.1} tok/s, grade {:?} -> {:?}{}{}",
                model.id,
                passage.id,
                level,
                row.total_ms,
                row.ttft_ms,
                row.tokens_per_second.unwrap_or(0.0),
                row.grade_before,
                row.grade_after,
                if row.missing.is_empty() {
                    String::new()
                } else {
                    format!(", missing {:?}", row.missing)
                },
                if row.chatter { ", chatter" } else { "" },
            );
            if level == Level::Plain {
                eprintln!("  > {}", row.output.replace('\n', "\n  > "));
            }
            rows.push(row);
        }
    }

    // How often does the model drift without the grammar? (Tells how much the guard is needed.)
    let mut drift_without = 0;
    let trials: Vec<&Passage> = corpus.iter().take(args.drift_subset).collect();
    for passage in &trials {
        let row = run_one(&mut engine, passage, Level::Plain, false).await;
        if row.drift {
            drift_without += 1;
            eprintln!(
                "[{}] drift without grammar on {}: {}",
                model.id, passage.id, row.output
            );
        }
    }
    stop.store(true, Ordering::SeqCst);
    engine.unload().await;

    let mut grade_drop: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for r in &rows {
        if let (Some(b), Some(a)) = (r.grade_before, r.grade_after) {
            grade_drop
                .entry(format!("{:?}", r.level).to_lowercase())
                .or_default()
                .push(f64::from(b - a));
        }
    }
    let tps: Vec<f64> = rows.iter().filter_map(|r| r.tokens_per_second).collect();
    let summary = Summary {
        model: model.id.to_string(),
        file_bytes: model.size_bytes,
        load_ms,
        peak_rss_bytes: peak.load(Ordering::SeqCst),
        first_request_ms: first_request_ms.unwrap_or(0),
        ttft_ms_median: median(rows.iter().filter_map(|r| r.ttft_ms).collect()),
        total_ms_median: median(rows.iter().map(|r| r.total_ms).collect()),
        tokens_per_second_mean: if tps.is_empty() {
            0.0
        } else {
            tps.iter().sum::<f64>() / tps.len() as f64
        },
        grade_drop_mean: grade_drop
            .into_iter()
            .map(|(k, v)| (k, v.iter().sum::<f64>() / v.len() as f64))
            .collect(),
        rows: rows.len(),
        rows_with_missing_facts: rows.iter().filter(|r| !r.missing.is_empty()).count(),
        rows_with_added_facts: rows.iter().filter(|r| !r.added.is_empty()).count(),
        rows_with_chatter: rows.iter().filter(|r| r.chatter).count(),
        drift_with_grammar: rows.iter().filter(|r| r.drift).count(),
        drift_without_grammar: drift_without,
        drift_trials: trials.len(),
    };
    std::fs::write(
        args.out.join(format!("{}-rows.json", model.id)),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
    summary
}

async fn run_one(engine: &mut Engine, passage: &Passage, level: Level, grammar: bool) -> Row {
    let source = text::clean(&passage.text);
    let lang = language::language(&passage.lang).or_else(|| language::detect(&source));
    let out = lang.map_or(OutputLanguage::SameAsText, OutputLanguage::Named);
    let allowed = AllowedScripts::for_request(&source, lang);
    let messages = prompt::rewrite(&source, level, out, None, None);
    let sampling = Sampling {
        temperature: 0.3,
        top_p: 0.9,
        max_tokens: prompt::max_tokens_for_rewrite(text::word_count(&source), level),
    };
    let grammar_text = grammar.then(|| allowed.grammar());
    let start = Instant::now();
    let mut first: Option<Duration> = None;
    let mut filter = StreamFilter::new();
    let completion = engine
        .chat(&messages, &sampling, grammar_text.as_deref(), |d| {
            first.get_or_insert_with(|| start.elapsed());
            filter.push(d);
        })
        .await
        .expect("chat request");
    let total_ms = start.elapsed().as_millis() as u64;
    let output = filter.finish();
    let report = meaning::check(&source, &output);
    let english = passage.lang == "en";
    let timings = completion.timings.unwrap_or_default();
    Row {
        passage: passage.id.clone(),
        domain: passage.domain.clone(),
        lang: passage.lang.clone(),
        level,
        ttft_ms: first.map(|d| d.as_millis() as u64),
        total_ms,
        tokens_per_second: timings.predicted_per_second,
        predicted_tokens: timings.predicted_n,
        // The grade formula is for English only.
        grade_before: english
            .then(|| readability::grade(&source).map(|r| r.grade))
            .flatten(),
        grade_after: english
            .then(|| readability::grade(&output).map(|r| r.grade))
            .flatten(),
        missing: report
            .missing
            .iter()
            .map(|f| f.text().to_string())
            .collect(),
        added: report.added.iter().map(|f| f.text().to_string()).collect(),
        chatter: completion.text.trim() != output.trim(),
        drift: allowed.violations(&output).is_some(),
        output,
    }
}

fn markdown(summaries: &[Summary]) -> String {
    let gb = |b: u64| b as f64 / 1024f64.powi(3);
    let mut s = String::from(
        "\n## Model evaluation\n\n| Model | File | Peak RAM | Load | First token (median) | Whole answer (median) | Speed | Grade drop (plain / simpler) | Facts missing | Facts added | Chatter | Drift without grammar | Drift with grammar |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for m in summaries {
        let drop = |k: &str| {
            m.grade_drop_mean
                .get(k)
                .map_or("–".into(), |v| format!("{v:.1}"))
        };
        s.push_str(&format!(
            "| {} | {:.2} GB | {:.2} GB | {:.1} s | {:.1} s | {:.1} s | {:.1} tok/s | {} / {} | {}/{} | {}/{} | {}/{} | {}/{} | {}/{} |\n",
            m.model,
            gb(m.file_bytes),
            gb(m.peak_rss_bytes),
            m.load_ms as f64 / 1000.0,
            m.ttft_ms_median as f64 / 1000.0,
            m.total_ms_median as f64 / 1000.0,
            m.tokens_per_second_mean,
            drop("plain"),
            drop("simpler"),
            m.rows_with_missing_facts,
            m.rows,
            m.rows_with_added_facts,
            m.rows,
            m.rows_with_chatter,
            m.rows,
            m.drift_without_grammar,
            m.drift_trials,
            m.drift_with_grammar,
            m.rows,
        ));
    }
    s
}
