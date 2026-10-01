//! The meaning guard: a deterministic check that a rewrite kept the facts a reader relies
//! on. Small models sometimes drop or change a number, or invent one. This compares the
//! numbers, acronyms and names in the source with the rewrite and reports what is missing
//! or new, which the card shows as a quiet "check this" note.
//!
//! It is deliberately forgiving about form: "15%" matches "15 percent", "1,200,000"
//! matches "1.2 million", "three" matches "3", and a date only needs its year.

use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", content = "text", rename_all = "snake_case")]
pub enum Fact {
    Number(String),
    Acronym(String),
    Name(String),
}

impl Fact {
    pub fn text(&self) -> &str {
        match self {
            Fact::Number(s) | Fact::Acronym(s) | Fact::Name(s) => s,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Report {
    /// In the source but not in the rewrite.
    pub missing: Vec<Fact>,
    /// Numbers in the rewrite that the source doesn't have.
    pub added: Vec<Fact>,
}

impl Report {
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.added.is_empty()
    }
}

pub fn check(source: &str, rewrite: &str) -> Report {
    let src_numbers = numbers(source);
    let out_numbers = numbers(rewrite);
    let out_values: Vec<f64> = out_numbers.iter().flat_map(|n| n.values.clone()).collect();
    let src_values: Vec<f64> = src_numbers.iter().flat_map(|n| n.values.clone()).collect();
    let out_lower = rewrite.to_lowercase();

    let mut missing = BTreeSet::new();
    for n in &src_numbers {
        if !n.required.iter().all(|v| contains_value(&out_values, *v)) {
            missing.insert(Fact::Number(n.text.clone()));
        }
    }
    for a in acronyms(source) {
        if !contains_word(rewrite, &a) {
            missing.insert(Fact::Acronym(a));
        }
    }
    for name in names(source) {
        let words: Vec<&str> = name
            .split_whitespace()
            .filter(|w| !CONNECTORS.contains(&w.to_lowercase().as_str()))
            .collect();
        if !words.iter().all(|w| out_lower.contains(&w.to_lowercase())) {
            missing.insert(Fact::Name(name));
        }
    }

    let mut added = BTreeSet::new();
    for n in &out_numbers {
        // Spelled-out small numbers ("two weeks") are too common in plain rewrites to flag.
        if n.is_word {
            continue;
        }
        if !n.values.iter().any(|v| contains_value(&src_values, *v)) {
            added.insert(Fact::Number(n.text.clone()));
        }
    }
    Report {
        missing: missing.into_iter().collect(),
        added: added.into_iter().collect(),
    }
}

fn contains_value(values: &[f64], v: f64) -> bool {
    values
        .iter()
        .any(|x| (x - v).abs() <= f64::EPSILON * v.abs().max(1.0) * 4.0)
}

fn contains_word(haystack: &str, word: &str) -> bool {
    haystack
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w == word)
}

#[derive(Debug, Clone, PartialEq)]
struct Number {
    text: String,
    /// Values this mention can match (a number with a multiplier word has both forms).
    values: Vec<f64>,
    /// Values the rewrite must contain for this mention to count as kept.
    required: Vec<f64>,
    is_word: bool,
}

const MULTIPLIERS: &[(&str, f64)] = &[
    ("thousand", 1e3),
    ("million", 1e6),
    ("millions", 1e6),
    ("billion", 1e9),
    ("billions", 1e9),
    ("trillion", 1e12),
    ("bn", 1e9),
    ("m", 1e6),
    ("k", 1e3),
    ("lakh", 1e5),
    ("lakhs", 1e5),
    ("crore", 1e7),
    ("crores", 1e7),
];

const SMALL: &[(&str, f64)] = &[
    ("zero", 0.0),
    ("one", 1.0),
    ("two", 2.0),
    ("three", 3.0),
    ("four", 4.0),
    ("five", 5.0),
    ("six", 6.0),
    ("seven", 7.0),
    ("eight", 8.0),
    ("nine", 9.0),
    ("ten", 10.0),
    ("eleven", 11.0),
    ("twelve", 12.0),
    ("thirteen", 13.0),
    ("fourteen", 14.0),
    ("fifteen", 15.0),
    ("sixteen", 16.0),
    ("seventeen", 17.0),
    ("eighteen", 18.0),
    ("nineteen", 19.0),
    ("twenty", 20.0),
    ("thirty", 30.0),
    ("forty", 40.0),
    ("fifty", 50.0),
    ("sixty", 60.0),
    ("seventy", 70.0),
    ("eighty", 80.0),
    ("ninety", 90.0),
    ("hundred", 100.0),
    ("thousand", 1000.0),
    ("million", 1e6),
    ("billion", 1e9),
    ("half", 0.5),
    ("first", 1.0),
    ("second", 2.0),
    ("third", 3.0),
    ("fourth", 4.0),
    ("fifth", 5.0),
    ("sixth", 6.0),
    ("seventh", 7.0),
    ("eighth", 8.0),
    ("ninth", 9.0),
    ("tenth", 10.0),
    ("twice", 2.0),
    ("double", 2.0),
    ("dozen", 12.0),
];

fn small_word(w: &str) -> Option<f64> {
    let w = w.to_lowercase();
    if let Some(&(_, v)) = SMALL.iter().find(|(s, _)| *s == w) {
        return Some(v);
    }
    // "twenty-five", "thirty-first"
    let (tens, units) = w.split_once('-')?;
    let t = SMALL.iter().find(|(s, _)| *s == tens)?.1;
    let u = SMALL.iter().find(|(s, _)| *s == units)?.1;
    ((20.0..=90.0).contains(&t) && (1.0..=9.0).contains(&u)).then_some(t + u)
}

/// Every number mentioned in `text`, in digits or (for small ones) words.
fn numbers(text: &str) -> Vec<Number> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let raw = tokens[i];
        let word = raw.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
        if let Some(v) = small_word(word) {
            // "two million": the multiplier applies to the word too.
            let mut v = v;
            let mut text = word.to_string();
            if let Some(next) = tokens.get(i + 1) {
                let next = next
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                if let Some(&(_, m)) = MULTIPLIERS.iter().find(|(s, _)| *s == next && s.len() > 2) {
                    v *= m;
                    text = format!("{text} {next}");
                    i += 1;
                }
            }
            out.push(Number {
                text,
                values: vec![v],
                required: vec![v],
                is_word: true,
            });
            i += 1;
            continue;
        }
        let mut consumed_next = false;
        for (start, end) in digit_runs(raw) {
            let run = &raw[start..end];
            let suffix = &raw[end..];
            let prefix = &raw[..start];
            // Dates: only the year has to survive ("12/03/2024" → "12 March 2024").
            if let Some(year) = date_year(run) {
                out.push(Number {
                    text: run.to_string(),
                    values: vec![year],
                    required: vec![year],
                    is_word: false,
                });
                continue;
            }
            // Times: 5:30 needs 5 and 30.
            if run.contains(':') {
                let parts: Vec<f64> = run.split(':').filter_map(|p| p.parse().ok()).collect();
                out.push(Number {
                    text: run.to_string(),
                    values: parts.clone(),
                    required: parts,
                    is_word: false,
                });
                continue;
            }
            let Some(v) = parse_number(run) else { continue };
            let mut values = vec![v];
            let mut text = format!(
                "{}{run}",
                prefix.trim_start_matches(|c: char| c.is_alphabetic())
            );
            // Multiplier attached ("5m", "2bn") or as the next word ("1.2 million").
            let attached = suffix
                .trim_end_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            let next = tokens.get(i + 1).map(|t| {
                t.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
            });
            if let Some(&(name, m)) = MULTIPLIERS.iter().find(|(s, _)| *s == attached) {
                values.push(v * m);
                text.push_str(name);
            } else if let Some(next) = next
                && let Some(&(name, m)) =
                    MULTIPLIERS.iter().find(|(s, _)| *s == next && s.len() > 2)
            {
                values.push(v * m);
                text = format!("{text} {name}");
                consumed_next = true;
            }
            if suffix.starts_with('%') {
                text.push('%');
            }
            // The mention is kept if any form of it appears.
            let required = vec![*values.last().unwrap_or(&v)];
            out.push(Number {
                text,
                values: values.clone(),
                required,
                is_word: false,
            });
            // Keep "1.2 million" findable as 1.2 as well as 1,200,000.
            if values.len() > 1 {
                let last = out.last_mut().expect("just pushed");
                last.required = vec![values[1]];
            }
        }
        i += if consumed_next { 2 } else { 1 };
    }
    out
}

