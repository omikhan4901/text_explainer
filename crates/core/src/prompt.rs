//! Prompts for each kind of request. Small models follow short, numbered, concrete rules
//! best, with one worked example. The system prompt and example are identical across
//! requests at a level, so llama-server's prompt cache makes them nearly free after the
//! first request.

use serde::{Deserialize, Serialize};

use crate::language::Language;

/// How far the rewrite simplifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// About grade 6: very short sentences, everyday words.
    Simpler,
    /// About grade 8–9: plain language. The default.
    #[default]
    Plain,
    /// Same level and vocabulary, better structure.
    Clearer,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Simpler, Level::Plain, Level::Clearer];

    fn style(self) -> &'static str {
        match self {
            Level::Simpler => {
                "Write for a 12-year-old: sentences under 12 words, the most common everyday words, one idea per sentence."
            }
            Level::Plain => {
                "Use plain language: sentences under 20 words, common words, active voice."
            }
            Level::Clearer => {
                "Keep the original vocabulary and level, but make it easier to follow: split long sentences, untangle nested clauses, use active voice."
            }
        }
    }

    fn example(self) -> &'static str {
        match self {
            Level::Simpler => {
                "The tenant (the Lessee) must pay $1,200 rent every month. They must pay it by the 5th day of the month. This rule comes before any other rule in the agreement."
            }
            Level::Plain => {
                "Whatever else this agreement says, the tenant (the Lessee) must pay the monthly rent of $1,200 by the 5th day of each month."
            }
            Level::Clearer => {
                "Despite anything else in this agreement, the Lessee must pay the monthly rent of $1,200 by the 5th day of each calendar month."
            }
        }
    }

    /// How much longer than the source a rewrite at this level may be, in tokens per word.
    fn tokens_per_source_word(self) -> f32 {
        match self {
            Level::Simpler => 2.6,
            Level::Plain => 2.2,
            Level::Clearer => 2.0,
        }
    }
}

const EXAMPLE_SOURCE: &str = "Notwithstanding any provision herein to the contrary, the Lessee shall remit payment of the monthly rent of $1,200 no later than the 5th day of each calendar month.";

/// Who said what, in OpenAI chat format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

fn msg(role: Role, content: impl Into<String>) -> Message {
    Message {
        role,
        content: content.into(),
    }
}

/// Where the language to write in comes from.
#[derive(Debug, Clone, Copy)]
pub enum OutputLanguage<'a> {
    Named(&'a Language),
    /// Couldn't tell; ask for the text's own language.
    SameAsText,
}

impl OutputLanguage<'_> {
    fn phrase(&self) -> String {
        match self {
            OutputLanguage::Named(l) => l.name.to_string(),
            OutputLanguage::SameAsText => "the same language as the text".to_string(),
        }
    }

    fn is_english(&self) -> bool {
        matches!(self, OutputLanguage::Named(l) if l.code == "en")
    }
}

/// Custom wording for a level, from Settings → Model → Prompts. `{language}` is replaced.
pub type CustomSystemPrompt<'a> = Option<&'a str>;

/// The system prompt for a rewrite (shown in Settings so people can edit it).
pub fn rewrite_system(level: Level, out: OutputLanguage<'_>) -> String {
    let lang = out.phrase();
    format!(
        "You make hard text easy to read. You rewrite the text the user gives you.\n\
         \n\
         Rules:\n\
         1. Keep the meaning exactly. Do not add facts, opinions, advice or warnings. Do not leave out facts.\n\
         2. Keep every number, date, amount, percentage and name exactly as written.\n\
         3. {style}\n\
         4. Explain a technical term in a few plain words the first time it appears.\n\
         5. Spell out an abbreviation the first time it appears, with the short form in brackets.\n\
         6. Keep the order of ideas. Keep lists as lists and keep paragraph breaks.\n\
         7. Write only in {lang}.\n\
         8. Reply with the rewritten text only: no title, no introduction, no notes.",
        style = level.style(),
    )
}

/// Messages for rewriting `text` (one chunk of `part.1` when a long text is split).
pub fn rewrite(
    text: &str,
    level: Level,
    out: OutputLanguage<'_>,
    custom: CustomSystemPrompt<'_>,
    part: Option<(usize, usize)>,
) -> Vec<Message> {
    let system = match custom {
        Some(c) if !c.trim().is_empty() => c.replace("{language}", &out.phrase()),
        _ => rewrite_system(level, out),
    };
    let mut messages = vec![msg(Role::System, system)];
    // The worked example is English; for other languages it would pull the answer
    // towards English, so it is left out.
    if out.is_english() {
        messages.push(msg(Role::User, wrap(EXAMPLE_SOURCE, None)));
        messages.push(msg(Role::Assistant, level.example()));
    }
    messages.push(msg(Role::User, wrap(text, part)));
    messages
}

