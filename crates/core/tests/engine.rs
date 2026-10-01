//! End-to-end tests of the engine and the explain pipeline against the fake llama-server
//! (`src/bin/te-fake-llama.rs`): process start, health wait, streaming, crash recovery,
//! the language guard and the output filter.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use te_core::engine::{Backend, Engine, EngineError, ServerConfig};
use te_core::explain::{self, Event, ExplainError, OutputChoice, Request};
use te_core::text::SelectionKind;

fn fake_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_te-fake-llama"))
}

fn config(dir: &tempfile::TempDir) -> ServerConfig {
    let model = dir.path().join("model.gguf");
    std::fs::write(&model, b"GGUF").unwrap();
    ServerConfig {
        exe: fake_exe(),
        model,
        ctx: 4096,
        threads: None,
        log: Some(dir.path().join("server.log")),
        extra_args: vec![],
    }
}

async fn collect(engine: &mut Engine, req: &Request) -> (Result<(), ExplainError>, Vec<Event>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let mut emit = move |e: Event| sink.lock().unwrap().push(e);
    let result = explain::run(engine, req, &mut emit).await;
    let events = events.lock().unwrap().clone();
    (result, events)
}

fn done_text(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|e| match e {
            Event::Done { text, .. } => Some(text.clone()),
            _ => None,
        })
        .expect("a Done event")
}

#[tokio::test]
async fn rewrites_a_passage_and_strips_the_preamble() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(Backend::Local(config(&dir)));
    let req =
        Request::new("The lessee shall remit payment of $1,200 by the 5th day of each month.");
    let (result, events) = collect(&mut engine, &req).await;
    result.unwrap();

    match &events[0] {
        Event::Started {
            kind,
            parts,
            language,
            ..
        } => {
            assert_eq!(*kind, SelectionKind::Passage);
            assert_eq!(*parts, 1);
            assert_eq!(language.as_deref(), Some("English"));
        }
        e => panic!("unexpected first event {e:?}"),
    }
    assert!(
        events.contains(&Event::Loading),
        "first request loads the model"
    );
    let streamed: String = events
        .iter()
        .filter_map(|e| match e {
            Event::Delta { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        !streamed.contains("Sure!"),
        "preamble never shown: {streamed:?}"
    );
    assert_eq!(
        done_text(&events),
        "The lessee shall remit payment of $1,200 by the 5th day of each month."
    );
    match events.last().unwrap() {
        Event::Done {
            report, timings, ..
        } => {
            assert!(report.is_clean());
            assert_eq!(timings.as_ref().unwrap().predicted_per_second, Some(20.0));
        }
        e => panic!("{e:?}"),
    }
    assert!(engine.is_loaded());

    // The second request reuses the running server: no Loading event.
    let (_, events) = collect(&mut engine, &req).await;
    assert!(!events.contains(&Event::Loading));
    engine.unload().await;
    assert!(!engine.is_loaded());
}

#[tokio::test]
async fn sends_the_language_grammar_and_never_shows_drift() {
    let dir = tempfile::tempdir().unwrap();
    let record = dir.path().join("requests.jsonl");
    let mut cfg = config(&dir);
    cfg.extra_args = vec![
        "--fake-record".into(),
        record.to_string_lossy().into_owned(),
    ];
    let mut engine = Engine::new(Backend::Local(cfg));
    let req = Request::new("[drift] The fee is due monthly and must be paid on time.");
    let (result, events) = collect(&mut engine, &req).await;
    result.unwrap();
    // With the grammar the fake model can't drift.
    assert!(!done_text(&events).contains('这'));
    let body = std::fs::read_to_string(&record).unwrap();
    let first: serde_json::Value = serde_json::from_str(body.lines().next().unwrap()).unwrap();
    let grammar = first["grammar"].as_str().expect("grammar sent");
    assert!(grammar.contains("\\u4E00-\\u9FFF"));
    assert_eq!(first["chat_template_kwargs"]["enable_thinking"], false);
    assert_eq!(first["cache_prompt"], true);
    engine.unload().await;
}

/// An endpoint without grammar support: the guard catches the drift and retries once
/// with a stricter instruction, and the UI is told to drop the drifted text.
#[tokio::test]
async fn retries_once_when_an_endpoint_drifts() {
    let dir = tempfile::tempdir().unwrap();
    // Start a fake server, then talk to it as an "endpoint" (no grammar).
    let mut local = Engine::new(Backend::Local(config(&dir)));
    let target = local.ensure_ready().await.unwrap();
    let mut engine = Engine::new(Backend::Endpoint {
        base_url: target.base_url.clone(),
        api_key: target.api_key.clone(),
        model: "fake".into(),
    });
    let req = Request::new("[drift] The fee is due monthly and must be paid on time.");
    let (result, events) = collect(&mut engine, &req).await;
    result.unwrap();
    assert!(events.contains(&Event::RestartPart { part: 0 }));
    let text = done_text(&events);
    assert!(!text.contains('这'), "{text}");
    assert!(text.contains("The fee is due monthly"));
    local.unload().await;
}

#[tokio::test]
async fn long_text_is_streamed_in_parts() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(Backend::Local(config(&dir)));
    let para = "This sentence is part of a long paragraph about the rules. ".repeat(12);
    let text = format!("{}\n\n{}\n\n{}", para.trim(), para.trim(), para.trim());
    let mut req = Request::new(text);
    req.part_words = 150; // each paragraph is 132 words: one part each
    let (result, events) = collect(&mut engine, &req).await;
    result.unwrap();
    let parts_done: Vec<usize> = events
        .iter()
        .filter_map(|e| match e {
            Event::PartDone { part, .. } => Some(*part),
            _ => None,
        })
        .collect();
    assert_eq!(parts_done, vec![0, 1, 2]);
    assert_eq!(done_text(&events).matches("This sentence").count(), 36);
    engine.unload().await;
}

