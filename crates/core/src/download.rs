//! Downloading model files: resumable (a dropped connection or a closed laptop doesn't
//! start a 3 GB download over), verified against the expected size and SHA-256, and
//! cancellable. The file only gets its final name once it has been verified.

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
    /// Average over the last few seconds.
    pub bytes_per_sec: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("download cancelled")]
    Cancelled,
    #[error("couldn't download ({0})")]
    Network(String),
    #[error("the server answered {0}")]
    Http(u16),
    #[error(
        "the downloaded file is damaged (checksum mismatch); it has been deleted, please try again"
    )]
    Checksum,
    #[error("the downloaded file has the wrong size ({got} bytes, expected {expected})")]
    Size { got: u64, expected: u64 },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct Download<'a> {
    pub url: &'a str,
    pub dest: &'a Path,
    pub sha256: Option<&'a str>,
    pub size: Option<u64>,
}

/// A handle to cancel a running download.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// The partial file next to the destination.
pub fn part_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    dest.with_file_name(name)
}

pub async fn download(
    job: Download<'_>,
    cancel: &Cancel,
    mut on_progress: impl FnMut(Progress),
) -> Result<(), DownloadError> {
    crate::tls_init();
    if let Some(dir) = job.dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let part = part_path(job.dest);

    // Resume: hash what we already have, then ask for the rest.
    let mut hasher = Sha256::new();
    let mut have = 0u64;
    if part.exists() {
        let mut f = std::fs::File::open(&part)?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            have += n as u64;
        }
        if job.size.is_some_and(|s| have > s) {
            std::fs::remove_file(&part)?;
            hasher = Sha256::new();
            have = 0;
        }
    }

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .user_agent(concat!("TextExplainer/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| DownloadError::Network(e.to_string()))?;

    let complete = job.size.is_some_and(|s| have == s);
    if !complete {
        let mut req = client.get(job.url);
        if have > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={have}-"));
        }
        let resp = req
            .send()
            .await
            .map_err(|e| DownloadError::Network(e.without_url().to_string()))?;
        let status = resp.status().as_u16();
        let mut file = match status {
            206 => std::fs::OpenOptions::new().append(true).open(&part)?,
            200 => {
                // The server ignored the range: start over.
                hasher = Sha256::new();
                have = 0;
                std::fs::File::create(&part)?
            }
            416 if job.size == Some(have) => {
                std::fs::OpenOptions::new().append(true).open(&part)?
            }
            s => return Err(DownloadError::Http(s)),
        };
        let total = job
            .size
            .or_else(|| resp.content_length().map(|l| l + have))
            .unwrap_or(0);

        let mut stream = resp.bytes_stream();
        let mut window_start = Instant::now();
        let mut window_bytes = 0u64;
        let mut rate = 0.0;
        let mut last_report = Instant::now() - Duration::from_secs(1);
        while let Some(chunk) = stream.next().await {
            if cancel.is_cancelled() {
                file.flush()?;
                return Err(DownloadError::Cancelled);
            }
            let chunk = chunk.map_err(|e| DownloadError::Network(e.without_url().to_string()))?;
            file.write_all(&chunk)?;
            hasher.update(&chunk);
            have += chunk.len() as u64;
            window_bytes += chunk.len() as u64;
            let elapsed = window_start.elapsed();
            if elapsed >= Duration::from_secs(2) {
                rate = window_bytes as f64 / elapsed.as_secs_f64();
                window_start = Instant::now();
                window_bytes = 0;
            }
            if last_report.elapsed() >= Duration::from_millis(100) {
                last_report = Instant::now();
                on_progress(Progress {
                    downloaded: have,
                    total,
                    bytes_per_sec: rate,
                });
            }
        }
        file.flush()?;
        file.sync_all()?;
        on_progress(Progress {
            downloaded: have,
            total,
            bytes_per_sec: rate,
        });
    }

    if let Some(expected) = job.size
        && have != expected
    {
        if have > expected {
            let _ = std::fs::remove_file(&part);
        }
        return Err(DownloadError::Size {
            got: have,
            expected,
        });
    }
    if let Some(expected) = job.sha256 {
        let got = hex(&hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            let _ = std::fs::remove_file(&part);
            return Err(DownloadError::Checksum);
        }
    }
    std::fs::rename(&part, job.dest)?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of a file (for imported models and diagnostics).
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}
