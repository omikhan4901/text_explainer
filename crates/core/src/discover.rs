//! Finds GGUF models the person already has, so they can use one without downloading
//! again: LM Studio, Jan, GPT4All, the Hugging Face cache, Ollama and the Downloads folder.
//! Only local folders are read, nothing is changed, and the walk is bounded so a huge
//! folder can't stall the app.

use serde::Serialize;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::models::{Fit, fit};

/// Smallest file taken for a model (vocabularies and test files are smaller).
pub const MIN_MODEL_BYTES: u64 = 50 * 1024 * 1024;
/// Most directory entries looked at in one search.
const MAX_ENTRIES: usize = 50_000;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FoundModel {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    /// Where it was found ("LM Studio", "Ollama", …).
    pub source: String,
    pub ram_needed_bytes: u64,
    pub fit: Fit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    /// Any `.gguf` files under the folder.
    Files,
    /// Ollama's store: manifests name the model, blobs hold the GGUF data.
    Ollama,
}

#[derive(Debug, Clone)]
pub struct Root {
    pub source: String,
    pub dir: PathBuf,
    pub kind: RootKind,
    /// How many folders deep to look.
    pub depth: usize,
}

impl Root {
    fn files(source: &str, dir: PathBuf, depth: usize) -> Root {
        Root {
            source: source.into(),
            dir,
            kind: RootKind::Files,
            depth,
        }
    }
}

/// The usual places, from the environment (Windows paths, with a Linux/macOS home too).
pub fn default_roots() -> Vec<Root> {
    let var = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let mut roots = Vec::new();
    if let Some(home) = var("USERPROFILE").or_else(|| var("HOME")) {
        roots.push(Root::files(
            "LM Studio",
            home.join(".lmstudio").join("models"),
            4,
        ));
        roots.push(Root::files(
            "LM Studio",
            home.join(".cache").join("lm-studio").join("models"),
            4,
        ));
        roots.push(Root::files("Jan", home.join("jan").join("models"), 4));
        roots.push(Root::files(
            "Hugging Face",
            home.join(".cache").join("huggingface").join("hub"),
            5,
        ));
        roots.push(Root::files("Downloads", home.join("Downloads"), 1));
        roots.push(Root {
            source: "Ollama".into(),
            dir: var("OLLAMA_MODELS").unwrap_or_else(|| home.join(".ollama").join("models")),
            kind: RootKind::Ollama,
            depth: 0,
        });
    }
    if let Some(hf) = var("HF_HOME") {
        roots.push(Root::files("Hugging Face", hf.join("hub"), 5));
    }
    if let Some(app_data) = var("APPDATA") {
        roots.push(Root::files(
            "Jan",
            app_data.join("Jan").join("data").join("models"),
            4,
        ));
    }
    if let Some(local) = var("LOCALAPPDATA") {
        roots.push(Root::files(
            "GPT4All",
            local.join("nomic.ai").join("GPT4All"),
            2,
        ));
    }
    roots
}

/// Searches `roots` and returns the models found, best fit first. `exclude` is the app's
/// own models folder (its downloads are listed with the catalog already).
pub fn discover(
    roots: &[Root],
    total_ram_bytes: u64,
    exclude: Option<&Path>,
    min_bytes: u64,
) -> Vec<FoundModel> {
    let exclude = exclude.and_then(|p| p.canonicalize().ok());
    let mut seen = HashSet::new();
    let mut budget = MAX_ENTRIES;
    let mut found = Vec::new();
    for root in roots {
        let mut add = |name: String, path: PathBuf, size: u64| {
            let Ok(real) = path.canonicalize() else {
                return;
            };
            if exclude.as_ref().is_some_and(|e| real.starts_with(e)) || !seen.insert(real) {
                return;
            }
            let ram = size + size / 2;
            found.push(FoundModel {
                name,
                path,
                size_bytes: size,
                source: root.source.clone(),
                ram_needed_bytes: ram,
                fit: fit(ram, total_ram_bytes),
            });
        };
        match root.kind {
            RootKind::Files => walk(&root.dir, root.depth, min_bytes, &mut budget, &mut add),
            RootKind::Ollama => ollama(&root.dir, min_bytes, &mut add),
        }
    }
    let rank = |f: Fit| match f {
        Fit::Comfortable => 0,
        Fit::Tight => 1,
        Fit::TooBig => 2,
    };
    found.sort_by(|a, b| {
        rank(a.fit)
            .cmp(&rank(b.fit))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    found
}

fn walk(
    dir: &Path,
    depth: usize,
    min_bytes: u64,
    budget: &mut usize,
    add: &mut impl FnMut(String, PathBuf, u64),
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            // Real folders only: a linked folder could loop.
            if depth > 0 {
                walk(&path, depth - 1, min_bytes, budget, add);
            }
            continue;
        }
        // Files, including links to files (the Hugging Face cache links snapshots to blobs).
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = model_stem(&name) else {
            continue;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() || meta.len() < min_bytes || !is_gguf(&path) {
            continue;
        }
        add(stem, path, meta.len() * split_parts(&name));
    }
}

/// The name to show for a GGUF file, or None when it isn't one to offer: not a .gguf,
/// a vision projector (needs a model next to it), or a later part of a split model.
fn model_stem(file_name: &str) -> Option<String> {
    let lower = file_name.to_ascii_lowercase();
    let stem = file_name.get(..file_name.len().checked_sub(5)?)?;
    if !lower.ends_with(".gguf") || lower.contains("mmproj") {
        return None;
    }
    if let Some((part, _)) = split_of(&lower)
        && part != 1
    {
        return None;
    }
    let base = match split_of(&lower) {
        // "name-00001-of-00003" → "name"
        Some(_) => &stem[..stem.len() - "-00001-of-00003".len()],
        None => stem,
    };
    Some(base.to_string())
}

/// For "…-00001-of-00003.gguf", (1, 3).
fn split_of(lower: &str) -> Option<(u32, u32)> {
    let stem = lower.strip_suffix(".gguf")?;
    let tail = stem.get(stem.len().checked_sub(15)?..)?;
    let (part, count) = tail.strip_prefix('-')?.split_once("-of-")?;
    if part.len() != 5 || count.len() != 5 {
        return None;
    }
    Some((part.parse().ok()?, count.parse().ok()?))
}

/// A split model's size is roughly its first part times the number of parts.
fn split_parts(file_name: &str) -> u64 {
    split_of(&file_name.to_ascii_lowercase()).map_or(1, |(_, n)| u64::from(n.max(1)))
}

/// GGUF files start with the bytes "GGUF".
fn is_gguf(path: &Path) -> bool {
    let mut magic = [0u8; 4];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut magic))
        .is_ok()
        && &magic == b"GGUF"
}

