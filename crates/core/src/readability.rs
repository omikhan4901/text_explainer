//! Reading grade of English text (Flesch–Kincaid grade level), shown on the card as
//! "Grade 15 → 8" so people can see what the rewrite did.

use crate::text::{sentences, words};

/// Below this the formula is noise, so no grade is shown.
const MIN_WORDS: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Readability {
    /// US school grade, clamped to 1..=18 ("18" reads as "college graduate and above").
    pub grade: f32,
    pub words: usize,
    pub sentences: usize,
    /// Average words per sentence.
    pub words_per_sentence: f32,
}

/// The grade of `text`, or `None` when it is too short or not mostly Latin-script
/// (the formula is for English).
pub fn grade(text: &str) -> Option<Readability> {
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    let latin = text
        .chars()
        .filter(|c| c.is_ascii_alphabetic() || ('\u{00C0}'..='\u{024F}').contains(c))
        .count();
    if letters == 0 || latin * 10 < letters * 9 {
        return None;
    }
    let word_list: Vec<String> = words(text)
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphabetic() || *c == '\'')
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    let n_words = word_list.len();
    if n_words < MIN_WORDS {
        return None;
    }
    let n_sentences = sentences(text)
        .into_iter()
        .filter(|s| words(s).next().is_some())
        .count()
        .max(1);
    let syllables: usize = word_list.iter().map(|w| syllables(w)).sum();
    let wps = n_words as f32 / n_sentences as f32;
    let spw = syllables as f32 / n_words as f32;
    let fk = 0.39 * wps + 11.8 * spw - 15.59;
    Some(Readability {
        grade: fk.clamp(1.0, 18.0),
        words: n_words,
        sentences: n_sentences,
        words_per_sentence: wps,
    })
}

/// English syllable estimate: vowel groups, minus silent endings. Accurate to about ±1
/// on most words, which is what the formula expects.
pub fn syllables(word: &str) -> usize {
    let w: String = word
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect();
    if w.is_empty() {
        return 0;
    }
    if w.len() <= 3 {
        return 1;
    }
    let mut w = w.as_str();
    // Silent endings: "make", "named", "boxes" (but not "table", "wanted", "cases").
    if let Some(stem) = w.strip_suffix("es").or_else(|| w.strip_suffix("ed")) {
        let ending_sounded = stem.ends_with(['t', 'd']) && w.ends_with("ed")
            || stem.ends_with(['s', 'x', 'z', 'c', 'g']) && w.ends_with("es")
            || stem.ends_with("sh")
            || stem.ends_with("ch");
        if !ending_sounded {
            w = stem;
        }
    } else if w.ends_with('e') && !w.ends_with("le") && !w.ends_with("ee") {
        w = &w[..w.len() - 1];
    }
    let mut count = 0;
    let mut prev_vowel = false;
    for c in w.chars() {
        let vowel = matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
        if vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = vowel;
    }
    // "ia" and "io" are usually two syllables ("radio", "media"), except in the
    // "-tion", "-sion", "-cial", "-cian", "-xious" family.
    let b = w.as_bytes();
    count += (1..b.len().saturating_sub(1))
        .filter(|&i| b[i] == b'i' && matches!(b[i + 1], b'a' | b'o'))
        .filter(|&i| !matches!(b[i - 1], b't' | b's' | b'c' | b'x' | b'g'))
        .count();
    count.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_estimates() {
        for (w, n) in [
            ("cat", 1),
            ("the", 1),
            ("make", 1),
            ("named", 1),
            ("wanted", 2),
            ("boxes", 2),
            ("table", 2),
            ("water", 2),
            ("happy", 2),
            ("beautiful", 3),
            ("radio", 3),
            ("information", 4),
            ("university", 5),
            ("responsibility", 6),
            ("free", 1),
        ] {
            assert_eq!(syllables(w), n, "{w}");
        }
    }

    #[test]
    fn hard_text_scores_higher_than_plain_text() {
        let hard = "Notwithstanding the aforementioned considerations, the implementation of comprehensive \
            regulatory frameworks necessitates substantial institutional coordination, particularly \
            regarding the harmonisation of administrative procedures across heterogeneous jurisdictions \
            with considerably divergent constitutional traditions.";
        let plain = "The rules are new. Many offices must work together to use them. \
            Each country does things its own way. So the offices need to agree on one way to work. \
            This will take time and care.";
        let h = grade(hard).unwrap();
        let p = grade(plain).unwrap();
        assert!(h.grade >= 16.0, "hard text graded {}", h.grade);
        assert!(p.grade <= 6.0, "plain text graded {}", p.grade);
        assert_eq!(p.sentences, 5);
    }

    #[test]
    fn short_or_non_latin_text_has_no_grade() {
        assert!(grade("Too short to grade.").is_none());
        assert!(grade(&"আমি বাংলায় গান গাই এবং আমি বাংলায় কথা বলি। ".repeat(10)).is_none());
        assert!(grade("").is_none());
    }

    #[test]
    fn grade_is_clamped() {
        let one_long_sentence = format!("{}.", "incomprehensibility ".repeat(60).trim());
        assert_eq!(grade(&one_long_sentence).unwrap().grade, 18.0);
        let tiny = "I am. ".repeat(30);
        assert_eq!(grade(&tiny).unwrap().grade, 1.0);
    }
}
