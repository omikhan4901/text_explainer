//! The bundled dictionary (WordNet 3.1 in SQLite, built by `scripts/build-dictionary.py`).
//! Single words and short terms get an instant entry here while the model writes the
//! meaning in context. Inflected forms are found through WordNet's own rules ("studies"
//! → "study") and its list of irregular forms ("mice" → "mouse").

use rusqlite::{Connection, OpenFlags, params};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sense {
    /// "noun", "verb", "adjective" or "adverb".
    pub pos: &'static str,
    pub definition: String,
    pub example: Option<String>,
    pub synonyms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    /// The dictionary form ("mouse" for "mice").
    pub lemma: String,
    pub senses: Vec<Sense>,
}

pub struct Dictionary {
    conn: Connection,
}

/// WordNet's detachment rules (morphy): suffix → replacement, per part of speech.
const RULES: &[(&str, &str, &str)] = &[
    ("n", "s", ""),
    ("n", "ses", "s"),
    ("n", "xes", "x"),
    ("n", "zes", "z"),
    ("n", "ches", "ch"),
    ("n", "shes", "sh"),
    ("n", "men", "man"),
    ("n", "ies", "y"),
    ("v", "s", ""),
    ("v", "ies", "y"),
    // Not in WordNet's list, but regular English: "studied", "carried".
    ("v", "ied", "y"),
    ("v", "es", "e"),
    ("v", "es", ""),
    ("v", "ed", "e"),
    ("v", "ed", ""),
    ("v", "ing", "e"),
    ("v", "ing", ""),
    ("a", "er", ""),
    ("a", "est", ""),
    ("a", "er", "e"),
    ("a", "est", "e"),
];

/// Parts of speech in the order entries are shown.
const POS_ORDER: [&str; 4] = ["n", "v", "a", "r"];

fn pos_name(code: &str) -> &'static str {
    match code {
        "n" => "noun",
        "v" => "verb",
        "a" | "s" => "adjective",
        _ => "adverb",
    }
}

impl Dictionary {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Ok(Dictionary { conn })
    }

    /// For tests and tools: a dictionary over an existing connection.
    pub fn from_connection(conn: Connection) -> Self {
        Dictionary { conn }
    }

    /// Looks up a word or short term, trying its dictionary forms. `None` if unknown.
    pub fn lookup(&self, word: &str) -> Option<Entry> {
        let word = normalise(word);
        if word.is_empty() || word.chars().count() > 60 {
            return None;
        }
        for candidate in self.candidates(&word) {
            let senses = self.senses(&candidate);
            if !senses.is_empty() {
                return Some(Entry {
                    lemma: candidate,
                    senses,
                });
            }
        }
        None
    }

    /// True when the word (or its base form) is an ordinary word, not only a proper noun:
    /// "lessee" and "agreement" are, "cambridge" isn't. Used by the meaning check so
    /// capitalised contract terms aren't mistaken for names.
    pub fn is_common_word(&self, word: &str) -> bool {
        let word = normalise(word);
        let Ok(mut stmt) = self
            .conn
            .prepare_cached("SELECT 1 FROM senses WHERE lemma = ?1 AND proper = 0 LIMIT 1")
        else {
            return false;
        };
        self.candidates(&word)
            .iter()
            .any(|c| stmt.exists(params![c]).unwrap_or(false))
    }

    /// The word and its American spelling, each followed by its irregular base forms and
    /// rule-based base forms.
    fn candidates(&self, word: &str) -> Vec<String> {
        let mut out = Vec::new();
        for spelling in [word.to_string(), american(word)] {
            out.extend(self.forms_of(&spelling));
        }
        let mut seen = std::collections::HashSet::new();
        out.retain(|c| seen.insert(c.clone()));
        out
    }

    fn forms_of(&self, word: &str) -> Vec<String> {
        let mut out = vec![word.to_string()];
        if let Ok(mut stmt) = self
            .conn
            .prepare_cached("SELECT lemma FROM forms WHERE form = ?1")
            && let Ok(rows) = stmt.query_map(params![word], |r| r.get::<_, String>(0))
        {
            out.extend(rows.flatten());
        }
        for &(_, suffix, replacement) in RULES {
            if let Some(stem) = word.strip_suffix(suffix)
                && stem.chars().count() >= 2
            {
                out.push(format!("{stem}{replacement}"));
            }
        }
        // "running" → "run": doubled consonant before -ing/-ed.
        for suffix in ["ing", "ed", "er", "est"] {
            if let Some(stem) = word.strip_suffix(suffix) {
                let b = stem.as_bytes();
                if b.len() >= 3 && b[b.len() - 1] == b[b.len() - 2] {
                    out.push(stem[..stem.len() - 1].to_string());
                }
            }
        }
        out
    }

    fn senses(&self, lemma: &str) -> Vec<Sense> {
        let Ok(mut stmt) = self.conn.prepare_cached(
            "SELECT pos, definition, examples, synonyms FROM senses WHERE lemma = ?1 ORDER BY pos, rank",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map(params![lemma], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        });
        let Ok(rows) = rows else { return Vec::new() };
        let mut senses: Vec<(usize, Sense)> = rows
            .flatten()
            .map(|(pos, definition, examples, synonyms)| {
                let examples: Vec<String> = serde_json::from_str(&examples).unwrap_or_default();
                let order = POS_ORDER.iter().position(|p| *p == pos).unwrap_or(4);
                (
                    order,
                    Sense {
                        pos: pos_name(&pos),
                        definition,
                        example: examples.into_iter().next(),
                        synonyms: serde_json::from_str(&synonyms).unwrap_or_default(),
                    },
                )
            })
            .collect();
        senses.sort_by_key(|(order, _)| *order);
        senses.into_iter().map(|(_, s)| s).collect()
    }
}

