//! The language guard: small models (Qwen in particular) sometimes drift into another
//! language mid-answer, most often Chinese. Asking nicely in the prompt isn't enough, so:
//!
//! 1. Every request carries a GBNF grammar that makes characters from unexpected scripts
//!    impossible to sample. A script is allowed only if it is Latin (names, units and
//!    terms appear in every language), the output language's script, or already in the
//!    selected text.
//! 2. The finished answer is checked again, for endpoints that ignore grammars; the app
//!    retries once with a stricter instruction and otherwise shows an error.
//!
//! It also detects the language of the selection so prompts can name it ("Write in
//! Spanish") instead of saying "the same language", which small models follow poorly.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// Writing systems, grouped the way drift happens (Han, kana and CJK punctuation together).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Script {
    Latin,
    Greek,
    Cyrillic,
    Armenian,
    Hebrew,
    Arabic,
    Devanagari,
    Bengali,
    Gurmukhi,
    Gujarati,
    Oriya,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    Sinhala,
    Thai,
    Lao,
    Tibetan,
    Myanmar,
    Georgian,
    Ethiopic,
    Khmer,
    Mongolian,
    /// Han characters, bopomofo, CJK punctuation and full-width forms.
    Cjk,
    /// Hiragana and katakana.
    Kana,
    Hangul,
}

/// Unicode ranges per script. Ranges not listed (digits, punctuation, symbols, emoji)
/// are never restricted.
const RANGES: &[(Script, u32, u32)] = &[
    (Script::Latin, 0x0041, 0x005A),
    (Script::Latin, 0x0061, 0x007A),
    (Script::Latin, 0x00C0, 0x00D6),
    (Script::Latin, 0x00D8, 0x00F6),
    (Script::Latin, 0x00F8, 0x024F),
    (Script::Latin, 0x1E00, 0x1EFF),
    (Script::Greek, 0x0370, 0x03FF),
    (Script::Greek, 0x1F00, 0x1FFF),
    (Script::Cyrillic, 0x0400, 0x052F),
    (Script::Cyrillic, 0x1C80, 0x1C8F),
    (Script::Cyrillic, 0x2DE0, 0x2DFF),
    (Script::Cyrillic, 0xA640, 0xA69F),
    (Script::Armenian, 0x0530, 0x058F),
    (Script::Hebrew, 0x0590, 0x05FF),
    (Script::Arabic, 0x0600, 0x06FF),
    (Script::Arabic, 0x0750, 0x077F),
    (Script::Arabic, 0x08A0, 0x08FF),
    (Script::Arabic, 0xFB50, 0xFDFF),
    (Script::Arabic, 0xFE70, 0xFEFF),
    // The danda (U+0964, U+0965) is shared punctuation: Bengali and others use it too.
    (Script::Devanagari, 0x0900, 0x0963),
    (Script::Devanagari, 0x0966, 0x097F),
    (Script::Bengali, 0x0980, 0x09FF),
    (Script::Gurmukhi, 0x0A00, 0x0A7F),
    (Script::Gujarati, 0x0A80, 0x0AFF),
    (Script::Oriya, 0x0B00, 0x0B7F),
    (Script::Tamil, 0x0B80, 0x0BFF),
    (Script::Telugu, 0x0C00, 0x0C7F),
    (Script::Kannada, 0x0C80, 0x0CFF),
    (Script::Malayalam, 0x0D00, 0x0D7F),
    (Script::Sinhala, 0x0D80, 0x0DFF),
    (Script::Thai, 0x0E00, 0x0E7F),
    (Script::Lao, 0x0E80, 0x0EFF),
    (Script::Tibetan, 0x0F00, 0x0FFF),
    (Script::Myanmar, 0x1000, 0x109F),
    (Script::Georgian, 0x10A0, 0x10FF),
    (Script::Hangul, 0x1100, 0x11FF),
    (Script::Ethiopic, 0x1200, 0x137F),
    (Script::Khmer, 0x1780, 0x17FF),
    (Script::Mongolian, 0x1800, 0x18AF),
    (Script::Cjk, 0x2E80, 0x2FDF),
    (Script::Cjk, 0x3000, 0x303F),
    (Script::Kana, 0x3040, 0x30FF),
    (Script::Cjk, 0x3100, 0x312F),
    (Script::Hangul, 0x3130, 0x318F),
    (Script::Cjk, 0x31A0, 0x31BF),
    (Script::Kana, 0x31F0, 0x31FF),
    (Script::Cjk, 0x3200, 0x33FF),
    (Script::Cjk, 0x3400, 0x4DBF),
    (Script::Cjk, 0x4E00, 0x9FFF),
    (Script::Hangul, 0xA960, 0xA97F),
    (Script::Hangul, 0xAC00, 0xD7AF),
    (Script::Hangul, 0xD7B0, 0xD7FF),
    (Script::Cjk, 0xF900, 0xFAFF),
    (Script::Cjk, 0xFE30, 0xFE4F),
    (Script::Cjk, 0xFF00, 0xFF60),
    (Script::Kana, 0xFF61, 0xFF9F),
    (Script::Hangul, 0xFFA0, 0xFFDC),
    (Script::Cjk, 0xFFE0, 0xFFEF),
    (Script::Kana, 0x1B000, 0x1B16F),
    (Script::Cjk, 0x20000, 0x2FA1F),
    (Script::Cjk, 0x30000, 0x323AF),
];

