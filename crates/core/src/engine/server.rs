//! Runs llama.cpp's `llama-server` as a child process: localhost only, a random port, a
//! per-launch API key, no console window, logs to a file, and killed with the app.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use super::EngineError;
use super::client::{Client, Target};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    /// `llama-server.exe` (bundled with the app).
    pub exe: PathBuf,
    pub model: PathBuf,
    /// Context window in tokens.
    pub ctx: u32,
    /// CPU threads; `None` lets llama.cpp decide.
    pub threads: Option<u32>,
    /// Where llama-server's output goes, for the diagnostics page.
    pub log: Option<PathBuf>,
    /// Extra command-line arguments from Settings (advanced).
    pub extra_args: Vec<String>,
}

impl ServerConfig {
    /// Upper bound of llama-server's prompt cache, in MiB.
    pub const CACHE_RAM_MIB: u32 = 256;

    pub fn args(&self, port: u16, api_key: &str) -> Vec<String> {
        let mut args = vec![
            "--model".into(),
            self.model.to_string_lossy().into_owned(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            port.to_string(),
            "--api-key".into(),
            api_key.into(),
            "--ctx-size".into(),
            self.ctx.to_string(),
            // One request at a time: the whole context goes to it.
            "--parallel".into(),
            "1".into(),
            // Use the model's own chat template (needed for template arguments).
            "--jinja".into(),
            // Answer directly; no hidden reasoning.
            "--reasoning-budget".into(),
            "0".into(),
            "--no-webui".into(),
            // A small host-memory prompt cache. Without it the instructions are re-read on
            // every request (the default models use sliding-window or recurrent layers,
            // which the slot can't partly reuse): 4 to 7 s slower per answer in the
            // evaluation. The default (8 GB) is too much to allow on an 8 GB laptop.
            "--cache-ram".into(),
            Self::CACHE_RAM_MIB.to_string(),
        ];
        if let Some(t) = self.threads {
            args.push("--threads".into());
            args.push(t.to_string());
        }
        args.extend(self.extra_args.iter().cloned());
        args
    }
}

/// Called with the process id right after llama-server starts.
pub type SpawnHook = dyn Fn(u32) + Send + Sync;

pub struct LlamaServer {
    child: tokio::process::Child,
    target: Target,
    config: ServerConfig,
}

impl LlamaServer {
    /// Starts the server and waits until the model is loaded.
    pub async fn start(config: ServerConfig, ready_timeout: Duration) -> Result<Self, EngineError> {
        Self::start_with(config, ready_timeout, None).await
    }

    /// Like `start`, calling `on_spawn` with the process id as soon as it exists (before
    /// the model has loaded), e.g. to tie it to the app's lifetime.
    pub async fn start_with(
        config: ServerConfig,
        ready_timeout: Duration,
        on_spawn: Option<&SpawnHook>,
    ) -> Result<Self, EngineError> {
        if !config.exe.is_file() {
            return Err(EngineError::MissingEngine(config.exe.clone()));
        }
        if !config.model.is_file() {
            return Err(EngineError::MissingModel(config.model.clone()));
        }
        // Absolute paths: the server runs in its own folder (for its DLLs), so relative
        // paths would point somewhere else. (`absolute` avoids Windows `\\?\` paths.)
        let mut config = config;
        config.exe = std::path::absolute(&config.exe).map_err(EngineError::Io)?;
        config.model = std::path::absolute(&config.model).map_err(EngineError::Io)?;
        let port = free_port()?;
        let api_key = random_key();

        let mut cmd = tokio::process::Command::new(&config.exe);
        cmd.args(config.args(port, &api_key))
            .stdin(Stdio::null())
            .kill_on_drop(true);
        if let Some(dir) = config.exe.parent() {
            // The DLLs next to llama-server.exe are found from its own folder.
            cmd.current_dir(dir);
        }
        match &config.log {
            Some(path) => {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let file = std::fs::File::create(path).map_err(EngineError::Io)?;
                let file2 = file.try_clone().map_err(EngineError::Io)?;
                cmd.stdout(Stdio::from(file)).stderr(Stdio::from(file2));
            }
            None => {
                cmd.stdout(Stdio::null()).stderr(Stdio::null());
            }
        }
        #[cfg(windows)]
        {
            // CREATE_NO_WINDOW: no console window flashes up.
            cmd.creation_flags(0x0800_0000);
        }
        let child = cmd.spawn().map_err(EngineError::Io)?;
        if let (Some(hook), Some(pid)) = (on_spawn, child.id()) {
            hook(pid);
        }
        let target = Target {
            base_url: format!("http://127.0.0.1:{port}"),
            api_key: Some(api_key),
            model: None,
            llama_server: true,
        };
        let mut server = LlamaServer {
            child,
            target,
            config,
        };
        server.wait_ready(ready_timeout).await?;
        Ok(server)
    }

    async fn wait_ready(&mut self, timeout: Duration) -> Result<(), EngineError> {
        let client = Client::new();
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().map_err(EngineError::Io)? {
                return Err(EngineError::Exited {
                    code: status.code(),
                    log_tail: log_tail(self.config.log.as_deref()),
                });
            }
            if let Ok(true) = client.health(&self.target).await {
                return Ok(());
            }
            if start.elapsed() > timeout {
                let _ = self.child.start_kill();
                return Err(EngineError::StartTimeout {
                    seconds: timeout.as_secs(),
                    log_tail: log_tail(self.config.log.as_deref()),
                });
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }

    pub fn target(&self) -> &Target {
        &self.target
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// The OS process id (to put it in a job object on Windows).
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// False once the process has exited (crashed or killed).
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub async fn stop(mut self) {
        let _ = self.child.start_kill();
        let _ = tokio::time::timeout(Duration::from_secs(5), self.child.wait()).await;
    }
}

fn free_port() -> Result<u16, EngineError> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).map_err(EngineError::Io)?;
    Ok(listener.local_addr().map_err(EngineError::Io)?.port())
}

fn random_key() -> String {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).expect("OS random numbers");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The last lines of the server log, for error messages.
pub fn log_tail(path: Option<&Path>) -> String {
    let Some(path) = path else {
        return String::new();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(12)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_include_safety_settings() {
        let cfg = ServerConfig {
            exe: "llama-server.exe".into(),
            model: "m.gguf".into(),
            ctx: 4096,
            threads: Some(4),
            log: None,
            extra_args: vec!["--mlock".into()],
        };
        let args = cfg.args(5555, "k");
        let joined = args.join(" ");
        assert!(joined.contains("--host 127.0.0.1"));
        assert!(joined.contains("--port 5555"));
        assert!(joined.contains("--api-key k"));
        assert!(joined.contains("--ctx-size 4096"));
        assert!(joined.contains("--threads 4"));
        assert!(joined.contains("--no-webui"));
        assert!(joined.contains("--cache-ram 256"));
        assert!(joined.ends_with("--mlock"));
    }

    #[test]
    fn keys_are_random_and_long() {
        let a = random_key();
        assert_eq!(a.len(), 48);
        assert_ne!(a, random_key());
    }
}
