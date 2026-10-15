//! The meaning guard: a deterministic check that a rewrite kept the facts a reader relies
//! on. Small models sometimes drop or change a number, or invent one. This compares the
//! numbers, acronyms and names in the source with the rewrite and reports what is missing
//! or new, which the card shows as a quiet "check this" note.
//!
//! It is deliberately forgiving about form: "15%" matches "15 percent", "1,200,000"
//! matches "1.2 million", "three" matches "3", and a date only needs its year.

use serde::Serialize;
use std::collections::BTreeSet;

use crate::language::Language;

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
    check_with(source, rewrite, &|_| false)
}

/// The meaning guard for a translation into `target`. Numbers and dates must survive,
/// whatever digits and separators the target uses: Bangla, Devanagari, Arabic-Indic and
/// full-width digits count as the same digits, "1.200,50" (Spanish) and "1,20,000"
/// (Indian grouping) are read too, and "12 লাখ" or "120万" match "1.2 million". Names and
/// acronyms are checked only when both texts are written in the same script: across
/// scripts they are transliterated, and checking them would only raise false alarms.
/// Spelled-out number words in the source are required only in an English translation
/// (the guard reads number words in English only).
pub fn check_translation(
    source: &str,
    translation: &str,
    target: &Language,
    is_common_word: &dyn Fn(&str) -> bool,
) -> Report {
    // Names aren't compared: a translation rightly writes them the target's way
    // ("Geneva" becomes "Ginebra", "WHO" becomes "OMS"), even in the same script.
    let rules = Rules {
        translation: true,
        word_numbers_required: target.code == "en",
        terms: false,
    };
    compare(
        &normalize_numerals(source),
        &normalize_numerals(translation),
        rules,
        is_common_word,
    )
}

/// What a comparison checks and how it reads numbers.
#[derive(Debug, Clone, Copy)]
struct Rules {
    /// The two texts are in different languages: read numbers in any locale's notation and
    /// multiplier words in several languages.
    translation: bool,
    /// Small numbers written as words ("thirty days") must survive.
    word_numbers_required: bool,
    /// Names and acronyms must survive.
    terms: bool,
}

const REWRITE: Rules = Rules {
    translation: false,
    word_numbers_required: true,
    terms: true,
};

/// Single capitalised words in `source` that the check would treat as names. The app
/// looks them up in the dictionary and passes the common ones (contract terms like
/// "Lessee" or "Agreement") back through `check_with` so they aren't flagged.
pub fn single_word_names(source: &str) -> Vec<String> {
    names(source)
        .into_iter()
        .filter(|n| !n.contains(' '))
        .collect()
}

/// Like `check`, skipping single-word "names" for which `is_common_word` (given the
/// lowercase word) is true.
pub fn check_with(source: &str, rewrite: &str, is_common_word: &dyn Fn(&str) -> bool) -> Report {
    compare(source, rewrite, REWRITE, is_common_word)
}