fn wrap(text: &str, part: Option<(usize, usize)>) -> String {
    let note = match part {
        Some((i, n)) if n > 1 => {
            format!(" This is part {i} of {n} of a longer text; rewrite only this part.")
        }
        _ => String::new(),
    };
    format!("Rewrite this text.{note}\n\n<text>\n{text}\n</text>")
}

/// Messages for "what does this word mean here?".
pub fn word_meaning(word: &str, context: Option<&str>, out: OutputLanguage<'_>) -> Vec<Message> {
    let lang = out.phrase();
    let system = format!(
        "You explain words in plain language.\n\
         Rules:\n\
         1. Answer in one or two short sentences, in {lang}.\n\
         2. If a sentence is given, explain what the word means in that sentence.\n\
         3. Do not repeat the sentence. Do not add examples, synonyms lists or notes.\n\
         4. Reply with the explanation only."
    );
    vec![
        msg(Role::System, system),
        msg(Role::User, word_request(word, context)),
    ]
}

/// "Word: bank\nSentence: We sat on the river bank." (the sentence only when it adds
/// something).
fn word_request(word: &str, context: Option<&str>) -> String {
    match context {
        Some(c) if !c.trim().is_empty() && c.trim() != word.trim() => {
            format!("Word: {word}\nSentence: {}", c.trim())
        }
        _ => format!("Word: {word}"),
    }
}

/// The system prompt for translating a passage into `to`. No worked example: one in a
/// single language pair would pull other pairs towards it, and a wrong one would teach
/// the error to every request.
pub fn translate_system(to: &Language) -> String {
    let lang = to.name;
    format!(
        "You are a careful translator. You translate the text the user gives you into {lang}.\n\
         \n\
         Rules:\n\
         1. Translate all of it: every sentence, in the same order. Do not shorten, summarise or skip anything.\n\
         2. Keep the meaning exactly. Do not add facts, explanations, opinions or warnings.\n\
         3. Keep every number, amount, percentage and date. Write numbers with digits.\n\
         4. Keep the names of people, places, organisations and products, and acronyms; write them the way {lang} usually does.\n\
         5. Write plain, natural {lang} that is easy to read, the way a careful native speaker would.\n\
         6. Keep lists as lists and keep paragraph breaks.\n\
         7. Write only in {lang}. Do not add the original text, a transliteration or a pronunciation guide.\n\
         8. Reply with the translation only: no title, no introduction, no notes."
    )
}

/// Messages for translating `text` (one chunk of `part.1` when a long text is split).
pub fn translate(text: &str, to: &Language, part: Option<(usize, usize)>) -> Vec<Message> {
    let note = match part {
        Some((i, n)) if n > 1 => {
            format!(" This is part {i} of {n} of a longer text; translate only this part.")
        }
        _ => String::new(),
    };
    vec![
        msg(Role::System, translate_system(to)),
        msg(
            Role::User,
            format!(
                "Translate this text into {}.{note}\n\n<text>\n{text}\n</text>",
                to.name
            ),
        ),
    ]
}

/// Messages for translating a word or short term, with a short note on the sense it has
/// in its sentence when the sentence is known.
pub fn translate_word(word: &str, context: Option<&str>, to: &Language) -> Vec<Message> {
    let lang = to.name;
    let system = format!(
        "You translate words and short phrases into {lang}.\n\
         Rules:\n\
         1. Give the {lang} word or phrase that best translates it.\n\
         2. If a sentence is given, translate it as it is used in that sentence, then add a very short explanation in {lang} of that sense, in brackets.\n\
         3. Do not translate the sentence. Do not add examples, other meanings, a transliteration or notes.\n\
         4. Write only in {lang}.\n\
         5. Reply with the translation only."
    );
    vec![
        msg(Role::System, system),
        msg(Role::User, word_request(word, context)),
    ]
}

/// The most tokens a rewrite of `words` source words may take.
pub fn max_tokens_for_rewrite(words: usize, level: Level) -> u32 {
    ((words as f32 * level.tokens_per_source_word()) as u32 + 64).clamp(96, 3072)
}

