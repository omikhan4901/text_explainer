//! A stand-in for llama.cpp's `llama-server`, used by tests and by CI (which has no
//! model). It speaks just enough of the real API: `/health` and a streaming
//! `/v1/chat/completions` that echoes the text it was asked to rewrite.
//!
//! Behaviour switches (extra arguments, which tests pass through `ServerConfig::extra_args`):
//! - `--fake-load-ms N`: `/health` answers 503 (loading) for this long.
//! - `--fake-exit`: exit at once with an error, like a bad model file.
//! - `--fake-record PATH`: append each request body (one JSON per line) to this file.
//! - Text containing `[drift]` is answered in Chinese unless the request carries a
//!   grammar or a language reminder, like a small model drifting.
//! - Text containing `[slow]` streams slowly.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if args.iter().any(|a| a == "--fake-exit") {
        eprintln!("llama_model_load: error loading model: invalid magic");
        std::process::exit(1);
    }
    let port: u16 = arg("--port").and_then(|p| p.parse().ok()).unwrap_or(8080);
    let key = arg("--api-key");
    let load = Duration::from_millis(
        arg("--fake-load-ms")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
    );
    let record = arg("--fake-record");
    let started = Instant::now();
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind");
    println!("fake llama-server listening on 127.0.0.1:{port}");
    for stream in listener.incoming().flatten() {
        let key = key.clone();
        let record = record.clone();
        std::thread::spawn(move || {
            handle(
                stream,
                key.as_deref(),
                record.as_deref(),
                started.elapsed() >= load,
            )
        });
    }
}

fn handle(mut stream: TcpStream, key: Option<&str>, record: Option<&str>, loaded: bool) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let mut content_length = 0usize;
    let mut auth = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let lower = line.to_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
        if lower.starts_with("authorization:") {
            auth = Some(line["authorization:".len()..].trim().to_string());
        }
    }
    let mut body = vec![0u8; content_length];
    if reader.read_exact(&mut body).is_err() {
        return;
    }
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");

    if let Some(k) = key
        && auth.as_deref() != Some(&format!("Bearer {k}"))
    {
        respond(
            &mut stream,
            401,
            r#"{"error":{"message":"Invalid API Key"}}"#,
        );
        return;
    }
    match path {
        "/health" if loaded => respond(&mut stream, 200, r#"{"status":"ok"}"#),
        "/health" => respond(&mut stream, 503, r#"{"error":{"message":"Loading model"}}"#),
        "/v1/chat/completions" if loaded => chat(&mut stream, &body, record),
        _ => respond(&mut stream, 404, r#"{"error":{"message":"Not found"}}"#),
    }
}

fn respond(stream: &mut TcpStream, status: u16, body: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

fn chat(stream: &mut TcpStream, body: &[u8], record: Option<&str>) {
    if let Some(path) = record
        && let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = f.write_all(body);
        let _ = f.write_all(b"\n");
    }
    let req: serde_json::Value = serde_json::from_slice(body).unwrap_or_default();
    let messages = req["messages"].as_array().cloned().unwrap_or_default();
    let last_user = messages
        .iter()
        .rev()
        .find(|m| m["role"] == "user")
        .and_then(|m| m["content"].as_str())
        .unwrap_or("")
        .to_string();
    let reminded = messages.iter().any(|m| {
        m["content"]
            .as_str()
            .is_some_and(|c| c.starts_with("Important: write the whole answer"))
    });
    let constrained = req.get("grammar").is_some();

    let text = match (last_user.find("<text>\n"), last_user.rfind("\n</text>")) {
        (Some(a), Some(b)) if b > a => last_user[a + 7..b].to_string(),
        _ => last_user
            .strip_prefix("Word: ")
            .map(|w| format!("It means {}.", w.lines().next().unwrap_or(w)))
            .unwrap_or_else(|| last_user.clone()),
    };
    let slow = text.contains("[slow]");
    let clean = text.replace("[drift]", "").replace("[slow]", "");
    let reply = if text.contains("[drift]") && !constrained && !reminded {
        format!("这是一个测试。{}", clean.trim())
    } else {
        format!("Sure! Here is a simpler version:\n\n{}", clean.trim())
    };

    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
    );
    let chars: Vec<char> = reply.chars().collect();
    for piece in chars.chunks(7) {
        let piece: String = piece.iter().collect();
        let event = serde_json::json!({"choices":[{"index":0,"delta":{"content":piece},"finish_reason":null}]});
        if write!(stream, "data: {event}\n\n").is_err() {
            return;
        }
        let _ = stream.flush();
        if slow {
            std::thread::sleep(Duration::from_millis(40));
        }
    }
    let n = chars.len() as u64 / 4 + 1;
    let done = serde_json::json!({
        "choices":[{"index":0,"delta":{},"finish_reason":"stop"}],
        "timings":{"prompt_n":42,"prompt_ms":120.0,"predicted_n":n,"predicted_ms":n as f64 * 50.0,"predicted_per_second":20.0}
    });
    let _ = write!(stream, "data: {done}\n\ndata: [DONE]\n\n");
    let _ = stream.flush();
}