/// Ollama keeps `manifests/<host>/<namespace>/<model>/<tag>` (JSON) naming a model layer
/// stored as `blobs/sha256-<hex>`, which is a plain GGUF file.
fn ollama(dir: &Path, min_bytes: u64, add: &mut impl FnMut(String, PathBuf, u64)) {
    let Ok(hosts) = std::fs::read_dir(dir.join("manifests")) else {
        return;
    };
    for host in hosts.flatten() {
        let Ok(namespaces) = std::fs::read_dir(host.path()) else {
            continue;
        };
        for namespace in namespaces.flatten() {
            let Ok(models) = std::fs::read_dir(namespace.path()) else {
                continue;
            };
            for model in models.flatten() {
                let Ok(tags) = std::fs::read_dir(model.path()) else {
                    continue;
                };
                for tag in tags.flatten() {
                    let Some(blob) = ollama_blob(dir, &tag.path()) else {
                        continue;
                    };
                    let Ok(meta) = std::fs::metadata(&blob) else {
                        continue;
                    };
                    if meta.len() < min_bytes || !is_gguf(&blob) {
                        continue;
                    }
                    let ns = namespace.file_name().to_string_lossy().into_owned();
                    let name = format!(
                        "{}:{}",
                        model.file_name().to_string_lossy(),
                        tag.file_name().to_string_lossy()
                    );
                    add(
                        if ns == "library" {
                            name
                        } else {
                            format!("{ns}/{name}")
                        },
                        blob,
                        meta.len(),
                    );
                }
            }
        }
    }
}