/// A ceiling on the tokens a translation into `to` may take, per source word. Small
/// models' tokenizers split some scripts into many more pieces than English: a word of
/// Bangla or Hindi often takes 3 to 6 tokens, Arabic 2 to 4, Chinese about 1 per
/// character (an English word is about 1.5 characters). These are estimates, set high on
/// purpose: a model stops at the end of its answer, so a ceiling that is too high costs
/// nothing, and one that is too low cuts the translation short. `te-eval --task
/// translate` reports the tokens each model really used per source word.
fn translation_tokens_per_word(to: &Language) -> f32 {
    use crate::language::Script;
    match to.scripts.first() {
        Some(Script::Latin) => 2.5,
        Some(Script::Cyrillic | Script::Greek | Script::Armenian | Script::Georgian) => 3.5,
        Some(Script::Cjk | Script::Kana | Script::Hangul) => 3.5,
        Some(Script::Arabic | Script::Hebrew) => 4.0,
        // Indic scripts, Thai and the rest.
        _ => 6.0,
    }
}

/// How many English-sized words `text` has: Chinese and Japanese aren't split by spaces,
/// so their characters are counted (about 1.5 per word).
pub fn source_words(text: &str) -> usize {
    use crate::language::{Script, script_of};
    let dense = text
        .chars()
        .filter(|&c| matches!(script_of(c), Some(Script::Cjk | Script::Kana)))
        .count();
    crate::text::word_count(text).max(dense * 2 / 3)
}

/// The most tokens a translation of `text` into `to` may take.
pub fn max_tokens_for_translation(text: &str, to: &Language) -> u32 {
    ((source_words(text) as f32 * translation_tokens_per_word(to)) as u32 + 64).clamp(96, 3072)
}

/// The most tokens a word's translation and its short explanation (about 15 words) may
/// take.
pub fn max_tokens_for_word_translation(to: &Language) -> u32 {
    (48.0 + 20.0 * translation_tokens_per_word(to)) as u32
}