pub fn script_of(c: char) -> Option<Script> {
    let cp = c as u32;
    if cp < 0x41 {
        return None;
    }
    RANGES
        .iter()
        .find(|&&(_, lo, hi)| (lo..=hi).contains(&cp))
        .map(|&(s, _, _)| s)
}

/// Counts letters per script.
pub fn script_counts(text: &str) -> Vec<(Script, usize)> {
    let mut counts: std::collections::BTreeMap<Script, usize> = Default::default();
    for c in text.chars() {
        if let Some(s) = script_of(c) {
            *counts.entry(s).or_default() += 1;
        }
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

pub fn dominant_script(text: &str) -> Option<Script> {
    script_counts(text).first().map(|&(s, _)| s)
}

/// A language the explanation can be written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Language {
    pub code: &'static str,
    /// The English name, used in prompts ("Write in Bangla").
    pub name: &'static str,
    /// The name in the language itself, for the settings list.
    pub native: &'static str,
    pub scripts: &'static [Script],
}

pub const LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        name: "English",
        native: "English",
        scripts: &[Script::Latin],
    },
    Language {
        code: "bn",
        name: "Bangla (Bengali)",
        native: "বাংলা",
        scripts: &[Script::Bengali],
    },
    Language {
        code: "hi",
        name: "Hindi",
        native: "हिन्दी",
        scripts: &[Script::Devanagari],
    },
    Language {
        code: "ur",
        name: "Urdu",
        native: "اردو",
        scripts: &[Script::Arabic],
    },
    Language {
        code: "ar",
        name: "Arabic",
        native: "العربية",
        scripts: &[Script::Arabic],
    },
    Language {
        code: "es",
        name: "Spanish",
        native: "Español",
        scripts: &[Script::Latin],
    },
    Language {
        code: "fr",
        name: "French",
        native: "Français",
        scripts: &[Script::Latin],
    },
    Language {
        code: "de",
        name: "German",
        native: "Deutsch",
        scripts: &[Script::Latin],
    },
    Language {
        code: "pt",
        name: "Portuguese",
        native: "Português",
        scripts: &[Script::Latin],
    },
    Language {
        code: "it",
        name: "Italian",
        native: "Italiano",
        scripts: &[Script::Latin],
    },
    Language {
        code: "nl",
        name: "Dutch",
        native: "Nederlands",
        scripts: &[Script::Latin],
    },
    Language {
        code: "id",
        name: "Indonesian",
        native: "Bahasa Indonesia",
        scripts: &[Script::Latin],
    },
    Language {
        code: "tr",
        name: "Turkish",
        native: "Türkçe",
        scripts: &[Script::Latin],
    },
    Language {
        code: "vi",
        name: "Vietnamese",
        native: "Tiếng Việt",
        scripts: &[Script::Latin],
    },
    Language {
        code: "ru",
        name: "Russian",
        native: "Русский",
        scripts: &[Script::Cyrillic],
    },
    Language {
        code: "uk",
        name: "Ukrainian",
        native: "Українська",
        scripts: &[Script::Cyrillic],
    },
    Language {
        code: "zh",
        name: "Chinese (Simplified)",
        native: "简体中文",
        scripts: &[Script::Cjk],
    },
    Language {
        code: "ja",
        name: "Japanese",
        native: "日本語",
        scripts: &[Script::Cjk, Script::Kana],
    },
    Language {
        code: "ko",
        name: "Korean",
        native: "한국어",
        scripts: &[Script::Hangul, Script::Cjk],
    },
];

