//! AI Translate: which languages the local model may translate into, and whether a
//! selection can be translated into the language chosen in Settings.
//!
//! A language can be an AI target only if the language guard knows its script
//! (`language::LANGUAGES`): the grammar and the script check are what keep a small model's
//! translation in the right language. Others are offered for the web Translate tab only.

use serde::Serialize;

use crate::language::{self, Language};

/// A language the "Translate into" setting offers: its code (as Google Translate takes it,
/// for the web tab's `{lang}`), English name and own name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Choice {
    pub code: &'static str,
    pub name: &'static str,
    pub native: &'static str,
}

const fn choice(code: &'static str, name: &'static str, native: &'static str) -> Choice {
    Choice { code, name, native }
}

/// Every language the setting offers, in the order shown.
pub const CHOICES: &[Choice] = &[
    choice("en", "English", "English"),
    choice("bn", "Bangla", "বাংলা"),
    choice("hi", "Hindi", "हिन्दी"),
    choice("ur", "Urdu", "اردو"),
    choice("ar", "Arabic", "العربية"),
    choice("es", "Spanish", "Español"),
    choice("fr", "French", "Français"),
    choice("de", "German", "Deutsch"),
    choice("pt", "Portuguese", "Português"),
    choice("it", "Italian", "Italiano"),
    choice("nl", "Dutch", "Nederlands"),
    choice("ru", "Russian", "Русский"),
    choice("uk", "Ukrainian", "Українська"),
    choice("tr", "Turkish", "Türkçe"),
    choice("id", "Indonesian", "Bahasa Indonesia"),
    choice("vi", "Vietnamese", "Tiếng Việt"),
    choice("zh-CN", "Chinese (Simplified)", "简体中文"),
    choice("zh-TW", "Chinese (Traditional)", "繁體中文"),
    choice("ja", "Japanese", "日本語"),
    choice("ko", "Korean", "한국어"),
    choice("th", "Thai", "ไทย"),
];

/// The AI target for a code from Settings: "bn", "pt-BR" (Portuguese), "zh-CN" (Chinese).
/// Traditional Chinese ("zh-TW", "zh-Hant", "zh-HK") has none: the prompt and the guard
/// are for Simplified Chinese.
pub fn target(code: &str) -> Option<&'static Language> {
    let code = code.trim().to_ascii_lowercase();
    if let Some(l) = language::LANGUAGES.iter().find(|l| l.code == code) {
        return Some(l);
    }
    let (primary, rest) = code.split_once(['-', '_'])?;
    if primary == "zh" && !matches!(rest, "cn" | "hans" | "sg" | "hans-cn") {
        return None;
    }
    language::language(primary)
}

/// Whether the AI can translate a selection, as the card shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Availability {
    Available {
        /// The code from Settings.
        code: String,
        /// The language's English name, for "Translate to Bangla".
        name: String,
    },
    /// The selection is already in the target language ("Already in English").
    SameLanguage { name: String },
    /// The language guard has no script for this language: the web Translate tab only.
    Unsupported { name: String },
}

/// The selection's language: a word or short term says little about its language, so the
/// sentence around it decides when the word alone can't.
fn detected(selection: &str, context: Option<&str>) -> Option<&'static Language> {
    language::detect_known(selection).or_else(|| context.and_then(language::detect_known))
}

/// Can `selection` (cleaned) be translated by the AI into `translate_to`? When the
/// language can't be told, translating is allowed.
pub fn availability(selection: &str, context: Option<&str>, translate_to: &str) -> Availability {
    let name = display_name(translate_to);
    let Some(to) = target(translate_to) else {
        return Availability::Unsupported { name };
    };
    let from = detected(selection, context);
    if from.is_some_and(|l| l.code == to.code) {
        return Availability::SameLanguage { name };
    }
    Availability::Available {
        code: translate_to.to_string(),
        name,
    }
}

/// A language as the card names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Named {
    pub code: String,
    pub name: String,
}

/// What the card's AI tab offers to translate into, and the selection's own language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Offer {
    #[serde(flatten)]
    pub availability: Availability,
    /// The selection's language when it can be told ("English · detected").
    pub source: Option<Named>,
}

/// Like Google Translate: text already in the chosen language goes into English instead
/// (Bangla text with "Translate into" set to Bangla is translated to English).
pub fn offer(selection: &str, context: Option<&str>, translate_to: &str) -> Offer {
    let from = detected(selection, context);
    let mut availability = availability(selection, context, translate_to);
    if matches!(availability, Availability::SameLanguage { .. })
        && from.is_some_and(|l| l.code != "en")
    {
        availability = Availability::Available {
            code: "en".into(),
            name: "English".into(),
        };
    }
    Offer {
        availability,
        source: from.map(|l| Named {
            code: l.code.into(),
            name: display_name(l.code),
        }),
    }
}