/// Prompt used when an answer drifted into another language and is retried.
pub fn language_reminder(out: OutputLanguage<'_>) -> Message {
    msg(
        Role::System,
        format!(
            "Important: write the whole answer in {} only. Do not use any other language or script.",
            out.phrase()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::language;

    #[test]
    fn english_rewrite_has_system_example_and_text() {
        let m = rewrite(
            "Hard text.",
            Level::Plain,
            OutputLanguage::Named(language("en").unwrap()),
            None,
            None,
        );
        assert_eq!(m.len(), 4);
        assert_eq!(m[0].role, Role::System);
        assert!(m[0].content.contains("Write only in English."));
        assert!(m[0].content.contains("sentences under 20 words"));
        assert_eq!(m[1].role, Role::User);
        assert_eq!(m[2].role, Role::Assistant);
        assert_eq!(
            m[3].content,
            "Rewrite this text.\n\n<text>\nHard text.\n</text>"
        );
    }

    #[test]
    fn other_languages_skip_the_english_example() {
        let m = rewrite(
            "Texto.",
            Level::Simpler,
            OutputLanguage::Named(language("es").unwrap()),
            None,
            None,
        );
        assert_eq!(m.len(), 2);
        assert!(m[0].content.contains("Write only in Spanish."));
        let m = rewrite(
            "Texto.",
            Level::Simpler,
            OutputLanguage::SameAsText,
            None,
            None,
        );
        assert!(
            m[0].content
                .contains("Write only in the same language as the text.")
        );
    }

    #[test]
    fn the_prefix_is_identical_across_requests_for_caching() {
        let en = OutputLanguage::Named(language("en").unwrap());
        let a = rewrite("One.", Level::Plain, en, None, None);
        let b = rewrite("Two, and different.", Level::Plain, en, None, None);
        assert_eq!(a[..3], b[..3]);
    }

    #[test]
    fn parts_are_labelled() {
        let m = rewrite(
            "Chunk.",
            Level::Plain,
            OutputLanguage::SameAsText,
            None,
            Some((2, 5)),
        );
        assert!(m.last().unwrap().content.contains("This is part 2 of 5"));
        let m = rewrite(
            "Chunk.",
            Level::Plain,
            OutputLanguage::SameAsText,
            None,
            Some((1, 1)),
        );
        assert!(!m.last().unwrap().content.contains("part"));
    }

    #[test]
    fn custom_prompt_replaces_the_system_message() {
        let en = OutputLanguage::Named(language("en").unwrap());
        let m = rewrite(
            "x",
            Level::Plain,
            en,
            Some("Be brief. Answer in {language}."),
            None,
        );
        assert_eq!(m[0].content, "Be brief. Answer in English.");
        let m = rewrite("x", Level::Plain, en, Some("   "), None);
        assert!(m[0].content.starts_with("You make hard text easy to read."));
    }

    #[test]
    fn levels_differ() {
        let en = OutputLanguage::Named(language("en").unwrap());
        let systems: Vec<String> = Level::ALL.iter().map(|&l| rewrite_system(l, en)).collect();
        assert_ne!(systems[0], systems[1]);
        assert_ne!(systems[1], systems[2]);
    }

    #[test]
    fn word_meaning_uses_context_when_it_adds_something() {
        let en = OutputLanguage::Named(language("en").unwrap());
        let m = word_meaning("bank", Some("We sat on the river bank."), en);
        assert_eq!(
            m[1].content,
            "Word: bank\nSentence: We sat on the river bank."
        );
        let m = word_meaning("bank", Some("bank"), en);
        assert_eq!(m[1].content, "Word: bank");
        let m = word_meaning("bank", None, en);
        assert_eq!(m[1].content, "Word: bank");
    }

    #[test]
    fn translation_prompt_names_the_target_and_forbids_commentary() {
        let bn = language("bn").unwrap();
        let m = translate("The rent is $1,200.", bn, None);
        assert_eq!(m.len(), 2, "no worked example");
        assert_eq!(m[0].role, Role::System);
        let s = &m[0].content;
        assert!(s.contains("into Bangla (Bengali)."));
        assert!(s.contains("Write only in Bangla (Bengali)."));
        assert!(s.contains("Translate all of it"));
        assert!(s.contains("Keep every number, amount, percentage and date"));
        assert!(s.contains("transliteration"));
        assert!(s.contains("no title, no introduction, no notes"));
        assert_eq!(
            m[1].content,
            "Translate this text into Bangla (Bengali).\n\n<text>\nThe rent is $1,200.\n</text>"
        );
        let m = translate("Chunk.", bn, Some((2, 3)));
        assert!(m[1].content.contains("This is part 2 of 3"));
        assert!(m[1].content.contains("translate only this part"));
    }

    #[test]
    fn translation_prefix_is_identical_across_requests_for_caching() {
        let es = language("es").unwrap();
        assert_eq!(
            translate("One.", es, None)[0],
            translate("Two.", es, None)[0]
        );
        assert_ne!(
            translate("One.", es, None)[0],
            translate("One.", language("hi").unwrap(), None)[0]
        );
    }

    #[test]
    fn word_translation_asks_for_the_sense_in_context() {
        let es = language("es").unwrap();
        let m = translate_word("bank", Some("We sat on the river bank."), es);
        assert!(m[0].content.contains("into Spanish."));
        assert!(m[0].content.contains("as it is used in that sentence"));
        assert!(m[0].content.contains("Do not translate the sentence"));
        assert_eq!(
            m[1].content,
            "Word: bank\nSentence: We sat on the river bank."
        );
        assert_eq!(translate_word("bank", None, es)[1].content, "Word: bank");
    }

    #[test]
    fn translation_budget_depends_on_the_target_script() {
        let text = "word ".repeat(100);
        let budget = |code: &str| max_tokens_for_translation(&text, language(code).unwrap());
        assert_eq!(budget("es"), 314);
        assert!(budget("bn") > budget("ar"));
        assert!(budget("ar") > budget("zh"));
        assert!(budget("zh") > budget("es"));
        assert_eq!(budget("bn"), 664);
        assert_eq!(max_tokens_for_translation("", language("es").unwrap()), 96);
        assert_eq!(
            max_tokens_for_translation(&"word ".repeat(10_000), language("bn").unwrap()),
            3072
        );
        assert!(
            max_tokens_for_word_translation(language("bn").unwrap())
                > max_tokens_for_word_translation(language("es").unwrap())
        );
    }

    #[test]
    fn chinese_source_is_measured_by_characters() {
        // 30 characters, no spaces: about 20 English-sized words.
        let zh = "租户应在每月第一天支付租金。".repeat(2) + "这是规定。!";
        assert_eq!(crate::text::word_count(&zh), 1);
        assert!(source_words(&zh) >= 18);
        assert_eq!(source_words("The rent is due."), 4);
    }

    #[test]
    fn token_budget_scales_with_length_and_is_capped() {
        assert_eq!(max_tokens_for_rewrite(0, Level::Plain), 96);
        assert!(
            max_tokens_for_rewrite(100, Level::Simpler)
                > max_tokens_for_rewrite(100, Level::Clearer)
        );
        assert_eq!(max_tokens_for_rewrite(100_000, Level::Plain), 3072);
    }
}