/// WordNet uses American spelling: "hypoxaemia" → "hypoxemia", "colour" → "color",
/// "organised" → "organized", "centre" → "center", "oestrogen" → "estrogen".
fn american(word: &str) -> String {
    let mut w = word.replace("ae", "e").replace("oe", "e");
    for (gb, us) in [
        ("our", "or"),
        ("ours", "ors"),
        ("ise", "ize"),
        ("ised", "ized"),
        ("ises", "izes"),
        ("ising", "izing"),
        ("isation", "ization"),
        ("tre", "ter"),
        ("tres", "ters"),
    ] {
        if let Some(stem) = w.strip_suffix(gb)
            && stem.chars().count() >= 3
        {
            w = format!("{stem}{us}");
            break;
        }
    }
    w
}

/// "  “Ubiquitous,” " → "ubiquitous"; "Habeas  Corpus" → "habeas corpus".
fn normalise(word: &str) -> String {
    crate::text::bare_word(word)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .replace(['’', '`'], "'")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A tiny dictionary with the same schema as the real one.
    pub fn fixture() -> Dictionary {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE senses (lemma TEXT, pos TEXT, rank INTEGER, definition TEXT, examples TEXT, synonyms TEXT, proper INTEGER DEFAULT 0);
            CREATE TABLE forms (form TEXT, lemma TEXT, pos TEXT);
            INSERT INTO senses (lemma, pos, rank, definition, examples, synonyms) VALUES ('ubiquitous','a',0,'being present everywhere at once','["ubiquitous computing"]','["omnipresent"]');
            INSERT INTO senses (lemma, pos, rank, definition, examples, synonyms) VALUES ('mouse','n',0,'any of numerous small rodents','[]','[]');
            INSERT INTO senses (lemma, pos, rank, definition, examples, synonyms) VALUES ('study','v',0,'consider in detail','[]','["analyze"]');
            INSERT INTO senses (lemma, pos, rank, definition, examples, synonyms) VALUES ('study','n',0,'a detailed critical inspection','[]','["survey"]');
            INSERT INTO senses (lemma, pos, rank, definition, examples, synonyms) VALUES ('run','v',0,'move fast by using one''s feet','["Don''t run!"]','[]');
            INSERT INTO senses (lemma, pos, rank, definition, examples, synonyms) VALUES ('go','v',0,'change location; move','[]','["travel"]');
            INSERT INTO senses (lemma, pos, rank, definition, examples, synonyms) VALUES ('habeas corpus','n',0,'a writ ordering a prisoner to be brought before a judge','[]','[]');
            INSERT INTO senses (lemma, pos, rank, definition, examples, synonyms) VALUES ('big','a',0,'above average in size','[]','["large"]');
            INSERT INTO senses VALUES ('lessee','n',0,'a tenant who holds a lease','[]','[]',0);
            INSERT INTO senses VALUES ('cambridge','n',0,'a city in eastern England','[]','[]',1);
            INSERT INTO forms VALUES ('mice','mouse','n');
            INSERT INTO forms VALUES ('went','go','v');
            "#,
        )
        .unwrap();
        Dictionary::from_connection(conn)
    }

    #[test]
    fn finds_words_and_normalises_them() {
        let d = fixture();
        let e = d.lookup("“Ubiquitous,”").unwrap();
        assert_eq!(e.lemma, "ubiquitous");
        assert_eq!(e.senses[0].pos, "adjective");
        assert_eq!(e.senses[0].example.as_deref(), Some("ubiquitous computing"));
        assert_eq!(e.senses[0].synonyms, vec!["omnipresent"]);
        assert_eq!(d.lookup("Habeas  Corpus").unwrap().lemma, "habeas corpus");
    }

    #[test]
    fn finds_inflected_and_irregular_forms() {
        let d = fixture();
        assert_eq!(d.lookup("mice").unwrap().lemma, "mouse");
        assert_eq!(d.lookup("went").unwrap().lemma, "go");
        assert_eq!(d.lookup("studies").unwrap().lemma, "study");
        assert_eq!(d.lookup("studied").unwrap().lemma, "study");
        assert_eq!(d.lookup("running").unwrap().lemma, "run");
        assert_eq!(d.lookup("bigger").unwrap().lemma, "big");
    }

    #[test]
    fn british_spellings_find_american_entries() {
        assert_eq!(american("hypoxaemia"), "hypoxemia");
        assert_eq!(american("colour"), "color");
        assert_eq!(american("organised"), "organized");
        assert_eq!(american("centre"), "center");
        assert_eq!(american("oestrogen"), "estrogen");
        assert_eq!(american("four"), "four", "short stems are left alone");
        assert_eq!(american("rise"), "rise");
    }

    #[test]
    fn tells_common_words_from_proper_nouns() {
        let d = fixture();
        assert!(d.is_common_word("Lessee"));
        assert!(d.is_common_word("studies"));
        assert!(!d.is_common_word("Cambridge"));
        assert!(!d.is_common_word("Qwertyuiop"));
    }

    #[test]
    fn nouns_come_before_verbs() {
        let d = fixture();
        let e = d.lookup("study").unwrap();
        assert_eq!(
            e.senses.iter().map(|s| s.pos).collect::<Vec<_>>(),
            vec!["noun", "verb"]
        );
    }

    #[test]
    fn unknown_or_odd_input_is_none() {
        let d = fixture();
        assert!(d.lookup("qwertyuiop").is_none());
        assert!(d.lookup("   ").is_none());
        assert!(d.lookup(&"a".repeat(200)).is_none());
        assert!(d.lookup("'; DROP TABLE senses; --").is_none());
        assert!(d.lookup("ubiquitous").is_some(), "the table is still there");
    }
}
