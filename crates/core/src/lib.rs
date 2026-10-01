//! Text Explainer core.
//!
//! Everything that isn't Windows UI lives here so it can be tested on any platform:
//! text cleanup and classification, readability scores, the meaning and language guards,
//! prompts, the local model engine (llama-server), the dictionary, models and settings.

pub mod text;
