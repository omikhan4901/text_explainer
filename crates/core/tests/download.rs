//! The downloader against a small local HTTP server that supports ranges and can drop
//! the connection halfway, like a flaky network.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sha2::{Digest, Sha256};
use te_core::download::{Cancel, Download, DownloadError, download, part_path, sha256_file};

struct Server {
    url: String,
    requests: Arc<AtomicUsize>,
}

/// Serves `data`. The first `drop_first` responses stop after half the body.
fn serve(data: Vec<u8>, drop_first: usize, honour_range: bool) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/model.gguf", listener.local_addr().unwrap());
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let n = count.fetch_add(1, Ordering::SeqCst);
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut start = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
                    break;
                }
                if let Some(r) = line.to_lowercase().strip_prefix("range: bytes=") {
                    start = r.trim().trim_end_matches('-').parse().unwrap_or(0);
                }
            }
            let mut stream = stream;
            let (status, body) = if honour_range && start > 0 {
                ("206 Partial Content", &data[start..])
            } else {
                ("200 OK", &data[..])
            };
            let send = if n < drop_first {
                &body[..body.len() / 2]
            } else {
                body
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(send);
        }
    });
    Server { url, requests }
}

fn data(n: usize) -> (Vec<u8>, String) {
    let d: Vec<u8> = (0..n).map(|i| (i * 7 % 251) as u8).collect();
    let sha = Sha256::digest(&d)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    (d, sha)
}

#[tokio::test]
async fn downloads_and_verifies() {
    let (d, sha) = data(300_000);
    let server = serve(d.clone(), 0, true);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("models").join("m.gguf");
    let mut last = None;
    download(
        Download {
            url: &server.url,
            dest: &dest,
            sha256: Some(&sha),
            size: Some(d.len() as u64),
        },
        &Cancel::new(),
        |p| last = Some(p),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), d);
    assert!(!part_path(&dest).exists());
    assert_eq!(last.unwrap().downloaded, d.len() as u64);
    assert_eq!(sha256_file(&dest).unwrap(), sha);
}

#[tokio::test]
async fn resumes_after_a_dropped_connection() {
    let (d, sha) = data(400_000);
    let server = serve(d.clone(), 1, true);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("m.gguf");
    let job = || Download {
        url: &server.url,
        dest: &dest,
        sha256: Some(&sha),
        size: Some(d.len() as u64),
    };
    // The first attempt gets half the file and the connection closes early.
    let first = download(job(), &Cancel::new(), |_| {}).await;
    assert!(
        first.is_err(),
        "short body must not count as complete: {first:?}"
    );
    let partial = std::fs::metadata(part_path(&dest)).unwrap().len();
    assert!(partial > 0 && partial < d.len() as u64);
    // The second attempt asks only for the rest.
    download(job(), &Cancel::new(), |_| {}).await.unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), d);
    assert_eq!(server.requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn restarts_when_the_server_ignores_ranges() {
    let (d, sha) = data(200_000);
    let server = serve(d.clone(), 0, false);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("m.gguf");
    std::fs::write(part_path(&dest), &d[..1000]).unwrap();
    download(
        Download {
            url: &server.url,
            dest: &dest,
            sha256: Some(&sha),
            size: Some(d.len() as u64),
        },
        &Cancel::new(),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), d);
}

#[tokio::test]
async fn a_wrong_checksum_deletes_the_file() {
    let (d, _) = data(100_000);
    let server = serve(d.clone(), 0, true);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("m.gguf");
    let err = download(
        Download {
            url: &server.url,
            dest: &dest,
            sha256: Some(&"0".repeat(64)),
            size: Some(d.len() as u64),
        },
        &Cancel::new(),
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(matches!(err, DownloadError::Checksum));
    assert!(!dest.exists());
    assert!(!part_path(&dest).exists());
}

#[tokio::test]
async fn cancelling_keeps_the_partial_file_for_later() {
    let (d, sha) = data(2_000_000);
    let server = serve(d.clone(), 0, true);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("m.gguf");
    let cancel = Cancel::new();
    cancel.cancel();
    let err = download(
        Download {
            url: &server.url,
            dest: &dest,
            sha256: Some(&sha),
            size: Some(d.len() as u64),
        },
        &cancel,
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(matches!(err, DownloadError::Cancelled));
    assert!(!dest.exists());
}

#[tokio::test]
async fn http_errors_are_reported() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/missing", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for mut s in listener.incoming().flatten() {
            let mut r = BufReader::new(s.try_clone().unwrap());
            let mut l = String::new();
            while r.read_line(&mut l).unwrap_or(0) > 2 {
                l.clear();
            }
            let _ = write!(s, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let err = download(
        Download {
            url: &url,
            dest: &dir.path().join("x"),
            sha256: None,
            size: None,
        },
        &Cancel::new(),
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(matches!(err, DownloadError::Http(404)));
}
