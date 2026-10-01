//! The model catalog. Sizes and SHA-256 hashes come from Hugging Face's API (see
//! `.github/workflows/probe.yml`), so every download is checked against a known file.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Smallest and quickest; for older or busy machines.
    Fastest,
    Fast,
    /// Between the fastest and the best: the default class.
    Balanced,
    /// Best quality; needs 16 GB of RAM.
    Best,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogModel {
    pub id: &'static str,
    pub name: &'static str,
    pub maker: &'static str,
    pub tier: Tier,
    /// Parameter count as people quote it ("4B").
    pub params: &'static str,
    pub quant: &'static str,
    pub file_name: &'static str,
    pub url: &'static str,
    pub size_bytes: u64,
    pub sha256: &'static str,
    pub license: &'static str,
    pub license_url: &'static str,
    /// A short, honest note shown in the catalog.
    pub note: &'static str,
}

impl CatalogModel {
    /// Rough memory needed while loaded: weights plus context cache and runtime.
    pub fn ram_needed_bytes(&self) -> u64 {
        self.size_bytes + 900 * MB
    }

    pub fn fit(&self, total_ram_bytes: u64) -> Fit {
        fit(self.ram_needed_bytes(), total_ram_bytes)
    }
}

const MB: u64 = 1024 * 1024;

/// Whether a model leaves enough memory for the browser or document being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// Uses at most half the RAM.
    Comfortable,
    /// Uses up to 70%: works, but other apps may slow down.
    Tight,
    TooBig,
}

pub fn fit(needed: u64, total: u64) -> Fit {
    if total == 0 {
        return Fit::Tight;
    }
    if needed * 2 <= total {
        Fit::Comfortable
    } else if needed * 10 <= total * 7 {
        Fit::Tight
    } else {
        Fit::TooBig
    }
}

/// The default when nothing else is known; the eval (`docs/models.md`) decides it.
pub const DEFAULT_MODEL: &str = "qwen3.5-4b";

pub const CATALOG: &[CatalogModel] = &[
    CatalogModel {
        id: "qwen3.5-4b",
        name: "Qwen3.5 4B",
        maker: "Alibaba Qwen",
        tier: Tier::Balanced,
        params: "4B",
        quant: "Q4_K_M",
        file_name: "Qwen3.5-4B-Q4_K_M.gguf",
        url: "https://huggingface.co/unsloth/Qwen3.5-4B-GGUF/resolve/main/Qwen3.5-4B-Q4_K_M.gguf",
        size_bytes: 2_740_937_888,
        sha256: "00fe7986ff5f6b463e62455821146049db6f9313603938a70800d1fb69ef11a4",
        license: "Apache 2.0",
        license_url: "https://www.apache.org/licenses/LICENSE-2.0",
        note: "Strong instruction following for its size; many languages.",
    },
    CatalogModel {
        id: "gemma-4-e2b",
        name: "Gemma 4 E2B",
        maker: "Google",
        tier: Tier::Balanced,
        params: "2B effective",
        quant: "QAT Q4_K_XL",
        file_name: "gemma-4-E2B-it-qat-UD-Q4_K_XL.gguf",
        url: "https://huggingface.co/unsloth/gemma-4-E2B-it-qat-GGUF/resolve/main/gemma-4-E2B-it-qat-UD-Q4_K_XL.gguf",
        size_bytes: 2_620_370_976,
        sha256: "e531007218dfab990486a5de7676a6932d6ea8dea233d1f698d7c21cf8a16889",
        license: "Apache 2.0",
        license_url: "https://www.apache.org/licenses/LICENSE-2.0",
        note: "Built for laptops and phones; quantisation-aware trained.",
    },
    CatalogModel {
        id: "qwen3.5-2b",
        name: "Qwen3.5 2B",
        maker: "Alibaba Qwen",
        tier: Tier::Fast,
        params: "2B",
        quant: "Q4_K_M",
        file_name: "Qwen3.5-2B-Q4_K_M.gguf",
        url: "https://huggingface.co/unsloth/Qwen3.5-2B-GGUF/resolve/main/Qwen3.5-2B-Q4_K_M.gguf",
        size_bytes: 1_280_835_840,
        sha256: "aaf42c8b7c3cab2bf3d69c355048d4a0ee9973d48f16c731c0520ee914699223",
        license: "Apache 2.0",
        license_url: "https://www.apache.org/licenses/LICENSE-2.0",
        note: "About twice as fast as the 4B; simpler rewrites.",
    },
    CatalogModel {
        id: "lfm2.5-1.2b",
        name: "LFM2.5 1.2B",
        maker: "Liquid AI",
        tier: Tier::Fastest,
        params: "1.2B",
        quant: "Q4_K_M",
        file_name: "LFM2.5-1.2B-Instruct-Q4_K_M.gguf",
        url: "https://huggingface.co/LiquidAI/LFM2.5-1.2B-Instruct-GGUF/resolve/main/LFM2.5-1.2B-Instruct-Q4_K_M.gguf",
        size_bytes: 730_895_168,
        sha256: "b1b3de114215d9507409a662a501a631095a479a419584e8a2ded6304b19b4f5",
        license: "LFM Open License v1.0",
        license_url: "https://huggingface.co/LiquidAI/LFM2.5-1.2B-Instruct/blob/main/LICENSE",
        note: "Very fast on any CPU. Free for personal use and smaller companies; check the license.",
    },
    CatalogModel {
        id: "gemma-4-e4b",
        name: "Gemma 4 E4B",
        maker: "Google",
        tier: Tier::Best,
        params: "4B effective",
        quant: "QAT Q4_K_XL",
        file_name: "gemma-4-E4B-it-qat-UD-Q4_K_XL.gguf",
        url: "https://huggingface.co/unsloth/gemma-4-E4B-it-qat-GGUF/resolve/main/gemma-4-E4B-it-qat-UD-Q4_K_XL.gguf",
        size_bytes: 4_215_695_776,
        sha256: "df0fd4ee07072c607c29a0a1cb4f98918426cca12f45a2776bdd6ee6d09a4de3",
        license: "Apache 2.0",
        license_url: "https://www.apache.org/licenses/LICENSE-2.0",
        note: "Higher quality, larger download; best with 12 GB of RAM or more.",
    },
    CatalogModel {
        id: "qwen3.5-9b",
        name: "Qwen3.5 9B",
        maker: "Alibaba Qwen",
        tier: Tier::Best,
        params: "9B",
        quant: "Q4_K_M",
        file_name: "Qwen3.5-9B-Q4_K_M.gguf",
        url: "https://huggingface.co/unsloth/Qwen3.5-9B-GGUF/resolve/main/Qwen3.5-9B-Q4_K_M.gguf",
        size_bytes: 5_680_522_464,
        sha256: "03b74727a860a56338e042c4420bb3f04b2fec5734175f4cb9fa853daf52b7e8",
        license: "Apache 2.0",
        license_url: "https://www.apache.org/licenses/LICENSE-2.0",
        note: "The most capable here; needs 16 GB of RAM and is slower on CPU.",
    },
];

