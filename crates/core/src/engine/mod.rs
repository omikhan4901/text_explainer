//! The local model engine: starts llama-server when first needed, keeps it warm while
//! the person reads, and unloads it after a quiet period to give the RAM back.

pub mod client;
pub mod server;

use std::path::PathBuf;
use std::time::{Duration, Instant};

pub use client::{Client, Completion, Sampling, Target, Timings};
pub use server::{LlamaServer, ServerConfig, SpawnHook};

use crate::prompt::Message;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("the bundled model engine is missing ({0}); reinstall Text Explainer")]
    MissingEngine(PathBuf),
    #[error("the model file is missing ({0}); choose or download a model in Settings")]
    MissingModel(PathBuf),
    #[error("no model is set up yet")]
    NoModel,
    #[error("the model engine stopped unexpectedly{}", exit_suffix(.code))]
    Exited { code: Option<i32>, log_tail: String },
    #[error("the model took more than {seconds} s to load")]
    StartTimeout { seconds: u64, log_tail: String },
    #[error("couldn't reach the model ({0})")]
    Connection(String),
    #[error("the model server answered {status}: {body}")]
    Http { status: u16, body: String },
    #[error("the model reported an error: {0}")]
    Model(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn exit_suffix(code: &Option<i32>) -> String {
    code.map(|c| format!(" (exit code {c})"))
        .unwrap_or_default()
}

impl EngineError {
    pub(crate) fn from_reqwest(e: reqwest::Error) -> Self {
        EngineError::Connection(e.without_url().to_string())
    }
}

/// Where answers come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backend {
    /// No model chosen yet.
    None,
    /// The bundled llama-server with a local GGUF file.
    Local(ServerConfig),
    /// An OpenAI-compatible server the person already runs (Ollama, LM Studio, …).
    Endpoint {
        base_url: String,
        api_key: Option<String>,
        model: String,
    },
}

pub struct Engine {
    backend: Backend,
    server: Option<LlamaServer>,
    client: Client,
    last_used: Instant,
    ready_timeout: Duration,
    on_spawn: Option<Box<server::SpawnHook>>,
}

impl Engine {
    pub fn new(backend: Backend) -> Self {
        Engine {
            backend,
            server: None,
            client: Client::new(),
            last_used: Instant::now(),
            ready_timeout: Duration::from_secs(120),
            on_spawn: None,
        }
    }

    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    /// Switches model or endpoint. A running server is stopped if it no longer matches.
    pub async fn set_backend(&mut self, backend: Backend) {
        if backend != self.backend {
            self.unload().await;
            self.backend = backend;
        }
    }

    pub fn is_loaded(&mut self) -> bool {
        self.server.as_mut().is_some_and(LlamaServer::is_running)
    }

    pub fn idle_for(&self) -> Duration {
        self.last_used.elapsed()
    }

    pub fn server_pid(&self) -> Option<u32> {
        self.server.as_ref().and_then(LlamaServer::pid)
    }

    /// Starts the model if needed (or restarts it after a crash) and returns the target.
    pub async fn ensure_ready(&mut self) -> Result<Target, EngineError> {
        self.last_used = Instant::now();
        match &self.backend {
            Backend::None => Err(EngineError::NoModel),
            Backend::Endpoint {
                base_url,
                api_key,
                model,
            } => Ok(Target {
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                model: Some(model.clone()),
                llama_server: false,
            }),
            Backend::Local(config) => {
                if let Some(server) = self.server.as_mut()
                    && server.is_running()
                {
                    return Ok(server.target().clone());
                }
                if let Some(dead) = self.server.take() {
                    dead.stop().await;
                }
                let server = LlamaServer::start_with(
                    config.clone(),
                    self.ready_timeout,
                    self.on_spawn.as_deref(),
                )
                .await?;
                let target = server.target().clone();
                self.server = Some(server);
                Ok(target)
            }
        }
    }

    /// Runs one chat request, streaming text to `on_delta`.
    pub async fn chat(
        &mut self,
        messages: &[Message],
        sampling: &Sampling,
        grammar: Option<&str>,
        on_delta: impl FnMut(&str),
    ) -> Result<Completion, EngineError> {
        let target = self.ensure_ready().await?;
        let result = self
            .client
            .chat(&target, messages, sampling, grammar, on_delta)
            .await;
        self.last_used = Instant::now();
        result
    }

    /// Whether requests can carry a GBNF grammar.
    pub fn supports_grammar(&self) -> bool {
        matches!(self.backend, Backend::Local(_))
    }

    /// Stops the local server (frees its RAM). The next request starts it again.
    pub async fn unload(&mut self) {
        if let Some(server) = self.server.take() {
            server.stop().await;
        }
    }

    /// Runs `hook` with the process id each time llama-server is started.
    pub fn set_spawn_hook(&mut self, hook: impl Fn(u32) + Send + Sync + 'static) {
        self.on_spawn = Some(Box::new(hook));
    }

    pub fn set_ready_timeout(&mut self, timeout: Duration) {
        self.ready_timeout = timeout;
    }
}