/// Runs like "1,200.50", "12/03/2024", "5:30", "3.5" inside a token.
fn digit_runs(token: &str) -> Vec<(usize, usize)> {
    let b = token.as_bytes();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let start = i;
            let mut end = i + 1;
            while end < b.len() {
                let separator_then_digit = matches!(b[end], b',' | b'.' | b'/' | b':' | b'-')
                    && end + 1 < b.len()
                    && b[end + 1].is_ascii_digit();
                if b[end].is_ascii_digit() || separator_then_digit {
                    end += 1;
                } else {
                    break;
                }
            }
            runs.push((start, end));
            i = end;
        } else {
            i += 1;
        }
    }
    runs
}

fn date_year(run: &str) -> Option<f64> {
    let sep = ['/', '-', '.']
        .into_iter()
        .find(|&s| run.matches(s).count() == 2)?;
    let parts: Vec<&str> = run.split(sep).collect();
    if parts.len() != 3
        || !parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    // 2024-03-12 or 12/03/2024 (or 12/03/24).
    let year = if parts[0].len() == 4 {
        parts[0]
    } else {
        parts[2]
    };
    let y: f64 = year.parse().ok()?;
    Some(if year.len() == 2 { 2000.0 + y } else { y })
}

fn parse_number(run: &str) -> Option<f64> {
    // Ranges like "10-20" are split by the caller's digit runs only when they are dates;
    // treat a single dash run as two numbers' first value.
    let run = run.split('-').next()?;
    let grouped = {
        // 1,200 or 1,200,000.5 → remove thousands separators.
        let int_part = run.split('.').next().unwrap_or(run);
        let groups: Vec<&str> = int_part.split(',').collect();
        groups.len() > 1 && groups[0].len() <= 3 && groups[1..].iter().all(|g| g.len() == 3)
    };
    let cleaned: String = if grouped {
        run.replace(',', "")
    } else if run.contains(',') && !run.contains('.') {
        // "3,5" is a decimal comma.
        run.replace(',', ".")
    } else {
        run.to_string()
    };
    cleaned.parse().ok()
}

