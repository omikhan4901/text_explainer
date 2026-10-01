//! Cleaning model output. Even with good prompts, small models add "Sure! Here's a
//! simpler version:", wrap the answer in quotes or tags, or emit a thinking block.
//!
//! `StreamFilter` does this while tokens stream in, holding back only the first line
//! until it knows whether it's a preamble, so the card can show text immediately.
//! `finish` returns the final cleaned text, which the card swaps in at the end.

/// Removes thinking blocks, preambles, wrapping tags/quotes and trailing notes.
pub fn clean(raw: &str) -> String {
    let mut s = strip_thinking(raw);
    s = strip_tags(&s);
    s = strip_preamble(&s).to_string();
    s = strip_trailing_note(&s).to_string();
    s = unwrap_quotes(s.trim());
    s.trim().to_string()
}

fn strip_thinking(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        match rest.find("<think>") {
            Some(start) => {
                out.push_str(&rest[..start]);
                match rest[start..].find("</think>") {
                    Some(end) => rest = &rest[start + end + "</think>".len()..],
                    // Unterminated: everything after is thinking.
                    None => return out,
                }
            }
            None => {
                out.push_str(rest);
                return out;
            }
        }
    }
}

fn strip_tags(s: &str) -> String {
    s.replace("<text>", "").replace("</text>", "")
}

const PREAMBLE_STARTS: &[&str] = &[
    "sure",
    "certainly",
    "of course",
    "okay",
    "ok,",
    "ok!",
    "here is",
    "here's",
    "here are",
    "below is",
    "absolutely",
    "great",
    "rewritten",
    "simplified",
    "plain version",
    "simpler version",
    "the rewritten",
    "the simplified",
    "in plain",
    "in simpler",
];

/// "Sure! Here is a simpler version:" and similar first lines.
fn strip_preamble(s: &str) -> &str {
    let t = s.trim_start();
    let first_line_end = t.find('\n').unwrap_or(t.len());
    let first = t[..first_line_end].trim();
    let lower = first.to_lowercase();
    let starts = PREAMBLE_STARTS.iter().any(|p| lower.starts_with(p));
    let ends_with_colon = first.ends_with(':') || first.ends_with("：");
    let short = first.split_whitespace().count() <= 14;
    if starts && short && (ends_with_colon || first_line_end < t.len()) {
        return &t[first_line_end..];
    }
    // "Sure! Here's the text: The tenant must…" on one line.
    if starts
        && let Some(colon) = first.find(": ")
        && first[..colon].split_whitespace().count() <= 10
    {
        return &t[colon + 2..];
    }
    t
}

const NOTE_STARTS: &[&str] = &[
    "note:",
    "(note",
    "notes:",
    "i hope",
    "let me know",
    "i have kept",
    "i kept",
    "i've kept",
];

/// A final paragraph like "Note: I kept the numbers the same." is chatter.
fn strip_trailing_note(s: &str) -> &str {
    let t = s.trim_end();
    if let Some(idx) = t.rfind("\n\n") {
        let last = t[idx..].trim().to_lowercase();
        if NOTE_STARTS.iter().any(|p| last.starts_with(p)) {
            return &t[..idx];
        }
    }
    t
}

fn unwrap_quotes(s: &str) -> String {
    for (open, close) in [("\"", "\""), ("“", "”"), ("```", "```"), ("'", "'")] {
        if s.len() > open.len() + close.len()
            && s.starts_with(open)
            && s.ends_with(close)
            && !s[open.len()..s.len() - close.len()].contains(open)
        {
            return s[open.len()..s.len() - close.len()].trim().to_string();
        }
    }
    s.to_string()
}

/// Cleans streamed output incrementally. Emits text only once it can't be part of a
/// preamble or a thinking block.
#[derive(Debug, Default)]
pub struct StreamFilter {
    raw: String,
    emitted: usize,
}