fn ollama_blob(dir: &Path, manifest: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(manifest).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let digest = json["layers"]
        .as_array()?
        .iter()
        .find(|l| l["mediaType"] == "application/vnd.ollama.image.model")?["digest"]
        .as_str()?;
    let hex = digest.strip_prefix("sha256:")?;
    // The digest names a file: never let it point anywhere else.
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(dir.join("blobs").join(format!("sha256-{hex}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    fn gguf(path: &Path, len: usize) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut bytes = b"GGUF".to_vec();
        bytes.resize(len, 0);
        std::fs::write(path, bytes).unwrap();
    }

    fn root(source: &str, dir: &Path, depth: usize) -> Root {
        Root::files(source, dir.to_path_buf(), depth)
    }

    #[test]
    fn finds_gguf_files_and_skips_the_rest() {
        let d = tempfile::tempdir().unwrap();
        let lm = d.path().join("lmstudio");
        gguf(
            &lm.join("Qwen")
                .join("Qwen3-4B-GGUF")
                .join("Qwen3-4B-Q4_K_M.gguf"),
            2000,
        );
        gguf(
            &lm.join("Qwen")
                .join("Qwen3-4B-GGUF")
                .join("mmproj-Qwen3-4B-f16.gguf"),
            2000,
        );
        gguf(&lm.join("tiny.gguf"), 10); // too small
        std::fs::write(lm.join("fake.gguf"), vec![0u8; 2000]).unwrap(); // not GGUF inside
        std::fs::write(lm.join("notes.txt"), "GGUF").unwrap();
        gguf(
            &lm.join("a")
                .join("b")
                .join("c")
                .join("d")
                .join("e")
                .join("deep.gguf"),
            2000,
        ); // too deep

        let found = discover(&[root("LM Studio", &lm, 3)], 8 * GB, None, 1000);
        let names: Vec<_> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["Qwen3-4B-Q4_K_M"]);
        assert_eq!(found[0].source, "LM Studio");
        assert_eq!(found[0].size_bytes, 2000);
        assert_eq!(found[0].ram_needed_bytes, 3000);
        assert_eq!(found[0].fit, Fit::Comfortable);
    }

    #[test]
    fn split_models_show_once_with_their_whole_size() {
        let d = tempfile::tempdir().unwrap();
        for part in 1..=3 {
            gguf(
                &d.path().join(format!("big-model-0000{part}-of-00003.gguf")),
                2000,
            );
        }
        let found = discover(&[root("Downloads", d.path(), 0)], 8 * GB, None, 1000);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "big-model");
        assert_eq!(found[0].size_bytes, 6000);
        assert!(found[0].path.ends_with("big-model-00001-of-00003.gguf"));
        assert_eq!(model_stem("x-1-of-3.gguf").as_deref(), Some("x-1-of-3"));
        assert_eq!(model_stem("MODEL.GGUF").as_deref(), Some("MODEL"));
        assert_eq!(model_stem("gguf"), None);
    }

    #[test]
    fn reads_ollama_manifests() {
        let d = tempfile::tempdir().unwrap();
        let store = d.path().join("ollama");
        let hex = "a".repeat(64);
        gguf(&store.join("blobs").join(format!("sha256-{hex}")), 2000);
        let manifest = format!(
            r#"{{"layers":[{{"mediaType":"application/vnd.ollama.image.license","digest":"sha256:{bad}"}},{{"mediaType":"application/vnd.ollama.image.model","digest":"sha256:{hex}"}}]}}"#,
            bad = "b".repeat(64)
        );
        let lib = store
            .join("manifests")
            .join("registry.ollama.ai")
            .join("library")
            .join("qwen3.5");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(lib.join("4b"), &manifest).unwrap();
        let user = store
            .join("manifests")
            .join("registry.ollama.ai")
            .join("someone")
            .join("tuned");
        std::fs::create_dir_all(&user).unwrap();
        std::fs::write(user.join("latest"), &manifest).unwrap(); // same blob: listed once
        // A digest that tries to leave the blobs folder is ignored.
        let evil = store
            .join("manifests")
            .join("registry.ollama.ai")
            .join("library")
            .join("evil");
        std::fs::create_dir_all(&evil).unwrap();
        std::fs::write(
            evil.join("x"),
            r#"{"layers":[{"mediaType":"application/vnd.ollama.image.model","digest":"sha256:../../../etc/passwd"}]}"#,
        )
        .unwrap();

        let roots = [Root {
            source: "Ollama".into(),
            dir: store,
            kind: RootKind::Ollama,
            depth: 0,
        }];
        let found = discover(&roots, 8 * GB, None, 1000);
        assert_eq!(found.len(), 1);
        assert!(found[0].name == "qwen3.5:4b" || found[0].name == "someone/tuned:latest");
        assert_eq!(found[0].source, "Ollama");
    }

    #[test]
    fn skips_the_apps_own_folder_and_duplicates() {
        let d = tempfile::tempdir().unwrap();
        let own = d.path().join("own");
        gguf(&own.join("gemma.gguf"), 2000);
        gguf(&d.path().join("other").join("llama.gguf"), 2000);
        let roots = [
            root("A", d.path(), 2),
            root("B", &d.path().join("other"), 0),
        ];
        let found = discover(&roots, 8 * GB, Some(&own), 1000);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "llama");
        assert_eq!(found[0].source, "A");
    }

    #[test]
    fn best_fit_first_and_missing_folders_are_fine() {
        let d = tempfile::tempdir().unwrap();
        gguf(&d.path().join("zz-small.gguf"), 2000);
        gguf(&d.path().join("aa-huge.gguf"), 9000);
        let roots = [
            root("X", d.path(), 0),
            root("Missing", &d.path().join("nope"), 3),
        ];
        let found = discover(&roots, 5_000, None, 1000);
        let order: Vec<_> = found.iter().map(|f| (f.name.as_str(), f.fit)).collect();
        assert_eq!(order, [("zz-small", Fit::Tight), ("aa-huge", Fit::TooBig)]);
    }

    #[test]
    fn default_roots_cover_the_usual_apps() {
        let sources: HashSet<_> = default_roots().into_iter().map(|r| r.source).collect();
        // The home folder is always set where the tests run.
        for s in ["LM Studio", "Jan", "Hugging Face", "Downloads", "Ollama"] {
            assert!(sources.contains(s), "{s}");
        }
    }
}