/// Words that are all capitals (2+ letters), like GDPR, NASA, COVID-19.
fn acronyms(text: &str) -> BTreeSet<String> {
    let words: Vec<&str> = text
        .split(|c: char| {
            c.is_whitespace() || matches!(c, '(' | ')' | ',' | ';' | ':' | '"' | '“' | '”' | '/')
        })
        .filter(|w| !w.is_empty())
        .collect();
    let caps: Vec<&str> = words
        .iter()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| {
            let letters: Vec<char> = w.chars().filter(|c| c.is_alphabetic()).collect();
            letters.len() >= 2
                && letters.iter().all(|c| c.is_uppercase())
                && w.chars().next().is_some_and(char::is_alphabetic)
                && !ROMAN.contains(w)
                && !["OK", "AM", "PM", "TV", "ID"].contains(w)
        })
        .collect();
    // Shouty text (headings, legal caps) isn't full of acronyms.
    if caps.len() * 10 > words.len() * 3 && words.len() > 5 {
        return BTreeSet::new();
    }
    caps.into_iter().map(String::from).collect()
}

const ROMAN: &[&str] = &[
    "II", "III", "IV", "VI", "VII", "VIII", "IX", "XI", "XII", "XX",
];

/// Joins capitalised words inside a name: "Bank of England", "Court of Justice".
const CONNECTORS: &[&str] = &[
    "of", "the", "and", "for", "de", "la", "van", "von", "da", "del", "&",
];