pub fn language(code: &str) -> Option<&'static Language> {
    LANGUAGES.iter().find(|l| l.code == code)
}

/// Very common words that identify Latin-script languages from a few sentences.
const STOPWORDS: &[(&str, &[&str])] = &[
    (
        "en",
        &[
            "the", "and", "of", "to", "is", "in", "that", "it", "for", "with", "as", "are", "this",
            "be", "by", "not", "or", "which", "have", "from",
        ],
    ),
    (
        "es",
        &[
            "el", "la", "de", "que", "y", "en", "los", "las", "se", "del", "por", "un", "una",
            "con", "para", "es", "al", "lo", "como", "más",
        ],
    ),
    (
        "fr",
        &[
            "le", "la", "les", "de", "des", "et", "est", "une", "un", "du", "que", "qui", "dans",
            "pour", "pas", "sur", "au", "avec", "ce", "sont",
        ],
    ),
    (
        "de",
        &[
            "der", "die", "und", "das", "ist", "nicht", "ein", "eine", "zu", "den", "von", "mit",
            "sich", "des", "auf", "für", "im", "dem", "auch", "werden",
        ],
    ),
    (
        "pt",
        &[
            "o", "a", "os", "as", "de", "que", "e", "do", "da", "em", "um", "uma", "para", "com",
            "não", "por", "mais", "se", "dos", "das",
        ],
    ),
    (
        "it",
        &[
            "il", "la", "di", "che", "e", "è", "un", "una", "per", "non", "in", "del", "della",
            "sono", "con", "gli", "le", "si", "da", "anche",
        ],
    ),
    (
        "nl",
        &[
            "de", "het", "een", "en", "van", "is", "dat", "niet", "op", "te", "zijn", "voor",
            "met", "die", "aan", "er", "ook", "als", "bij", "wordt",
        ],
    ),
    (
        "id",
        &[
            "yang", "dan", "di", "ini", "itu", "dengan", "untuk", "tidak", "dari", "dalam", "akan",
            "pada", "juga", "ke", "karena", "ada", "adalah", "oleh", "atau", "bisa",
        ],
    ),
    (
        "tr",
        &[
            "ve", "bir", "bu", "da", "de", "için", "ile", "olarak", "çok", "daha", "gibi", "olan",
            "ama", "ne", "değil", "en", "mi", "ki", "kadar", "sonra",
        ],
    ),
    (
        "vi",
        &[
            "và", "của", "là", "có", "không", "được", "cho", "trong", "một", "những", "người",
            "này", "với", "các", "để", "đã", "khi", "từ", "như", "cũng",
        ],
    ),
];

/// Best guess at the language of `text`, or `None` if it can't tell.
pub fn detect(text: &str) -> Option<&'static Language> {
    let script = dominant_script(text)?;
    let by_script = |s: Script| LANGUAGES.iter().find(|l| l.scripts.first() == Some(&s));
    match script {
        Script::Latin => {
            let words: Vec<String> = text
                .split(|c: char| !c.is_alphabetic())
                .filter(|w| !w.is_empty())
                .take(400)
                .map(str::to_lowercase)
                .collect();
            if words.is_empty() {
                return None;
            }
            let mut best: Option<(&str, usize)> = None;
            for &(code, list) in STOPWORDS {
                let hits = words.iter().filter(|w| list.contains(&w.as_str())).count();
                if hits > best.map_or(0, |b| b.1) {
                    best = Some((code, hits));
                }
            }
            match best {
                // Single words and short phrases have no stopwords: assume English, the
                // language most people select terms from. Callers can override.
                None => language("en"),
                Some((code, _)) => language(code),
            }
        }
        // Text with any kana is Japanese, even though kanji outnumber it.
        Script::Cjk if text.chars().any(|c| script_of(c) == Some(Script::Kana)) => language("ja"),
        Script::Arabic => {
            // Urdu-only letters: ٹ ڈ ڑ ں ے ھ.
            if text
                .chars()
                .any(|c| matches!(c, 'ٹ' | 'ڈ' | 'ڑ' | 'ں' | 'ے' | 'ھ'))
            {
                language("ur")
            } else {
                language("ar")
            }
        }
        Script::Cyrillic => {
            if text
                .chars()
                .any(|c| matches!(c, 'і' | 'ї' | 'є' | 'ґ' | 'І' | 'Ї' | 'Є' | 'Ґ'))
            {
                language("uk")
            } else {
                language("ru")
            }
        }
        s => by_script(s),
    }
}

