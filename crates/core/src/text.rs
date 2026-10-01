//! Cleaning and shaping the selected text before anything else sees it.
//!
//! Text copied from PDFs and web pages arrives damaged: words split by end-of-line
//! hyphens, paragraphs hard-wrapped into short lines, ligatures, soft hyphens and odd
//! spaces. Repairing that is part of making text readable, and it also keeps the model's
//! input clean.

use serde::Serialize;

/// The most text one request handles (about four pages). Longer selections are cut at a
/// paragraph or sentence boundary and the card says so.
pub const MAX_CHARS: usize = 16_000;

/// What the person selected, which decides how it is explained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionKind {
    /// Nothing but whitespace.
    Empty,
    /// One word: dictionary entry plus a meaning in context.
    Word,
    /// Two to four words without sentence punctuation: treated like a term.
    Phrase,
    /// Sentences and paragraphs: the readable rewrite.
    Passage,
}

/// Repairs text copied from PDFs and web pages. Keeps paragraph breaks and lists.
pub fn clean(raw: &str) -> String {
    let mut s = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '\r' => {}
            // Soft hyphen, zero-width space, word joiner, byte order mark. (ZWJ and ZWNJ
            // are kept: they matter in scripts such as Bengali.)
            '\u{00AD}' | '\u{200B}' | '\u{2060}' | '\u{FEFF}' => {}
            '\u{FB00}' => s.push_str("ff"),
            '\u{FB01}' => s.push_str("fi"),
            '\u{FB02}' => s.push_str("fl"),
            '\u{FB03}' => s.push_str("ffi"),
            '\u{FB04}' => s.push_str("ffl"),
            '\u{FB05}' | '\u{FB06}' => s.push_str("st"),
            '\t' => s.push(' '),
            '\u{2028}' | '\u{2029}' => s.push('\n'),
            c if c != '\n' && c.is_whitespace() => s.push(' '),
            c if c.is_control() && c != '\n' => {}
            c => s.push(c),
        }
    }

    let paragraphs: Vec<String> = split_paragraphs(&s)
        .into_iter()
        .map(|block| {
            let lines: Vec<&str> = block
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            join_lines(&lines)
        })
        .filter(|p| !p.is_empty())
        .collect();
    paragraphs.join("\n\n")
}

/// Splits on blank lines.
fn split_paragraphs(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for line in s.split('\n') {
        if line.trim().is_empty() {
            if !current.trim().is_empty() {
                out.push(std::mem::take(&mut current));
            }
            current.clear();
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    out
}

/// Joins the lines of one block. A block is treated as hard-wrapped (one paragraph
/// broken into lines by a PDF or an email client) when its lines are of similar length;
/// otherwise line breaks are meaningful (addresses, poems, short lists) and kept.
/// List items always start a new line.
fn join_lines(lines: &[&str]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let wrapped = is_hard_wrapped(lines);
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        let line = collapse_spaces(line);
        if i == 0 {
            out.push_str(&line);
            continue;
        }
        if is_list_item(&line) || !wrapped {
            out.push('\n');
            out.push_str(&line);
            continue;
        }
        // "infor-" + "mation" → "information" when both sides are lowercase letters.
        let prev_ends_hyphen = out.ends_with('-')
            && out
                .chars()
                .rev()
                .nth(1)
                .is_some_and(|c| c.is_alphabetic() && c.is_lowercase());
        let next_starts_lower = line
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() && c.is_lowercase());
        if prev_ends_hyphen && next_starts_lower {
            out.pop();
            out.push_str(&line);
        } else {
            out.push(' ');
            out.push_str(&line);
        }
    }
    out
}

fn is_hard_wrapped(lines: &[&str]) -> bool {
    if lines.len() < 2 {
        return false;
    }
    let lens: Vec<usize> = lines.iter().map(|l| l.chars().count()).collect();
    let longest = *lens.iter().max().unwrap_or(&0);
    if longest < 30 {
        // Short lines (addresses, menus, verse) are deliberate.
        return false;
    }
    // Every line but the last is reasonably full, and at least one doesn't end a sentence.
    let body = &lines[..lines.len() - 1];
    let full = body
        .iter()
        .filter(|l| l.chars().count() * 10 >= longest * 5)
        .count();
    let mid_sentence = body
        .iter()
        .filter(|l| !l.trim_end().ends_with(['.', '!', '?', ':', ';']))
        .count();
    full * 10 >= body.len() * 7 && mid_sentence > 0
}