/// Words often capitalised mid-sentence that aren't names worth checking.
const NOT_NAMES: &[&str] = &[
    "I", "I'm", "I've", "I'd", "I'll", "Mr", "Mrs", "Ms", "Dr", "Prof", "Sir", "Madam", "The",
    "This", "That", "These", "Those", "A", "An", "It", "We", "You", "They", "He", "She", "Section",
    "Article", "Clause", "Chapter", "Table", "Figure", "Fig", "Page", "Part", "Schedule",
    "Appendix", "Annex", "Note",
];

/// Proper names: capitalised words that don't start a sentence, joined into phrases.
fn names(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for sentence in crate::text::sentences(text) {
        let tokens: Vec<&str> = sentence.split_whitespace().collect();
        let mut i = 0;
        while i < tokens.len() {
            let w = clean_token(tokens[i]);
            // The first word of a sentence is capitalised by grammar, not because it's a
            // name; names starting a sentence are still caught when they recur.
            if i == 0 || !is_capitalised(w) || NOT_NAMES.contains(&w) {
                i += 1;
                continue;
            }
            let mut parts = vec![w.to_string()];
            let mut j = i + 1;
            while j < tokens.len() {
                let next = clean_token(tokens[j]);
                // Stop at punctuation that ends a phrase.
                if tokens[j - 1].ends_with([',', ';', ':', ')', '.']) {
                    break;
                }
                let name_word = is_capitalised(next) && !NOT_NAMES.contains(&next);
                let connector = CONNECTORS.contains(&next)
                    && tokens
                        .get(j + 1)
                        .is_some_and(|t| is_capitalised(clean_token(t)))
                    && !tokens[j].ends_with([',', ';', ':', '.']);
                if name_word || connector {
                    parts.push(next.to_string());
                    j += 1;
                } else {
                    break;
                }
            }
            if parts.iter().any(|p| p.chars().count() >= 2) {
                out.insert(parts.join(" "));
            }
            i = j;
        }
    }
    // Acronyms are checked on their own.
    out.retain(|n| {
        !n.chars()
            .filter(|c| c.is_alphabetic())
            .all(char::is_uppercase)
    });
    out
}

fn clean_token(t: &str) -> &str {
    t.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '&')
        .trim_end_matches("'s")
        .trim_end_matches("’s")
}