/// The scripts an answer may use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowedScripts(pub BTreeSet<Script>);

impl AllowedScripts {
    /// Latin, the output language's scripts, and every script that appears in the source
    /// at least `min_chars` times (one stray symbol doesn't open the door).
    pub fn for_request(source: &str, output: Option<&Language>) -> Self {
        let mut set = BTreeSet::from([Script::Latin]);
        if let Some(lang) = output {
            set.extend(lang.scripts.iter().copied());
        }
        for (script, n) in script_counts(source) {
            if n >= 2 {
                set.insert(script);
            }
        }
        AllowedScripts(set)
    }

    pub fn allows(&self, s: Script) -> bool {
        self.0.contains(&s)
    }

    /// A GBNF grammar (llama.cpp) accepting any text without letters from other scripts.
    pub fn grammar(&self) -> String {
        let mut class = String::new();
        for &(script, lo, hi) in RANGES {
            if !self.allows(script) {
                let _ = write!(class, "{}-{}", escape(lo), escape(hi));
            }
        }
        if class.is_empty() {
            return "root ::= [^\\x00]*\n".to_string();
        }
        format!("root ::= [^{class}]*\n")
    }

    /// Letters from scripts that aren't allowed, with a short sample for the error.
    pub fn violations(&self, output: &str) -> Option<Drift> {
        let mut scripts = BTreeSet::new();
        let mut sample = String::new();
        let mut count = 0;
        for c in output.chars() {
            if let Some(s) = script_of(c)
                && !self.allows(s)
            {
                scripts.insert(s);
                count += 1;
                if sample.chars().count() < 12 {
                    sample.push(c);
                }
            }
        }
        (count > 0).then_some(Drift {
            scripts,
            count,
            sample,
        })
    }
}

fn escape(cp: u32) -> String {
    if cp > 0xFFFF {
        format!("\\U{cp:08X}")
    } else {
        format!("\\u{cp:04X}")
    }
}

