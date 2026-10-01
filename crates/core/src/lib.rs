//! Text Explainer core.
//!
//! Everything that isn't Windows UI lives here so it can be tested on any platform:
//! text cleanup and classification, readability scores, the meaning and language guards,
//! prompts, the local model engine (llama-server), the dictionary, models and settings.

pub mod engine;
pub mod explain;
pub mod language;
pub mod meaning;
pub mod output;
pub mod prompt;
pub mod readability;
pub mod text;

/// Installs the TLS backend (rustls with ring) once per process. Every HTTP client in the
/// app calls this first; downloads need it, localhost requests don't mind it.
pub fn tls_init() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
pub mod download;
pub mod models;
pub mod settings;
