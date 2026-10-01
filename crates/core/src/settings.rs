//! Settings, stored as JSON in the app's config folder. Unknown or missing fields fall
//! back to defaults, out-of-range values are clamped, and a file that can't be read is
//! kept aside (never silently overwritten) so nothing the person set is lost for good.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::explain::OutputChoice;
use crate::prompt::Level;

pub const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    /// Global shortcut in Tauri's format, e.g. "Ctrl+Shift+Space".
    pub hotkey: String,
    /// Pressing Ctrl+C twice explains the copied text.
    pub double_copy: bool,
    /// When an app doesn't expose its selection, the hotkey may copy it (the clipboard is
    /// restored afterwards).
    pub clipboard_fallback: bool,
    pub level: Level,
    pub output: OutputChoice,
    pub model: ModelChoice,
    pub temperature: f32,
    pub top_p: f32,
    pub context_size: u32,
    /// CPU threads for the model; `None` lets llama.cpp choose.
    pub threads: Option<u32>,
    /// Unload the model after this many idle minutes (0 = keep it loaded).
    pub idle_unload_minutes: u32,
    pub prompts: CustomPrompts,
    /// Extra llama-server arguments (advanced).
    pub extra_server_args: Vec<String>,
    pub appearance: Appearance,
    pub start_with_windows: bool,
    pub first_run_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: CURRENT_VERSION,
            hotkey: "Ctrl+Shift+Space".into(),
            double_copy: true,
            clipboard_fallback: true,
            level: Level::Plain,
            output: OutputChoice::SameAsText,
            model: ModelChoice::None,
            temperature: 0.3,
            top_p: 0.9,
            context_size: 4096,
            threads: None,
            idle_unload_minutes: 10,
            prompts: CustomPrompts::default(),
            extra_server_args: Vec::new(),
            appearance: Appearance::default(),
            start_with_windows: true,
            first_run_done: false,
        }
    }
}

/// Which model answers.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModelChoice {
    #[default]
    None,
    /// A model from the catalog, downloaded into the app's models folder.
    Catalog { id: String },
    /// Any GGUF file the person picked.
    File { path: PathBuf },
    /// A local OpenAI-compatible server (Ollama, LM Studio, …).
    Endpoint {
        base_url: String,
        model: String,
        #[serde(default)]
        api_key: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomPrompts {
    pub simpler: Option<String>,
    pub plain: Option<String>,
    pub clearer: Option<String>,
}

impl CustomPrompts {
    pub fn for_level(&self, level: Level) -> Option<&str> {
        match level {
            Level::Simpler => self.simpler.as_deref(),
            Level::Plain => self.plain.as_deref(),
            Level::Clearer => self.clearer.as_deref(),
        }
        .filter(|p| !p.trim().is_empty())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Accent {
    #[default]
    Plum,
    Saffron,
    Garnet,
    Ink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingFont {
    #[default]
    Plex,
    /// Atkinson Hyperlegible: designed for low-vision readers.
    Atkinson,
    Serif,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub theme: Theme,
    pub accent: Accent,
    pub reading_font: ReadingFont,
    /// Card text size in CSS pixels.
    pub font_size: u32,
    pub line_height: f32,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            theme: Theme::System,
            accent: Accent::Plum,
            reading_font: ReadingFont::Plex,
            font_size: 15,
            line_height: 1.6,
        }
    }
}

impl Settings {
    /// Brings every value into its allowed range.
    pub fn sanitized(mut self) -> Self {
        self.version = CURRENT_VERSION;
        self.temperature = finite_or(self.temperature, 0.3).clamp(0.0, 2.0);
        self.top_p = finite_or(self.top_p, 0.9).clamp(0.05, 1.0);
        self.context_size = self.context_size.clamp(1024, 32_768);
        self.threads = self.threads.map(|t| t.clamp(1, 64));
        self.idle_unload_minutes = self.idle_unload_minutes.min(24 * 60);
        self.appearance.font_size = self.appearance.font_size.clamp(12, 24);
        self.appearance.line_height = finite_or(self.appearance.line_height, 1.6).clamp(1.2, 2.2);
        if self.hotkey.trim().is_empty() {
            self.hotkey = Settings::default().hotkey;
        }
        self.extra_server_args.retain(|a| !a.trim().is_empty());
        self
    }

    /// Loads settings. A missing file gives defaults; an unreadable one is renamed to
    /// `settings.broken.json` (so it can be recovered) and defaults are used.
    pub fn load(path: &Path) -> Settings {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Settings::default();
        };
        match serde_json::from_str::<Settings>(&text) {
            Ok(s) => s.sanitized(),
            Err(e) => {
                tracing::warn!(
                    "settings file unreadable ({e}); keeping it aside and using defaults"
                );
                let _ = std::fs::rename(path, path.with_file_name("settings.broken.json"));
                Settings::default()
            }
        }
    }

    /// Saves atomically: a crash mid-write never leaves a half-written file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).expect("settings serialise"),
        )?;
        std::fs::rename(&tmp, path)
    }
}

fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() { v } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut s = Settings::default();
        s.level = Level::Simpler;
        s.model = ModelChoice::Endpoint {
            base_url: "http://127.0.0.1:11434/v1".into(),
            model: "qwen3.5:4b".into(),
            api_key: None,
        };
        s.output = OutputChoice::Fixed("bn".into());
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            Settings::load(&dir.path().join("nope.json")),
            Settings::default()
        );
    }

    #[test]
    fn partial_files_keep_what_they_have() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"level":"clearer","appearance":{"font_size":18}}"#,
        )
        .unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.level, Level::Clearer);
        assert_eq!(s.appearance.font_size, 18);
        assert_eq!(s.appearance.line_height, 1.6);
        assert_eq!(s.hotkey, "Ctrl+Shift+Space");
    }

    #[test]
    fn broken_files_are_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        assert!(dir.path().join("settings.broken.json").exists());
        assert!(!path.exists());
    }

    #[test]
    fn values_are_clamped() {
        let s = Settings {
            temperature: f32::NAN,
            top_p: 7.0,
            context_size: 10,
            threads: Some(0),
            hotkey: "  ".into(),
            appearance: Appearance {
                font_size: 99,
                line_height: 0.5,
                ..Default::default()
            },
            extra_server_args: vec!["".into(), "--mlock".into()],
            ..Default::default()
        }
        .sanitized();
        assert_eq!(s.temperature, 0.3);
        assert_eq!(s.top_p, 1.0);
        assert_eq!(s.context_size, 1024);
        assert_eq!(s.threads, Some(1));
        assert_eq!(s.hotkey, "Ctrl+Shift+Space");
        assert_eq!(s.appearance.font_size, 24);
        assert_eq!(s.appearance.line_height, 1.2);
        assert_eq!(s.extra_server_args, vec!["--mlock".to_string()]);
    }

    #[test]
    fn empty_custom_prompts_are_ignored() {
        let p = CustomPrompts {
            plain: Some("  ".into()),
            simpler: Some("Be simple.".into()),
            clearer: None,
        };
        assert_eq!(p.for_level(Level::Plain), None);
        assert_eq!(p.for_level(Level::Simpler), Some("Be simple."));
    }
}