/// The short English name the setting shows ("Bangla"), or the code for one it doesn't
/// list.
fn display_name(code: &str) -> String {
    CHOICES
        .iter()
        .find(|c| c.code.eq_ignore_ascii_case(code.trim()))
        .map(|c| c.name.to_string())
        .or_else(|| target(code).map(|l| l.name.to_string()))
        .unwrap_or_else(|| code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_codes_map_to_guarded_languages() {
        let code = |c: &str| target(c).map(|l| l.code);
        assert_eq!(code("bn"), Some("bn"));
        assert_eq!(code(" BN "), Some("bn"));
        assert_eq!(code("zh-CN"), Some("zh"));
        assert_eq!(code("zh-Hans"), Some("zh"));
        assert_eq!(code("zh"), Some("zh"));
        assert_eq!(code("pt-BR"), Some("pt"));
        assert_eq!(code("en_GB"), Some("en"));
        assert_eq!(code("zh-TW"), None, "Traditional Chinese isn't Simplified");
        assert_eq!(code("zh-Hant"), None);
        assert_eq!(code("th"), None, "no Thai in the guard's languages");
        assert_eq!(code("xx-YY"), None);
        assert_eq!(code(""), None);
    }

    #[test]
    fn every_choice_is_either_an_ai_target_or_web_only() {
        let web_only: Vec<&str> = CHOICES
            .iter()
            .filter(|c| target(c.code).is_none())
            .map(|c| c.code)
            .collect();
        assert_eq!(web_only, vec!["zh-TW", "th"]);
        // Codes are unique, and every guarded language can be chosen.
        for (i, c) in CHOICES.iter().enumerate() {
            assert!(
                CHOICES[i + 1..].iter().all(|d| d.code != c.code),
                "{}",
                c.code
            );
        }
        for l in language::LANGUAGES {
            assert!(
                CHOICES.iter().any(|c| target(c.code) == Some(l)),
                "{} can't be chosen",
                l.code
            );
        }
    }

    #[test]
    fn availability() {
        use Availability::*;
        let en = "The tenant must pay the rent on the first day of each month.";
        assert_eq!(
            super::availability(en, None, "bn"),
            Available {
                code: "bn".into(),
                name: "Bangla".into()
            }
        );
        assert_eq!(
            super::availability(en, None, "en"),
            SameLanguage {
                name: "English".into()
            }
        );
        assert_eq!(
            super::availability("ভাড়া প্রতি মাসের প্রথম দিনে দিতে হবে।", None, "bn"),
            SameLanguage {
                name: "Bangla".into()
            }
        );
        assert_eq!(
            super::availability("租户应在每月第一天支付租金。", None, "zh-CN"),
            SameLanguage {
                name: "Chinese (Simplified)".into()
            }
        );
        assert_eq!(
            super::availability(en, None, "th"),
            Unsupported {
                name: "Thai".into()
            }
        );
        assert_eq!(
            super::availability(en, None, "sw"),
            Unsupported { name: "sw".into() }
        );
        // A bare word can't tell its language: allowed...
        assert!(matches!(
            super::availability("bank", None, "en"),
            Available { .. }
        ));
        // ...unless its sentence can.
        assert!(matches!(
            super::availability("bank", Some("We sat on the bank of the river."), "en"),
            SameLanguage { .. }
        ));
        assert!(matches!(
            super::availability("banco", Some("Nos sentamos en el banco del parque."), "en"),
            Available { .. }
        ));
        // A word in a script says its language on its own.
        assert!(matches!(
            super::availability("ভাড়া", Some("The word is used in Dhaka."), "bn"),
            SameLanguage { .. }
        ));
        // Only numbers: unknown, allowed.
        assert!(matches!(
            super::availability("12,345", None, "en"),
            Available { .. }
        ));
    }

    #[test]
    fn text_in_the_chosen_language_goes_into_english() {
        let bn = "ভাড়া প্রতি মাসের প্রথম দিনে দিতে হবে।";
        let o = super::offer(bn, None, "bn");
        assert_eq!(
            o.availability,
            Availability::Available {
                code: "en".into(),
                name: "English".into()
            }
        );
        assert_eq!(o.source.unwrap().code, "bn");
        // English text in English: nothing to translate into.
        let en = "The tenant must pay the rent on the first day of each month.";
        assert!(matches!(
            super::offer(en, None, "en").availability,
            Availability::SameLanguage { .. }
        ));
        // Otherwise the chosen language, with the source named.
        let o = super::offer(en, None, "bn");
        assert!(matches!(o.availability, Availability::Available { ref code, .. } if code == "bn"));
        assert_eq!(o.source.unwrap().name, "English");
        // A bare number has no language.
        assert_eq!(super::offer("12,345", None, "bn").source, None);
    }

    #[test]
    fn offer_serializes_for_the_card() {
        let json = serde_json::to_value(super::offer("ভাড়া দিন।", None, "bn")).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"state": "available", "code": "en", "name": "English",
                "source": {"code": "bn", "name": "Bangla"}})
        );
    }

    #[test]
    fn availability_serializes_for_the_card() {
        let json = serde_json::to_value(super::availability("bank", None, "es")).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"state": "available", "code": "es", "name": "Spanish"})
        );
    }
}