impl StreamFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a delta; returns the newly displayable text (may be empty).
    pub fn push(&mut self, delta: &str) -> String {
        self.raw.push_str(delta);
        let stable = self.stable_prefix();
        if stable.len() > self.emitted && stable.is_char_boundary(self.emitted) {
            let new = stable[self.emitted..].to_string();
            self.emitted = stable.len();
            new
        } else {
            String::new()
        }
    }

    /// The final, fully cleaned text.
    pub fn finish(&self) -> String {
        clean(&self.raw)
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The cleaned text that won't change as more tokens arrive (except trailing notes
    /// and closing quotes, which only `finish` removes).
    fn stable_prefix(&self) -> String {
        let no_think = strip_thinking(&self.raw);
        // Inside an unterminated think block, or one might be starting.
        if self.raw.contains("<think>") && !self.raw.contains("</think>") {
            return String::new();
        }
        let t = strip_tags(&no_think);
        let trimmed = t.trim_start();
        if trimmed.is_empty() || "<think>".starts_with(trimmed) || "<text>".starts_with(trimmed) {
            return String::new();
        }
        // Wait for the first line to finish (or get long) before deciding on a preamble.
        let first_line_done = trimmed.contains('\n') || trimmed.chars().count() > 160;
        let colon_seen = trimmed.contains(": ");
        if !first_line_done && !colon_seen {
            let lower = trimmed.to_lowercase();
            if PREAMBLE_STARTS
                .iter()
                .any(|p| lower.starts_with(p) || p.starts_with(lower.as_str()))
            {
                return String::new();
            }
        }
        let body = strip_preamble(trimmed).trim_start();
        let body = body.strip_prefix(['"', '“']).unwrap_or(body);
        // Hold back a partial "<" that may be the start of a tag.
        let cut = body
            .rfind('<')
            .filter(|&i| !body[i..].contains('>'))
            .unwrap_or(body.len());
        body[..cut].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_common_preambles() {
        for raw in [
            "Sure! Here is a simpler version:\n\nThe tenant pays rent.",
            "Here's the rewritten text:\nThe tenant pays rent.",
            "Certainly. Here is the text in plain English:\n\nThe tenant pays rent.",
            "Rewritten text: The tenant pays rent.",
            "<text>\nThe tenant pays rent.\n</text>",
            "\"The tenant pays rent.\"",
            "<think>The user wants…</think>\nThe tenant pays rent.",
            "<think>\n\n</think>\n\nThe tenant pays rent.",
            "The tenant pays rent.\n\nNote: I kept all numbers the same.",
        ] {
            assert_eq!(clean(raw), "The tenant pays rent.", "{raw:?}");
        }
    }

    #[test]
    fn keeps_real_content_that_looks_similar() {
        // A real first sentence that starts like a preamble but isn't one.
        assert_eq!(
            clean("Here is how the process works. First you apply."),
            "Here is how the process works. First you apply."
        );
        assert_eq!(
            clean("Okay means the same as fine."),
            "Okay means the same as fine."
        );
        // Quotes inside the text stay.
        assert_eq!(
            clean("\"Yes,\" she said. \"No,\" he said."),
            "\"Yes,\" she said. \"No,\" he said."
        );
        // A note in the middle stays.
        assert_eq!(
            clean("Note: rent is due.\n\nPay on time."),
            "Note: rent is due.\n\nPay on time."
        );
    }

    #[test]
    fn unterminated_thinking_produces_nothing() {
        assert_eq!(clean("<think>still thinking"), "");
    }

    fn stream(chunks: &[&str]) -> (String, String) {
        let mut f = StreamFilter::new();
        let mut shown = String::new();
        for c in chunks {
            shown.push_str(&f.push(c));
        }
        (shown, f.finish())
    }

    #[test]
    fn streaming_hides_preamble_and_think_blocks() {
        let (shown, done) = stream(&[
            "Sure",
            "! Here",
            " is a simpler",
            " version:\n\n",
            "The tenant",
            " pays rent.",
        ]);
        assert_eq!(shown, "The tenant pays rent.");
        assert_eq!(done, "The tenant pays rent.");

        let (shown, done) = stream(&["<thi", "nk>hmm", "</think>", "\n", "Pay ", "rent."]);
        assert_eq!(shown, "Pay rent.");
        assert_eq!(done, "Pay rent.");
    }

    #[test]
    fn streaming_shows_ordinary_text_quickly() {
        let mut f = StreamFilter::new();
        // "The " could still become "The rewritten text:", so it waits one token.
        assert_eq!(f.push("The "), "");
        assert_eq!(f.push("tenant"), "The tenant");
        assert_eq!(f.push(" <te"), " ");
        assert_eq!(f.push("xt>"), "");
    }

    #[test]
    fn streaming_handles_multibyte_text() {
        let (shown, done) = stream(&["ভাড়া ", "প্রতি ", "মাসে।"]);
        assert_eq!(shown, "ভাড়া প্রতি মাসে।");
        assert_eq!(done, "ভাড়া প্রতি মাসে।");
    }

    #[test]
    fn streamed_text_is_a_prefix_of_the_final_text() {
        let cases: &[&[&str]] = &[
            &["Here's the", " rewritten text:", " The fee", " is 5%."],
            &["\"The fee", " is 5%.\""],
            &["The fee is 5%.", "\n\nNote: kept numbers."],
            &["<text>", "The fee is 5%.", "</text>"],
        ];
        for chunks in cases {
            let (shown, done) = stream(chunks);
            assert!(
                done.starts_with(shown.trim_end()) || shown.starts_with(&done),
                "{chunks:?}: {shown:?} vs {done:?}"
            );
        }
    }
}