pub fn by_id(id: &str) -> Option<&'static CatalogModel> {
    CATALOG.iter().find(|m| m.id == id)
}

/// The model to suggest on first run for a machine with this much RAM.
pub fn recommend(total_ram_bytes: u64) -> &'static CatalogModel {
    let default = by_id(DEFAULT_MODEL).expect("default model in catalog");
    if default.fit(total_ram_bytes) != Fit::TooBig {
        return default;
    }
    CATALOG
        .iter()
        .filter(|m| matches!(m.tier, Tier::Fast | Tier::Fastest))
        .find(|m| m.fit(total_ram_bytes) != Fit::TooBig)
        .unwrap_or_else(|| by_id("lfm2.5-1.2b").expect("smallest model in catalog"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * MB;

    #[test]
    fn catalog_is_well_formed() {
        let mut ids = std::collections::HashSet::new();
        for m in CATALOG {
            assert!(ids.insert(m.id), "duplicate id {}", m.id);
            assert_eq!(m.sha256.len(), 64, "{}", m.id);
            assert!(
                m.sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            );
            assert!(m.url.starts_with("https://huggingface.co/"));
            assert!(m.url.ends_with(m.file_name), "{}", m.id);
            assert!(m.file_name.ends_with(".gguf"));
            assert!(m.size_bytes > 100 * MB);
        }
        assert!(by_id(DEFAULT_MODEL).is_some());
    }

    #[test]
    fn eight_gigabyte_laptops_get_the_balanced_default() {
        let m = recommend(8 * GB);
        assert_eq!(m.id, DEFAULT_MODEL);
        assert_eq!(m.fit(8 * GB), Fit::Comfortable);
        assert_eq!(by_id("qwen3.5-9b").unwrap().fit(8 * GB), Fit::TooBig);
    }

    #[test]
    fn small_machines_get_smaller_models() {
        assert_eq!(recommend(4 * GB).id, "qwen3.5-2b");
        assert_eq!(recommend(2 * GB).id, "lfm2.5-1.2b");
    }

    #[test]
    fn unknown_memory_is_treated_as_tight() {
        assert_eq!(fit(GB, 0), Fit::Tight);
    }
}