/// "- item", "• item", "1. item", "2) item", "a) item".
pub fn is_list_item(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with(['-', '*', '•', '–', '·', '▪', '◦']) {
        return t.chars().nth(1).is_some_and(char::is_whitespace);
    }
    let marker: String = t
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    if marker.is_empty() || marker.len() > 3 {
        return false;
    }
    let numeric = marker.chars().all(|c| c.is_ascii_digit());
    let letter = marker.len() == 1 && marker.chars().all(|c| c.is_ascii_lowercase());
    let rest = &t[marker.len()..];
    (numeric || letter)
        && (rest.starts_with(". ") || rest.starts_with(") "))
        && rest.len() > 2
}

fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for c in s.chars() {
        if c == ' ' {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(c);
            space = false;
        }
    }
    out.trim().to_string()
}

/// Words as a reader counts them: whitespace-separated tokens containing a letter or digit.
pub fn words(s: &str) -> impl Iterator<Item = &str> {
    s.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
}

pub fn word_count(s: &str) -> usize {
    words(s).count()
}

/// Decides how a (cleaned) selection is explained.
pub fn classify(cleaned: &str) -> SelectionKind {
    let n = word_count(cleaned);
    if n == 0 {
        return SelectionKind::Empty;
    }
    let ends_sentence = cleaned.trim_end().ends_with(['.', '!', '?', '।']);
    if n == 1 && !cleaned.contains('\n') {
        return SelectionKind::Word;
    }
    if n <= 4 && !ends_sentence && !cleaned.contains('\n') {
        return SelectionKind::Phrase;
    }
    SelectionKind::Passage
}

/// Strips the punctuation around a single selected word: `“Ubiquitous,”` → `Ubiquitous`.
pub fn bare_word(s: &str) -> &str {
    s.trim()
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
        .trim_matches(['-', '\''])
}

/// Cuts text longer than `max_chars` at the last paragraph or sentence end before the
/// limit. Returns the text and whether it was cut.
pub fn truncate(s: &str, max_chars: usize) -> (&str, bool) {
    if s.chars().count() <= max_chars {
        return (s, false);
    }
    let byte_limit = s
        .char_indices()
        .nth(max_chars)
        .map_or(s.len(), |(i, _)| i);
    let head = &s[..byte_limit];
    let cut = head
        .rfind("\n\n")
        .filter(|&i| i > byte_limit / 2)
        .or_else(|| {
            sentence_spans(head)
                .last()
                .map(|r| r.end)
                .filter(|&e| e > byte_limit / 2 && e < head.len())
        })
        .unwrap_or(byte_limit);
    (s[..cut].trim_end(), true)
}

const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "vs", "etc", "e.g", "i.e", "cf", "al",
    "fig", "figs", "no", "nos", "vol", "pp", "p", "ed", "eds", "approx", "dept", "inc", "ltd",
    "co", "corp", "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "sept", "oct", "nov",
    "dec", "u.s", "u.k", "a.m", "p.m", "art", "sec", "ch", "para", "op", "cit", "ibid", "approx",
];

/// Byte ranges of the sentences in `s`. Handles abbreviations, initials, decimals and
/// closing quotes; good enough for readability scores and chunking.
pub fn sentence_spans(s: &str) -> Vec<std::ops::Range<usize>> {
    let mut spans = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (pos, c) = chars[i];
        let is_break = match c {
            '\n' => {
                // A line break ends a sentence in lists and headings.
                true
            }
            '!' | '?' | '।' | '。' => true,
            '.' => !is_non_terminal_period(s, &chars, i),
            _ => false,
        };
        if is_break {
            // Include closing quotes and brackets: She said "no." Then…
            let mut j = i + 1;
            while j < chars.len() && matches!(chars[j].1, '"' | '\'' | '”' | '’' | ')' | ']') {
                j += 1;
            }
            // Repeated terminators: "Really?!" or "Wait..."
            while j < chars.len() && matches!(chars[j].1, '.' | '!' | '?') {
                j += 1;
            }
            let end = chars.get(j).map_or(s.len(), |&(p, _)| p);
            let followed_by_space_or_end = j >= chars.len() || chars[j].1.is_whitespace();
            if c == '\n' || followed_by_space_or_end {
                if !s[start..end].trim().is_empty() {
                    spans.push(trim_range(s, start..end));
                }
                start = end;
                i = j;
                continue;
            }
        }
        let _ = pos;
        i += 1;
    }
    if !s[start..].trim().is_empty() {
        spans.push(trim_range(s, start..s.len()));
    }
    spans
}

