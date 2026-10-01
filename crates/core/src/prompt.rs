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
    let user = match context {
        Some(c) if !c.trim().is_empty() && c.trim() != word.trim() => {
            format!("Word: {word}\nSentence: {}", c.trim())
        }
        _ => format!("Word: {word}"),
    };
    vec![msg(Role::System, system), msg(Role::User, user)]
}

/// The most tokens a rewrite of `words` source words may take.
pub fn max_tokens_for_rewrite(words: usize, level: Level) -> u32 {
    ((words as f32 * level.tokens_per_source_word()) as u32 + 64).clamp(96, 3072)
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
    fn token_budget_scales_with_length_and_is_capped() {
        assert_eq!(max_tokens_for_rewrite(0, Level::Plain), 96);
        assert!(
            max_tokens_for_rewrite(100, Level::Simpler)
                > max_tokens_for_rewrite(100, Level::Clearer)
        );
        assert_eq!(max_tokens_for_rewrite(100_000, Level::Plain), 3072);
    }
}