fn is_capitalised(w: &str) -> bool {
    let mut chars = w.chars();
    chars.next().is_some_and(|c| c.is_uppercase())
        && chars.clone().count() >= 1
        && chars.any(|c| c.is_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing(src: &str, out: &str) -> Vec<String> {
        check(src, out)
            .missing
            .iter()
            .map(|f| f.text().to_string())
            .collect()
    }
    fn added(src: &str, out: &str) -> Vec<String> {
        check(src, out)
            .added
            .iter()
            .map(|f| f.text().to_string())
            .collect()
    }

    #[test]
    fn identical_text_is_clean() {
        let s =
            "The World Health Organization (WHO) reported 1,200 cases in 2023, up 15% from March.";
        assert!(check(s, s).is_clean());
    }

    #[test]
    fn numbers_can_change_form() {
        let src =
            "Revenue grew 15% to $1,200,000 over three years, about 2.5 times the 2019 level.";
        let out = "Over 3 years, revenue went up by 15 percent to $1.2 million. That is about 2.5 times what it was in 2019.";
        assert_eq!(missing(src, out), Vec::<String>::new());
        assert_eq!(added(src, out), Vec::<String>::new());
    }

    #[test]
    fn dropped_and_changed_numbers_are_reported() {
        let src = "The fee is 2.5% of the loan and must be paid within 30 days.";
        let out = "You pay a fee of 3% of the loan soon.";
        assert_eq!(missing(src, out), vec!["2.5%", "30"]);
        assert_eq!(added(src, out), vec!["3%"]);
    }

    #[test]
    fn invented_numbers_are_reported_but_spelled_out_ones_are_not() {
        let src = "Payment is due within a fortnight.";
        assert_eq!(added(src, "Pay within two weeks."), Vec::<String>::new());
        assert_eq!(
            added(src, "Pay within 14 days or a 5% fine applies."),
            vec!["14", "5%"]
        );
    }

    #[test]
    fn dates_need_only_their_year() {
        let src = "The contract ends on 31/12/2026.";
        assert!(missing(src, "The contract ends on 31 December 2026.").is_empty());
        assert_eq!(
            missing(src, "The contract ends at the end of the year."),
            vec!["31/12/2026"]
        );
        assert!(missing("Filed 2024-03-12.", "Filed on 12 March 2024.").is_empty());
    }

    #[test]
    fn multipliers_and_lakh_crore() {
        assert!(missing("It cost 5m dollars.", "It cost 5 million dollars.").is_empty());
        assert!(missing("It cost 5 million dollars.", "It cost $5,000,000.").is_empty());
        assert!(
            missing(
                "The budget is 2 crore taka.",
                "The budget is 20,000,000 taka."
            )
            .is_empty()
        );
        assert!(missing("two million people", "2,000,000 people").is_empty());
    }

    #[test]
    fn word_numbers() {
        assert!(missing("It took twenty-five minutes.", "It took 25 minutes.").is_empty());
        assert!(missing("It took 25 minutes.", "It took twenty-five minutes.").is_empty());
        assert!(missing("He came first.", "He came 1st.").is_empty());
    }

    #[test]
    fn acronyms_must_stay() {
        let src = "Under the GDPR, firms must appoint a DPO.";
        assert!(missing(src, "Under the General Data Protection Regulation (GDPR), companies must name a Data Protection Officer (DPO).").is_empty());
        assert_eq!(
            missing(
                src,
                "Under the data law, companies must name a privacy officer."
            ),
            vec!["DPO", "GDPR"]
        );
    }

    #[test]
    fn shouting_is_not_acronyms() {
        let src = "THE SOFTWARE IS PROVIDED AS IS, WITHOUT WARRANTY OF ANY KIND.";
        assert!(missing(src, "The software comes with no promises.").is_empty());
    }

    #[test]
    fn names_must_stay_but_sentence_starts_are_ignored() {
        let src =
            "Yesterday the Bank of England raised rates. Analysts at Goldman Sachs expected it.";
        assert!(missing(src, "The Bank of England put rates up yesterday. Experts at Goldman Sachs saw it coming.").is_empty());
        assert_eq!(
            missing(
                src,
                "The central bank put rates up yesterday. Experts saw it coming."
            ),
            vec!["Bank of England", "Goldman Sachs"]
        );
    }

    #[test]
    fn names_are_case_insensitive_and_possessives_are_fine() {
        let src = "We met with Microsoft's lawyers in Seattle.";
        assert!(missing(src, "We met lawyers from Microsoft in seattle.").is_empty());
    }

    #[test]
    fn section_words_are_not_names() {
        let src = "See Section 4 and Article 12 of the Treaty.";
        // "Treaty" alone at the end is a name; "Section" and "Article" are not.
        let m = missing(src, "Look at parts 4 and 12 of the agreement.");
        assert_eq!(m, vec!["Treaty"]);
    }

    #[test]
    fn times_and_ranges() {
        assert!(missing("Open 9:30 to 17:00.", "Open from 9:30 until 17:00.").is_empty());
        assert_eq!(
            missing("Open 9:30 to 17:00.", "Open in the morning."),
            vec!["17:00", "9:30"]
        );
    }

    #[test]
    fn decimal_comma() {
        assert!(missing("Inflation was 3,5 percent.", "Prices rose 3.5%.").is_empty());
    }
}