fn trim_range(s: &str, r: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let slice = &s[r.clone()];
    let lead = slice.len() - slice.trim_start().len();
    let trail = slice.len() - slice.trim_end().len();
    r.start + lead..r.end - trail
}

fn is_non_terminal_period(s: &str, chars: &[(usize, char)], i: usize) -> bool {
    // Decimal or version number: 3.5, v1.2
    let prev = i.checked_sub(1).map(|k| chars[k].1);
    let next = chars.get(i + 1).map(|&(_, c)| c);
    if prev.is_some_and(|c| c.is_ascii_digit()) && next.is_some_and(|c| c.is_ascii_digit()) {
        return true;
    }
    // Inside a token (example.com, e.g.)
    if next.is_some_and(|c| c.is_alphanumeric()) {
        return true;
    }
    // The word before the period.
    let end = chars[i].0;
    let word_start = s[..end]
        .rfind(|c: char| c.is_whitespace() || c == '(' || c == '"' || c == '“')
        .map_or(0, |p| p + s[p..].chars().next().map_or(1, char::len_utf8));
    let word = s[word_start..end].to_lowercase();
    if ABBREVIATIONS.contains(&word.as_str()) {
        return true;
    }
    // A single capital letter is an initial: "J. K. Rowling".
    let mut wc = word.chars();
    if let (Some(c), None) = (wc.next(), wc.next())
        && s[word_start..end].chars().all(char::is_uppercase)
        && c.is_alphabetic()
    {
        return true;
    }
    // Next word starts lowercase: "approx. three" is not a new sentence.
    let rest = s[end + 1..].trim_start();
    rest.chars()
        .next()
        .is_some_and(|c| c.is_lowercase() && c.is_alphabetic())
}

pub fn sentences(s: &str) -> Vec<&str> {
    sentence_spans(s).into_iter().map(|r| &s[r]).collect()
}