/// An answer that slipped into another script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Drift {
    pub scripts: BTreeSet<Script>,
    pub count: usize,
    pub sample: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_scripts() {
        assert_eq!(script_of('a'), Some(Script::Latin));
        assert_eq!(script_of('é'), Some(Script::Latin));
        assert_eq!(script_of('ß'), Some(Script::Latin));
        assert_eq!(script_of('×'), None, "multiplication sign is a symbol");
        assert_eq!(script_of('÷'), None);
        assert_eq!(script_of('1'), None);
        assert_eq!(script_of('.'), None);
        assert_eq!(script_of('€'), None);
        assert_eq!(script_of('µ'), None);
        assert_eq!(script_of('中'), Some(Script::Cjk));
        assert_eq!(script_of('。'), Some(Script::Cjk));
        assert_eq!(script_of('，'), Some(Script::Cjk));
        assert_eq!(script_of('の'), Some(Script::Kana));
        assert_eq!(script_of('한'), Some(Script::Hangul));
        assert_eq!(script_of('ব'), Some(Script::Bengali));
        assert_eq!(script_of('ক'), Some(Script::Bengali));
        assert_eq!(script_of('я'), Some(Script::Cyrillic));
        assert_eq!(script_of('α'), Some(Script::Greek));
        assert_eq!(script_of('𠀋'), Some(Script::Cjk));
    }

    #[test]
    fn ranges_do_not_overlap() {
        let mut sorted: Vec<_> = RANGES.iter().map(|&(_, lo, hi)| (lo, hi)).collect();
        sorted.sort();
        for w in sorted.windows(2) {
            assert!(w[0].1 < w[1].0, "{:X?} overlaps {:X?}", w[0], w[1]);
            assert!(w[0].0 <= w[0].1);
        }
    }

    #[test]
    fn detects_languages() {
        let code = |t: &str| detect(t).map(|l| l.code);
        assert_eq!(
            code("The tenant shall pay the rent on the first day of each month."),
            Some("en")
        );
        assert_eq!(
            code("El inquilino pagará el alquiler el primer día de cada mes."),
            Some("es")
        );
        assert_eq!(
            code("Le locataire paiera le loyer le premier jour de chaque mois."),
            Some("fr")
        );
        assert_eq!(
            code("Der Mieter zahlt die Miete am ersten Tag des Monats und nicht später."),
            Some("de")
        );
        assert_eq!(
            code("ভাড়াটে প্রতি মাসের প্রথম দিনে ভাড়া পরিশোধ করবেন।"),
            Some("bn")
        );
        assert_eq!(code("किरायेदार हर महीने के पहले दिन किराया देगा।"), Some("hi"));
        assert_eq!(code("租户应在每月第一天支付租金。"), Some("zh"));
        assert_eq!(code("借主は毎月一日に家賃を支払う。"), Some("ja"));
        assert_eq!(code("Арендатор платит в первый день месяца."), Some("ru"));
        assert_eq!(
            code("Орендар платить у перший день місяця, і все."),
            Some("uk")
        );
        assert_eq!(
            code("کرایہ دار ہر مہینے کی پہلی تاریخ کو کرایہ ادا کرے گا۔"),
            Some("ur")
        );
        assert_eq!(code("ubiquitous"), Some("en"));
        assert_eq!(code("12345 !!"), None);
    }

    #[test]
    fn english_source_forbids_chinese_but_allows_latin() {
        let allowed = AllowedScripts::for_request("The rent is due monthly.", language("en"));
        assert!(allowed.allows(Script::Latin));
        assert!(!allowed.allows(Script::Cjk));
        let drift = allowed
            .violations("The rent is due 每个月 each month.")
            .unwrap();
        assert_eq!(drift.scripts, BTreeSet::from([Script::Cjk]));
        assert_eq!(drift.count, 3);
        assert_eq!(drift.sample, "每个月");
        assert!(
            allowed
                .violations("Rent: 1,200 € — due on the 1st (×12/year).")
                .is_none()
        );
    }

    #[test]
    fn scripts_in_the_source_stay_allowed() {
        let src = "The word 道 (dào) means \"the way\" in Chinese philosophy, written 道路.";
        let allowed = AllowedScripts::for_request(src, language("en"));
        assert!(allowed.allows(Script::Cjk));
        // One stray character doesn't open a script.
        let allowed = AllowedScripts::for_request("Cost: 5 元 per item.", language("en"));
        assert!(!allowed.allows(Script::Cjk));
    }

    #[test]
    fn bangla_output_allows_bengali_and_latin() {
        let allowed = AllowedScripts::for_request("The rent is due monthly.", language("bn"));
        assert!(allowed.allows(Script::Bengali));
        assert!(allowed.allows(Script::Latin));
        assert!(
            allowed
                .violations("ভাড়া প্রতি মাসে দিতে হয় (monthly)।")
                .is_none()
        );
        assert!(allowed.violations("ভাড়া 租金").is_some());
    }

    #[test]
    fn grammar_excludes_every_other_script() {
        let allowed = AllowedScripts::for_request("Plain English text here.", language("en"));
        let g = allowed.grammar();
        assert!(g.starts_with("root ::= [^"));
        assert!(g.trim_end().ends_with("]*"));
        assert!(g.contains("\\u4E00-\\u9FFF"), "Han excluded");
        assert!(g.contains("\\u3040-\\u30FF"), "kana excluded");
        assert!(g.contains("\\uAC00-\\uD7AF"), "Hangul excluded");
        assert!(
            g.contains("\\U00020000-\\U0002FA1F"),
            "supplementary Han excluded"
        );
        assert!(!g.contains("\\u0041-\\u005A"), "Latin allowed");
        // Grammar character classes must be ascending pairs.
        for pair in g["root ::= [^".len()..g.len() - 4]
            .split("\\u")
            .filter(|p| !p.is_empty())
        {
            assert!(pair.len() >= 4, "{pair}");
        }
    }

    #[test]
    fn grammar_with_everything_allowed_still_parses() {
        let all = AllowedScripts(RANGES.iter().map(|r| r.0).collect());
        assert_eq!(all.grammar(), "root ::= [^\\x00]*\n");
    }

    #[test]
    fn every_language_has_a_known_script() {
        for l in LANGUAGES {
            assert!(!l.scripts.is_empty(), "{}", l.code);
            assert!(language(l.code).is_some());
        }
    }
}