#[tokio::test]
async fn single_words_ask_for_a_meaning() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(Backend::Local(config(&dir)));
    let mut req = Request::new("“ubiquitous,”");
    req.context = Some("Smartphones are ubiquitous in modern life.".into());
    let (result, events) = collect(&mut engine, &req).await;
    result.unwrap();
    assert_eq!(done_text(&events), "It means ubiquitous.");
    match &events[0] {
        Event::Started {
            kind, grade_before, ..
        } => {
            assert_eq!(*kind, SelectionKind::Word);
            assert_eq!(*grade_before, None);
        }
        e => panic!("{e:?}"),
    }
    engine.unload().await;
}

#[tokio::test]
async fn fixed_output_language_is_named_in_the_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let record = dir.path().join("requests.jsonl");
    let mut cfg = config(&dir);
    cfg.extra_args = vec![
        "--fake-record".into(),
        record.to_string_lossy().into_owned(),
    ];
    let mut engine = Engine::new(Backend::Local(cfg));
    let mut req =
        Request::new("The rent is due on the first day of every month, without exception.");
    req.output = OutputChoice::Fixed("bn".into());
    let (result, _) = collect(&mut engine, &req).await;
    result.unwrap();
    let body = std::fs::read_to_string(&record).unwrap();
    assert!(body.contains("Write only in Bangla (Bengali)."), "{body}");
    engine.unload().await;
}

#[tokio::test]
async fn empty_selection_is_an_error_without_starting_the_model() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(Backend::Local(config(&dir)));
    let (result, events) = collect(&mut engine, &Request::new("  \n ")).await;
    assert!(matches!(result, Err(ExplainError::Empty)));
    assert!(matches!(events.as_slice(), [Event::Error { .. }]));
    assert!(!engine.is_loaded());
}

#[tokio::test]
async fn no_model_is_a_clear_error() {
    let mut engine = Engine::new(Backend::None);
    let (result, _) = collect(&mut engine, &Request::new("Some text to explain here.")).await;
    assert!(matches!(
        result,
        Err(ExplainError::Engine(EngineError::NoModel))
    ));
}

#[tokio::test]
async fn missing_model_file_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = config(&dir);
    cfg.model = dir.path().join("gone.gguf");
    let mut engine = Engine::new(Backend::Local(cfg));
    let err = engine.ensure_ready().await.unwrap_err();
    assert!(matches!(err, EngineError::MissingModel(_)));
}

#[tokio::test]
async fn waits_for_the_model_to_load() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = config(&dir);
    cfg.extra_args = vec!["--fake-load-ms".into(), "600".into()];
    let mut engine = Engine::new(Backend::Local(cfg));
    let started = std::time::Instant::now();
    engine.ensure_ready().await.unwrap();
    assert!(started.elapsed() >= Duration::from_millis(500));
    engine.unload().await;
}

#[tokio::test]
async fn a_server_that_fails_to_start_reports_its_log() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = config(&dir);
    cfg.extra_args = vec!["--fake-exit".into()];
    let mut engine = Engine::new(Backend::Local(cfg));
    match engine.ensure_ready().await.unwrap_err() {
        EngineError::Exited { code, log_tail } => {
            assert_eq!(code, Some(1));
            assert!(log_tail.contains("invalid magic"), "{log_tail}");
        }
        e => panic!("{e:?}"),
    }
}

#[tokio::test]
async fn restarts_after_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = Engine::new(Backend::Local(config(&dir)));
    let first = engine.ensure_ready().await.unwrap();
    // Kill the server behind the engine's back, like a crash.
    let pid = engine.server_pid().unwrap();
    kill(pid);
    for _ in 0..50 {
        if !engine.is_loaded() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!engine.is_loaded());
    let second = engine.ensure_ready().await.unwrap();
    assert_ne!(
        first.base_url, second.base_url,
        "a new server on a new port"
    );
    engine.unload().await;
}

#[cfg(unix)]
fn kill(pid: u32) {
    std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .status()
        .unwrap();
}

#[cfg(windows)]
fn kill(pid: u32) {
    std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .status()
        .unwrap();
}