/// Splits a passage into pieces of at most about `max_words`, at paragraph boundaries
/// first and sentence boundaries inside long paragraphs, so each piece can be rewritten
/// on its own and streamed in order.
pub fn chunk(s: &str, max_words: usize) -> Vec<String> {
    let max_words = max_words.max(20);
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_words = 0;

    fn flush(chunks: &mut Vec<String>, current: &mut String, current_words: &mut usize) {
        if !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
        }
        current.clear();
        *current_words = 0;
    }

    for para in s.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        let n = word_count(para);
        if n > max_words {
            flush(&mut chunks, &mut current, &mut current_words);
            let mut piece = String::new();
            let mut piece_words = 0;
            for sentence in sentences(para) {
                let sn = word_count(sentence);
                if piece_words > 0 && piece_words + sn > max_words {
                    chunks.push(std::mem::take(&mut piece));
                    piece_words = 0;
                }
                if !piece.is_empty() {
                    piece.push(if para.contains('\n') { '\n' } else { ' ' });
                }
                piece.push_str(sentence);
                piece_words += sn;
            }
            if !piece.is_empty() {
                chunks.push(piece);
            }
            continue;
        }
        if current_words > 0 && current_words + n > max_words {
            flush(&mut chunks, &mut current, &mut current_words);
        }
        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(para);
        current_words += n;
    }
    flush(&mut chunks, &mut current, &mut current_words);
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_pdf_hyphenation_and_hard_wraps() {
        let raw = "The committee reviewed the infor-\nmation provided by the applicant and\ndecided that further evidence was re-\nquired before a final decision.";
        assert_eq!(
            clean(raw),
            "The committee reviewed the information provided by the applicant and decided that further evidence was required before a final decision."
        );
    }

    #[test]
    fn keeps_hyphen_before_capitalised_continuation() {
        let raw = "This approach was first used in the pre-\nWar period by many governments around the world.\nIt worked.";
        assert!(clean(raw).contains("pre- War"));
    }

    #[test]
    fn keeps_paragraphs_and_lists() {
        let raw = "Requirements for the application process are listed below for all\napplicants who wish to apply this year:\n\n- a passport\n- two photos\n1. Fill the form\n2) Pay the fee";
        let c = clean(raw);
        assert_eq!(
            c,
            "Requirements for the application process are listed below for all applicants who wish to apply this year:\n\n- a passport\n- two photos\n1. Fill the form\n2) Pay the fee"
        );
    }

    #[test]
    fn keeps_short_deliberate_lines() {
        let raw = "Jane Doe\n12 High Street\nLondon";
        assert_eq!(clean(raw), raw);
    }

    #[test]
    fn fixes_ligatures_spaces_and_invisible_characters() {
        let raw = "\u{FEFF}The \u{FB01}nal\u{00A0}e\u{00AD}ffect\u{200B}  is  \tclear.\r\n";
        assert_eq!(clean(raw), "The final effect is clear.");
    }

    #[test]
    fn keeps_joiners_used_by_bengali() {
        let raw = "র\u{200D}্যাব";
        assert_eq!(clean(raw), raw);
    }

    #[test]
    fn collapses_many_blank_lines() {
        assert_eq!(clean("One.\n\n\n\n\nTwo."), "One.\n\nTwo.");
    }

    #[test]
    fn classifies_selections() {
        assert_eq!(classify(""), SelectionKind::Empty);
        assert_eq!(classify("   \n "), SelectionKind::Empty);
        assert_eq!(classify("ubiquitous"), SelectionKind::Word);
        assert_eq!(classify("“ubiquitous,”"), SelectionKind::Word);
        assert_eq!(classify("habeas corpus"), SelectionKind::Phrase);
        assert_eq!(classify("res ipsa loquitur doctrine"), SelectionKind::Phrase);
        assert_eq!(classify("It rained."), SelectionKind::Passage);
        assert_eq!(
            classify("the party of the first part hereinafter"),
            SelectionKind::Passage
        );
        assert_eq!(classify("one\ntwo"), SelectionKind::Passage);
    }

    #[test]
    fn bare_word_strips_surrounding_punctuation() {
        assert_eq!(bare_word("“Ubiquitous,”"), "Ubiquitous");
        assert_eq!(bare_word("(well-known)"), "well-known");
        assert_eq!(bare_word("'tis"), "tis");
        assert_eq!(bare_word("don't."), "don't");
    }

    #[test]
    fn splits_sentences_around_abbreviations_initials_and_decimals() {
        let s = "Dr. Smith paid $3.50 for it, e.g. at the U.S. store. J. K. Rowling wrote it. Really?! Yes… \"Done.\" Next one";
        assert_eq!(
            sentences(s),
            vec![
                "Dr. Smith paid $3.50 for it, e.g. at the U.S. store.",
                "J. K. Rowling wrote it.",
                "Really?!",
                "Yes… \"Done.\"",
                "Next one",
            ]
        );
    }

    #[test]
    fn line_breaks_end_sentences() {
        assert_eq!(sentences("Heading\nBody text here."), vec!["Heading", "Body text here."]);
    }

    #[test]
    fn urls_and_versions_do_not_split() {
        assert_eq!(
            sentences("See example.com for v1.2 details. Then stop."),
            vec!["See example.com for v1.2 details.", "Then stop."]
        );
    }

    #[test]
    fn chunks_by_paragraph_then_sentence() {
        let para = |n: usize| {
            (0..n)
                .map(|i| format!("Sentence number {i} has five words."))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let text = format!("{}\n\n{}\n\n{}", para(2), para(2), para(30));
        let chunks = chunk(&text, 25);
        // The two short paragraphs share a chunk; the long one is split by sentences.
        assert_eq!(chunks[0], format!("{}\n\n{}", para(2), para(2)));
        assert!(chunks.len() > 3);
        for c in &chunks {
            assert!(word_count(c) <= 25, "chunk too long: {c}");
        }
        let rejoined: usize = chunks.iter().map(|c| word_count(c)).sum();
        assert_eq!(rejoined, word_count(&text));
    }

    #[test]
    fn a_single_huge_sentence_is_its_own_chunk() {
        let long = "word ".repeat(100);
        let chunks = chunk(long.trim(), 30);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn truncates_at_a_paragraph_or_sentence_end() {
        let text = format!("{}\n\n{}", "First paragraph here. ".repeat(10).trim(), "Second. ".repeat(50));
        let (head, cut) = truncate(&text, 300);
        assert!(cut);
        assert!(head.ends_with('.'));
        assert!(head.chars().count() <= 300);
        let (same, cut) = truncate("short", 300);
        assert_eq!((same, cut), ("short", false));
    }

    #[test]
    fn truncate_handles_multibyte_text() {
        let text = "আমি বাংলায় গান গাই। ".repeat(200);
        let (head, cut) = truncate(&text, 100);
        assert!(cut);
        assert!(head.chars().count() <= 100);
    }

    #[test]
    fn list_items() {
        assert!(is_list_item("- milk"));
        assert!(is_list_item("• milk"));
        assert!(is_list_item("12. milk"));
        assert!(is_list_item("b) milk"));
        assert!(!is_list_item("-milk"));
        assert!(!is_list_item("2026. was a year"));
        assert!(!is_list_item("Mr. Smith"));
    }
}