fn compare(
    source: &str,
    rewrite: &str,
    rules: Rules,
    is_common_word: &dyn Fn(&str) -> bool,
) -> Report {
    let src_numbers = numbers(source, rules);
    let out_numbers = numbers(rewrite, rules);
    let out_values: Vec<f64> = out_numbers.iter().flat_map(|n| n.values.clone()).collect();
    let src_values: Vec<f64> = src_numbers.iter().flat_map(|n| n.values.clone()).collect();
    let out_lower = rewrite.to_lowercase();

    let mut missing = BTreeSet::new();
    for n in &src_numbers {
        let kept = n
            .required
            .iter()
            .all(|any| any.iter().any(|v| contains_value(&out_values, *v)));
        if !kept {
            missing.insert(Fact::Number(n.text.clone()));
        }
    }
    let terms = if rules.terms {
        (acronyms(source), names(source))
    } else {
        Default::default()
    };
    for a in terms.0 {
        if !contains_word(rewrite, &a) {
            missing.insert(Fact::Acronym(a));
        }
    }
    for name in terms.1 {
        let words: Vec<&str> = name
            .split_whitespace()
            .filter(|w| !CONNECTORS.contains(&w.to_lowercase().as_str()))
            .collect();
        if words.len() == 1 && is_common_word(&words[0].to_lowercase()) {
            continue;
        }
        let found = |w: &&str| out_lower.contains(&w.to_lowercase());
        let all = words.iter().all(found);
        // "United Kingdom" → "UK", "World Health Organization" → "WHO".
        let initials: String = words.iter().filter_map(|w| w.chars().next()).collect();
        let by_initials = words.len() >= 2 && contains_word(rewrite, &initials);
        // A long name may be shortened to its distinctive last word: "the University of
        // Cambridge" → "Cambridge" (but "the Bank of England" → "the bank" is not enough).
        let last = words.last().copied().unwrap_or_default();
        let by_last = words.len() >= 2 && found(&last) && !is_common_word(&last.to_lowercase());
        let kept = all || by_last;
        if !(kept || by_initials) {
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
    /// What the rewrite must contain for this mention to count as kept: one value from
    /// each group (a time needs its hour and its minutes; an ambiguous "1.200" in a
    /// translation needs either reading).
    required: Vec<Vec<f64>>,
    is_word: bool,
}

/// Multiplier words in the languages AI Translate writes, as the next word ("1,2
/// millones", "12 লাখ", "5 मिलियन"). English ones are in `MULTIPLIERS`. Spanish
/// "billón" (a million million) is left out: it is a false friend of "billion".
const TRANSLATED_MULTIPLIERS: &[(&str, f64)] = &[
    ("mil", 1e3),
    ("millón", 1e6),
    ("millones", 1e6),
    ("milliers", 1e3),
    ("milliards", 1e9),
    ("milliard", 1e9),
    ("milhão", 1e6),
    ("milhões", 1e6),
    ("tausend", 1e3),
    ("million", 1e6),
    ("millionen", 1e6),
    ("milliarde", 1e9),
    ("milliarden", 1e9),
    ("тысяч", 1e3),
    ("тысячи", 1e3),
    ("млн", 1e6),
    ("миллиона", 1e6),
    ("миллионов", 1e6),
    ("млрд", 1e9),
    ("হাজার", 1e3),
    ("লাখ", 1e5),
    ("লক্ষ", 1e5),
    ("কোটি", 1e7),
    // য় and ड़ come both precomposed and as letter plus nukta.
    ("মিলি\u{09DF}ন", 1e6),
    ("মিলি\u{09AF}\u{09BC}ন", 1e6),
    ("বিলি\u{09DF}ন", 1e9),
    ("বিলি\u{09AF}\u{09BC}ন", 1e9),
    ("ह\u{095B}ार", 1e3),
    ("हज\u{093C}ार", 1e3),
    ("हजार", 1e3),
    ("लाख", 1e5),
    ("करो\u{095C}", 1e7),
    ("करो\u{0921}\u{093C}", 1e7),
    ("मिलियन", 1e6),
    ("बिलियन", 1e9),
    ("अरब", 1e9),
    ("ألف", 1e3),
    ("آلاف", 1e3),
    ("مليون", 1e6),
    ("ملايين", 1e6),
    ("مليار", 1e9),
];

/// Multipliers written straight after the digits ("120万", "1.2亿"), longest first.
const ATTACHED_MULTIPLIERS: &[(&str, f64)] = &[
    ("千万", 1e7),
    ("百万", 1e6),
    ("万", 1e4),
    ("萬", 1e4),
    ("亿", 1e8),
    ("億", 1e8),
    ("千", 1e3),
    ("만", 1e4),
    ("억", 1e8),
];

/// The zero of every decimal digit set the guard reads (Arabic-Indic, Eastern
/// Arabic-Indic, NKo, Devanagari, Bengali, Gurmukhi, Gujarati, Oriya, Tamil, Telugu,
/// Kannada, Malayalam, Sinhala, Thai, Lao, Tibetan, Myanmar, Khmer, Mongolian, full-width).
const DIGIT_ZEROS: &[u32] = &[
    0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66, 0x0BE6, 0x0C66, 0x0CE6, 0x0D66,
    0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040, 0x17E0, 0x1810, 0xFF10,
];

fn ascii_digit(c: char) -> Option<char> {
    let cp = c as u32;
    DIGIT_ZEROS
        .iter()
        .find(|&&zero| (zero..zero + 10).contains(&cp))
        .and_then(|&zero| char::from_digit(cp - zero, 10))
}

/// Writes every number with ASCII digits and separators, so numbers in any script compare:
/// native digits become 0-9; the Arabic decimal and thousands separators and percent sign,
/// and full-width ones, become `.` `,` `%`; and digit groups separated by a space, a
/// no-break or thin space or an apostrophe ("1 200", "1'200") are joined with a comma.
pub fn normalize_numerals(text: &str) -> String {
    let chars: Vec<char> = text
        .chars()
        .map(|c| {
            ascii_digit(c).unwrap_or(match c {
                '\u{066B}' | '．' => '.',
                '\u{066C}' => ',',
                '\u{066A}' | '％' => '%',
                '：' => ':',
                '／' => '/',
                other => other,
            })
        })
        .collect();
    let mut out = String::with_capacity(text.len());
    // Digits since the last separator, or None when not inside a number.
    let mut group: Option<usize> = None;
    for (i, &c) in chars.iter().enumerate() {
        let joins = matches!(
            c,
            ' ' | '\u{00A0}' | '\u{202F}' | '\u{2009}' | '\'' | '’' | '，'
        );
        let three_after = chars
            .get(i + 1..i + 4)
            .is_some_and(|d| d.iter().all(char::is_ascii_digit))
            && !chars.get(i + 4).is_some_and(char::is_ascii_digit);
        if joins && group.is_some_and(|n| (1..=3).contains(&n)) && three_after {
            out.push(',');
            group = Some(0);
            continue;
        }
        if c.is_ascii_digit() {
            group = Some(group.map_or(1, |n| n + 1));
        } else if c == ',' && group.is_some() {
            group = Some(0);
        } else {
            group = None;
        }
        out.push(c);
    }
    out
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

const IDIOMATIC: &[&str] = &[
    "zero", "one", "first", "second", "third", "half", "double", "twice", "dozen",
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
fn numbers(text: &str, rules: Rules) -> Vec<Number> {
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
            // "first request", "one of", "scales to zero", "half the time": words like
            // these are usually idiom, not a quantity the reader needs.
            let idiom = !text.contains(' ') && IDIOMATIC.contains(&text.to_lowercase().as_str());
            let required = !idiom && rules.word_numbers_required;
            out.push(Number {
                text,
                values: vec![v],
                required: if required { vec![vec![v]] } else { vec![] },
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
            // Dates: only the year has to survive ("12/03/2024" → "12 March 2024"), but
            // the day and month may appear on their own.
            if let Some((year, parts)) = date_parts(run) {
                out.push(Number {
                    text: run.to_string(),
                    values: parts,
                    required: vec![vec![year]],
                    is_word: false,
                });
                continue;
            }
            // Times: 5:30 needs 5 and 30.
            if run.contains(':') {
                let parts: Vec<f64> = run.split(':').filter_map(|p| p.parse().ok()).collect();
                out.push(Number {
                    text: run.to_string(),
                    required: parts.iter().map(|&p| vec![p]).collect(),
                    values: parts,
                    is_word: false,
                });
                continue;
            }
            // A translation may write "1.200" for 1,200: every reading counts.
            let readings = if rules.translation {
                parse_any(run)
            } else {
                parse_number(run).into_iter().collect()
            };
            let Some(&v) = readings.first() else { continue };
            // Currency signs and the like just before the digits ("US$5" → "$5").
            let lead = &prefix[prefix
                .trim_end_matches(|c: char| !c.is_alphanumeric())
                .len()..];
            let mut text = format!("{lead}{run}");
            // Multiplier attached ("5m", "2bn", "120万") or as the next word ("1.2 million").
            let attached = suffix
                .trim_end_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            let next = tokens.get(i + 1).map(|t| word_of(t).to_lowercase());
            let next_word = |list: &[(&'static str, f64)]| {
                let next = next.as_deref()?;
                list.iter()
                    .find(|(s, _)| *s == next && s.len() > 2)
                    .copied()
            };
            let multiplier =
                if let Some(&(name, m)) = MULTIPLIERS.iter().find(|(s, _)| *s == attached) {
                    text.push_str(name);
                    Some(m)
                } else if let Some((name, m)) = rules
                    .translation
                    .then(|| {
                        ATTACHED_MULTIPLIERS
                            .iter()
                            .find(|(s, _)| suffix.starts_with(s))
                    })
                    .flatten()
                {
                    text.push_str(name);
                    Some(*m)
                } else if let Some((name, m)) = next_word(MULTIPLIERS).or_else(|| {
                    rules
                        .translation
                        .then(|| next_word(TRANSLATED_MULTIPLIERS))
                        .flatten()
                }) {
                    text = format!("{text} {name}");
                    consumed_next = true;
                    Some(m)
                } else {
                    None
                };
            if suffix.starts_with('%') {
                text.push('%');
            }
            let mut values = readings.clone();
            let required = match multiplier {
                Some(m) => {
                    values.extend(readings.iter().map(|r| r * m));
                    if rules.translation {
                        // Multiplier words differ between languages ("1.2 million" is
                        // "12 লাখ"): any reading or its multiple counts.
                        vec![values.clone()]
                    } else {
                        // Kept as "1.2 million" or "1,200,000", not as a bare 1.2.
                        vec![vec![v * m]]
                    }
                }
                None => vec![readings],
            };
            out.push(Number {
                text,
                values,
                required,
                is_word: false,
            });
        }
        i += if consumed_next { 2 } else { 1 };
    }
    out
}

/// A token without the punctuation around it. Indic vowel signs and nukta are part of the
/// word (some aren't "alphabetic" to Rust); the danda is punctuation.
fn word_of(token: &str) -> &str {
    token.trim_matches(|c: char| {
        let indic = (0x0900..=0x0DFF).contains(&(c as u32)) && !matches!(c, '।' | '॥');
        !(c.is_alphanumeric() || indic)
    })
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

/// A date's year, and all of its parts.
fn date_parts(run: &str) -> Option<(f64, Vec<f64>)> {
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
    let y = if year.len() == 2 { 2000.0 + y } else { y };
    let mut values: Vec<f64> = parts.iter().filter_map(|p| p.parse().ok()).collect();
    values.push(y);
    Some((y, values))
}

fn parse_number(run: &str) -> Option<f64> {
    // Ranges like "10-20" are split by the caller's digit runs only when they are dates;
    // treat a single dash run as two numbers' first value.
    let run = run.split('-').next()?;
    let grouped = {
        // 1,200 or 1,200,000.5 → remove thousands separators. Indian grouping too:
        // 1,20,000 (a lakh and twenty thousand).
        let int_part = run.split('.').next().unwrap_or(run);
        let groups: Vec<&str> = int_part.split(',').collect();
        let western =
            groups.len() > 1 && groups[0].len() <= 3 && groups[1..].iter().all(|g| g.len() == 3);
        let indian = groups.len() > 2
            && groups[0].len() <= 2
            && groups[1..groups.len() - 1].iter().all(|g| g.len() == 2)
            && groups.last().is_some_and(|g| g.len() == 3);
        western || indian
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

/// Every plausible reading of a number written in some language's notation: English
/// "1,200.5", continental "1.200,5", and a lone comma as either ("3,500" is 3500 in
/// English, 3.5 in Spanish). The usual English reading comes first.
fn parse_any(run: &str) -> Vec<f64> {
    let Some(run) = run.split('-').next() else {
        return vec![];
    };
    let mut readings: Vec<f64> = parse_number(run).into_iter().collect();
    // Continental: dots group thousands, a comma marks decimals.
    let (int_part, decimals) = match run.split_once(',') {
        Some((i, d)) if !d.contains([',', '.']) => (i, Some(d)),
        Some(_) => ("", None),
        None => (run, None),
    };
    let groups: Vec<&str> = int_part.split('.').collect();
    if groups.len() > 1 && groups[0].len() <= 3 && groups[1..].iter().all(|g| g.len() == 3) {
        let whole = groups.concat();
        let text = match decimals {
            Some(d) => format!("{whole}.{d}"),
            None => whole,
        };
        readings.extend(text.parse::<f64>().ok());
    }
    if run.matches(',').count() == 1 && !run.contains('.') {
        readings.extend(run.replace(',', ".").parse::<f64>().ok());
    }
    let mut unique: Vec<f64> = Vec::with_capacity(readings.len());
    for r in readings {
        if !contains_value(&unique, r) {
            unique.push(r);
        }
    }
    unique
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

/// Words that may join the parts of one name ("Bank of England"). Not "the" or "and":
/// "this Agreement and the Lessee" is two terms, not one name.
const JOINERS: &[&str] = &["of", "for", "de", "la", "van", "von", "da", "del", "&"];

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
                let connector = JOINERS.contains(&next)
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
    fn shortened_names_and_initials_count_as_kept() {
        let src = "Applicants who have lived in the United Kingdom for 12 months may apply.";
        assert!(
            missing(
                src,
                "People who have lived in the UK for 12 months may apply."
            )
            .is_empty()
        );
        let src = "Researchers at the University of Cambridge reported it in 2025.";
        assert!(missing(src, "Cambridge researchers reported it in 2025.").is_empty());
        assert_eq!(
            missing(src, "Researchers reported it in 2025."),
            vec!["University of Cambridge"]
        );
    }

    #[test]
    fn idiomatic_number_words_are_not_required() {
        let src = "The service scales to zero, and the first request after that is slow.";
        assert!(
            missing(
                src,
                "The service stops when idle, and the next request is slow."
            )
            .is_empty()
        );
        // Real quantities in words still are.
        assert_eq!(
            missing("Pay within thirty days.", "Pay soon."),
            vec!["thirty"]
        );
    }

    #[test]
    fn common_words_capitalised_as_defined_terms_can_be_skipped() {
        let src = "Under this Agreement the Lessee pays Microsoft monthly.";
        let out = "The tenant pays monthly.";
        let common = |w: &str| matches!(w, "agreement" | "lessee");
        let m: Vec<String> = check_with(src, out, &common)
            .missing
            .iter()
            .map(|f| f.text().to_string())
            .collect();
        assert_eq!(m, vec!["Microsoft"]);
        let mut singles = single_word_names(src);
        singles.sort();
        assert_eq!(singles, vec!["Agreement", "Lessee", "Microsoft"]);
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

    #[test]
    fn a_dates_day_and_month_are_not_invented() {
        assert!(added("Filed 2024-03-12.", "Filed on 12 March 2024.").is_empty());
        assert!(added("Due 31/12/2026.", "Due on the 31st of December, 2026.").is_empty());
    }

    #[test]
    fn indian_grouping_is_read() {
        assert!(missing("It costs 120,000 taka.", "It costs 1,20,000 taka.").is_empty());
        assert!(missing("The fund is 12,34,56,789.", "The fund is 123456789.").is_empty());
    }

    // Translations.

    fn lang(code: &str) -> &'static Language {
        crate::language::language(code).unwrap()
    }
    fn translated(src: &str, out: &str, to: &str) -> Report {
        check_translation(src, out, lang(to), &|_| false)
    }
    fn texts(facts: &[Fact]) -> Vec<&str> {
        facts.iter().map(Fact::text).collect()
    }

    #[test]
    fn numerals_in_every_script_become_ascii() {
        assert_eq!(normalize_numerals("৳১,২০০ ও ৫%"), "৳1,200 ও 5%");
        assert_eq!(
            normalize_numerals("१२ लाख, ०५/०३/२०२६"),
            "12 लाख, 05/03/2026"
        );
        assert_eq!(normalize_numerals("١٬٢٠٠٫٥ و ٪٥"), "1,200.5 و %5");
        assert_eq!(normalize_numerals("۱۲۰۰"), "1200");
        assert_eq!(normalize_numerals("１２００％、３：３０"), "1200%、3:30");
        // Digit groups split by spaces, no-break spaces and apostrophes are joined.
        assert_eq!(normalize_numerals("1 200 €"), "1,200 €");
        assert_eq!(
            normalize_numerals("1\u{202F}200\u{00A0}000 €"),
            "1,200,000 €"
        );
        assert_eq!(normalize_numerals("CHF 1'200.50"), "CHF 1,200.50");
        // ...but not when they can't be one number.
        assert_eq!(
            normalize_numerals("in 2024 100 people"),
            "in 2024 100 people"
        );
        assert_eq!(
            normalize_numerals("pages 3, 100 and 7"),
            "pages 3, 100 and 7"
        );
        assert_eq!(normalize_numerals("1 2000"), "1 2000");
        assert_eq!(normalize_numerals("plain text"), "plain text");
    }

    #[test]
    fn numbers_survive_into_bangla_digits() {
        let src = "The rent of $1,200 is due on 05/03/2026, and the late fee is 5%.";
        let ok = translated(
            src,
            "$১,২০০ ভাড়া ০৫/০৩/২০২৬ তারিখের মধ্যে দিতে হবে, এবং দেরির ফি ৫%।",
            "bn",
        );
        assert!(ok.is_clean(), "{ok:?}");
        // ASCII digits in a Bangla translation are fine too.
        assert!(translated(src, "$1,200 ভাড়া 05/03/2026 তারিখে, দেরির ফি 5%।", "bn").is_clean());
        let bad = translated(src, "$১,৩০০ ভাড়া ২০২৬ সালে দিতে হবে।", "bn");
        assert_eq!(texts(&bad.missing), vec!["$1,200", "5%"]);
        assert_eq!(texts(&bad.added), vec!["$1,300"]);
    }

    #[test]
    fn numbers_survive_into_devanagari_and_arabic_digits() {
        let src = "About 1.2 million people paid 120,000 rupees by 31/12/2025.";
        assert!(
            translated(
                src,
                "लगभग 12 लाख लोगों ने 31/12/2025 तक 1,20,000 रुपये दिए।",
                "hi"
            )
            .is_clean()
        );
        assert!(
            translated(
                src,
                "लगभग १२ लाख लोगों ने ३१/१२/२०२५ तक १,२०,००० रुपये दिए।",
                "hi"
            )
            .is_clean()
        );
        let bad = translated(src, "लगभग १२ लाख लोगों ने २०२५ तक रुपये दिए।", "hi");
        assert_eq!(texts(&bad.missing), vec!["120,000"]);
        let src = "The fee is 1,200.5 dollars, or 3.5% a year.";
        assert!(translated(src, "الرسوم ١٬٢٠٠٫٥ دولار، أو ٣٫٥٪ سنويا.", "ar").is_clean());
        assert!(translated(src, "الرسوم ۱۲۰۰.۵ دولار، أو ۳.۵٪ سنويا.", "ar").is_clean());
        let bad = translated(src, "الرسوم ١٬٢٠٠ دولار.", "ar");
        assert_eq!(texts(&bad.missing), vec!["1,200.5", "3.5%"]);
        assert_eq!(texts(&bad.added), vec!["1,200"]);
    }

    #[test]
    fn continental_notation_in_spanish() {
        let src = "The deposit is $1,200,000.50, about 1.2 million, plus 3.5% interest.";
        let ok = translated(
            src,
            "El depósito es de 1.200.000,50 $, unos 1,2 millones, más un 3,5 % de interés.",
            "es",
        );
        assert!(ok.is_clean(), "{ok:?}");
        // A translation that keeps the English notation is fine too.
        assert!(
            translated(
                src,
                "El depósito es de $1,200,000.50, unos 1.2 millones, más un 3.5% de interés.",
                "es"
            )
            .is_clean()
        );
        assert!(translated("It costs $1,200.", "Cuesta 1.200 $.", "es").is_clean());
        assert!(translated("It costs $1,200.", "Cuesta 1 200 $.", "es").is_clean());
        let bad = translated("It costs $1,200.", "Cuesta 1.300 $.", "es");
        assert_eq!(texts(&bad.missing), vec!["$1,200"]);
        assert_eq!(texts(&bad.added), vec!["1.300"]);
    }

    #[test]
    fn chinese_numbers_dates_and_full_width_digits() {
        let src = "About 1.2 million people paid $1,200 on 12/03/2024.";
        let ok = translated(src, "约120万人在2024年3月12日支付了1,200美元。", "zh");
        assert!(ok.is_clean(), "{ok:?}");
        assert!(
            translated(
                src,
                "约１２０万人在２０２４年３月１２日支付了１２００美元。",
                "zh"
            )
            .is_clean()
        );
        let bad = translated(src, "约120万人支付了美元。", "zh");
        assert_eq!(texts(&bad.missing), vec!["$1,200", "12/03/2024"]);
    }

    #[test]
    fn names_are_not_checked_in_translations() {
        let src = "Analysts at Goldman Sachs met the WHO in Geneva in 2025.";
        // Across scripts, names and acronyms are transliterated: no false alarm.
        let bn = translated(
            src,
            "২০২৫ সালে গোল্ডম্যান স্যাকসের বিশ্লেষকরা জেনেভায় বিশ্ব স্বাস্থ্য সংস্থার সঙ্গে দেখা করেন।",
            "bn",
        );
        assert!(bn.is_clean(), "{bn:?}");
        assert!(translated(src, "2025年，高盛的分析师在日内瓦会见了世卫组织。", "zh").is_clean());
        // In the same script a translation writes names its own way: still no alarm.
        let es = translated(
            src,
            "Analistas de Goldman Sachs se reunieron con la OMS en Ginebra en 2025.",
            "es",
        );
        assert!(es.is_clean(), "{es:?}");
        // Numbers are still checked.
        let es_bad = translated(
            src,
            "Unos analistas se reunieron con la OMS en Ginebra.",
            "es",
        );
        assert_eq!(texts(&es_bad.missing), vec!["2025"]);
        // A Bangla source translated into English.
        assert!(translated("ঢাকায় ৫ জন এসেছিলেন।", "Five people came to Dhaka.", "en").is_clean());
    }

    #[test]
    fn number_words_are_required_only_in_english() {
        let src = "Pay within thirty days.";
        assert!(translated(src, "Paga en treinta días.", "es").is_clean());
        assert!(translated(src, "৩০ দিনের মধ্যে পরিশোধ করুন।", "bn").is_clean());
        // Into English, a number from the source may come back as a word.
        assert!(
            translated("৩০ দিনের মধ্যে পরিশোধ করুন।", "Pay within thirty days.", "en").is_clean()
        );
        assert!(translated("Paga en 30 días.", "Pay within 30 days.", "en").is_clean());
        let bad = translated("Paga en 30 días.", "Pay soon.", "en");
        assert_eq!(texts(&bad.missing), vec!["30"]);
    }

    #[test]
    fn invented_numbers_in_a_translation_are_reported() {
        let r = translated(
            "Pay the rent on time.",
            "ভাড়া ৫ তারিখের মধ্যে ৫০০ টাকা দিন।",
            "bn",
        );
        assert_eq!(texts(&r.added), vec!["5", "500"]);
        assert!(r.missing.is_empty());
    }

    #[test]
    fn times_survive_translation() {
        assert!(translated("Open 9:30 to 17:00.", "খোলা ৯:৩০ থেকে ১৭:০০।", "bn").is_clean());
        let r = translated("Open 9:30 to 17:00.", "খোলা ৯:৩০ থেকে।", "bn");
        assert_eq!(texts(&r.missing), vec!["17:00"]);
    }
}
