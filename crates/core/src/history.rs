//! The Word book (words the person saved from the card) and the lookup history, in
//! `history.sqlite` in the app's local data folder.
//!
//! The schema is versioned with `PRAGMA user_version` and migrated by [`migrate`], which is
//! safe to run any number of times. Input is checked and trimmed once, here at the
//! boundary; every query binds its values (no SQL is built from text). A file that isn't a
//! database or fails its integrity check is renamed aside (`history.broken.sqlite`) and a
//! fresh one is started; when even that fails the book lives in memory for the session, so
//! the app never stops working over it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, ErrorCode, OptionalExtension, Row, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use crate::text::SelectionKind;

/// The schema version this build writes.
pub const SCHEMA_VERSION: i32 = 1;
/// A saved term (a word, a short term, or a short sentence).
pub const MAX_TERM_CHARS: usize = 200;
/// Each of definition, explanation and context.
pub const MAX_TEXT_CHARS: usize = 8000;
/// A lookup as kept in the history (longer selections are cut).
pub const MAX_QUERY_CHARS: usize = 300;
/// The history never keeps more lookups than this.
pub const MAX_HISTORY_ROWS: u32 = 2000;
/// The most rows one page of a list returns.
pub const MAX_PAGE: u32 = 200;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Version 1: lookups and saved words. `STRICT` tables so a wrong type is an error, not a
/// silent conversion; the checks repeat the limits enforced in Rust.
const V1: &str = "
CREATE TABLE IF NOT EXISTS lookups (
    id INTEGER PRIMARY KEY,
    query TEXT NOT NULL CHECK (length(query) BETWEEN 1 AND 300),
    kind TEXT NOT NULL CHECK (kind IN ('word', 'phrase', 'passage')),
    created_at INTEGER NOT NULL
) STRICT;
CREATE INDEX IF NOT EXISTS lookups_created_at ON lookups (created_at);
CREATE TABLE IF NOT EXISTS saved (
    id INTEGER PRIMARY KEY,
    term TEXT NOT NULL CHECK (length(term) BETWEEN 1 AND 200),
    term_key TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('word', 'phrase', 'passage')),
    definition TEXT CHECK (length(definition) <= 8000),
    explanation TEXT CHECK (length(explanation) <= 8000),
    context TEXT CHECK (length(context) <= 8000),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;
CREATE INDEX IF NOT EXISTS saved_created_at ON saved (created_at);
";

/// What kind of selection a lookup or saved term was (the card's classification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Word,
    Phrase,
    Passage,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Word => "word",
            Kind::Phrase => "phrase",
            Kind::Passage => "passage",
        }
    }

    fn parse(s: &str) -> Option<Kind> {
        match s {
            "word" => Some(Kind::Word),
            "phrase" => Some(Kind::Phrase),
            "passage" => Some(Kind::Passage),
            _ => None,
        }
    }

    /// An empty selection is never recorded.
    pub fn of(kind: SelectionKind) -> Option<Kind> {
        match kind {
            SelectionKind::Empty => None,
            SelectionKind::Word => Some(Kind::Word),
            SelectionKind::Phrase => Some(Kind::Phrase),
            SelectionKind::Passage => Some(Kind::Passage),
        }
    }
}

/// A word to save, as the card sends it. Empty texts count as missing.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct NewWord {
    pub term: String,
    pub kind: Option<Kind>,
    #[serde(default)]
    pub definition: Option<String>,
    #[serde(default)]
    pub explanation: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SavedWord {
    pub id: i64,
    pub term: String,
    pub kind: Kind,
    pub definition: Option<String>,
    pub explanation: Option<String>,
    pub context: Option<String>,
    /// Unix milliseconds.
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Lookup {
    pub id: i64,
    pub query: String,
    pub kind: Kind,
    pub created_at: i64,
}

/// One page of a list and how many rows there are in all.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
}

/// How long lookups are kept (from settings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retention {
    pub keep: bool,
    pub days: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("there is nothing to save")]
    EmptyTerm,
    #[error("that is too long to save (at most {MAX_TERM_CHARS} characters)")]
    TermTooLong,
    #[error("the word book couldn't be read or written: {0}")]
    Db(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Why a file couldn't be used as the book.
#[derive(Debug, thiserror::Error)]
enum OpenError {
    #[error("not a usable database ({0})")]
    Corrupt(String),
    #[error("written by a newer version of the app (schema {0})")]
    Newer(i32),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
}

type Clock = Box<dyn Fn() -> i64 + Send>;

pub struct History {
    conn: Connection,
    clock: Clock,
    /// False when the file couldn't be used and the book is in memory for this session.
    persistent: bool,
}

/// Unix milliseconds now.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

impl History {
    /// Opens (or creates) the book at `path`. Never fails: a corrupt file is set aside and
    /// replaced; if the folder can't be written, the book lives in memory.
    pub fn open(path: &Path) -> History {
        let conn = match open_file(path) {
            Ok(conn) => Some(conn),
            Err(OpenError::Corrupt(why)) => {
                let aside = broken_path(path);
                tracing::warn!(
                    "word book unreadable ({why}); keeping it as {} and starting a new one",
                    aside.display()
                );
                set_aside(path, &aside);
                open_file(path)
                    .map_err(|e| tracing::error!("couldn't start a new word book: {e}"))
                    .ok()
            }
            Err(e) => {
                // Not corrupt (a newer schema, a locked or unwritable file): leave it alone.
                tracing::error!("word book unavailable ({e}); keeping it in memory this time");
                None
            }
        };
        let persistent = conn.is_some();
        let conn = conn.unwrap_or_else(|| {
            let mut conn = Connection::open_in_memory().expect("in-memory SQLite");
            migrate(&mut conn).expect("in-memory schema");
            conn
        });
        History {
            conn,
            clock: Box::new(now_ms),
            persistent,
        }
    }

    /// For tests: a fixed or moving clock instead of the system time.
    pub fn with_clock(mut self, clock: impl Fn() -> i64 + Send + 'static) -> Self {
        self.clock = Box::new(clock);
        self
    }

    pub fn is_persistent(&self) -> bool {
        self.persistent
    }

    /// Records that the card opened on `query`, then prunes lookups older than the
    /// retention and beyond [`MAX_HISTORY_ROWS`]. Returns false when nothing was recorded
    /// (history is off, or the query is blank).
    pub fn record(&mut self, query: &str, kind: Kind, retention: Retention) -> Result<bool> {
        if !retention.keep {
            return Ok(false);
        }
        let query = truncate_chars(&one_line(query), MAX_QUERY_CHARS);
        if query.is_empty() {
            return Ok(false);
        }
        let now = (self.clock)();
        let days = i64::from(retention.days.clamp(1, 365));
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO lookups (query, kind, created_at) VALUES (?1, ?2, ?3)",
            params![query, kind.as_str(), now],
        )?;
        tx.execute(
            "DELETE FROM lookups WHERE created_at < ?1",
            params![now - days * DAY_MS],
        )?;
        tx.execute(
            "DELETE FROM lookups WHERE id IN (
                SELECT id FROM lookups ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?1
            )",
            params![MAX_HISTORY_ROWS],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// Newest first.
    pub fn history(&self, limit: u32, offset: u32) -> Result<Page<Lookup>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, query, kind, created_at FROM lookups
             ORDER BY created_at DESC, id DESC LIMIT ?1 OFFSET ?2",
        )?;
        let items = stmt
            .query_map(params![page_limit(limit), offset], |r| {
                Ok(Lookup {
                    id: r.get(0)?,
                    query: r.get(1)?,
                    kind: kind_at(r, 2)?,
                    created_at: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let total: i64 = self
            .conn
            .query_row("SELECT count(*) FROM lookups", [], |r| r.get(0))?;
        Ok(Page { items, total })
    }

    /// Deletes every lookup; returns how many there were.
    pub fn clear_history(&mut self) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM lookups", [])?)
    }

    /// Saves a word, or updates it if a word with the same key is saved: the first saved
    /// time stays, the given texts replace the stored ones, missing ones keep theirs.
    pub fn save(&mut self, word: &NewWord) -> Result<SavedWord> {
        let term = one_line(&word.term);
        if term.is_empty() {
            return Err(Error::EmptyTerm);
        }
        if term.chars().count() > MAX_TERM_CHARS {
            return Err(Error::TermTooLong);
        }
        let kind = word.kind.unwrap_or_else(|| guess_kind(&term));
        let now = (self.clock)();
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO saved
                (term, term_key, kind, definition, explanation, context, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT (term_key) DO UPDATE SET
                term = excluded.term,
                kind = excluded.kind,
                definition = coalesce(excluded.definition, saved.definition),
                explanation = coalesce(excluded.explanation, saved.explanation),
                context = coalesce(excluded.context, saved.context),
                updated_at = excluded.updated_at
             RETURNING id, term, kind, definition, explanation, context, created_at, updated_at",
        )?;
        Ok(stmt.query_row(
            params![
                term,
                term_key(&term),
                kind.as_str(),
                text_field(word.definition.as_deref()),
                text_field(word.explanation.as_deref()),
                text_field(word.context.as_deref()),
                now
            ],
            saved_word,
        )?)
    }

    /// Removes a saved word; true if it was saved.
    pub fn remove(&mut self, term: &str) -> Result<bool> {
        Ok(self.conn.execute(
            "DELETE FROM saved WHERE term_key = ?1",
            params![term_key(term)],
        )? > 0)
    }

    pub fn is_saved(&self, term: &str) -> Result<bool> {
        Ok(self
            .conn
            .prepare_cached("SELECT 1 FROM saved WHERE term_key = ?1")?
            .exists(params![term_key(term)])?)
    }

    pub fn get(&self, term: &str) -> Result<Option<SavedWord>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, term, kind, definition, explanation, context, created_at, updated_at
                 FROM saved WHERE term_key = ?1",
                params![term_key(term)],
                saved_word,
            )
            .optional()?)
    }

    /// Saved words, newest first. `search` matches anywhere in the term (any case), the
    /// definition or the explanation; `%` and `_` in it are matched literally.
    pub fn words(&self, search: &str, limit: u32, offset: u32) -> Result<Page<SavedWord>> {
        let pattern = like_pattern(search);
        let filter = "?1 IS NULL
            OR term_key LIKE ?1 ESCAPE '\\'
            OR definition LIKE ?1 ESCAPE '\\'
            OR explanation LIKE ?1 ESCAPE '\\'";
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT id, term, kind, definition, explanation, context, created_at, updated_at
             FROM saved WHERE {filter}
             ORDER BY created_at DESC, id DESC LIMIT ?2 OFFSET ?3"
        ))?;
        let items = stmt
            .query_map(params![pattern, page_limit(limit), offset], saved_word)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let total: i64 = self.conn.query_row(
            &format!("SELECT count(*) FROM saved WHERE {filter}"),
            params![pattern],
            |r| r.get(0),
        )?;
        Ok(Page { items, total })
    }

    /// Every saved word, oldest first (for export).
    pub fn all_words(&self) -> Result<Vec<SavedWord>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, term, kind, definition, explanation, context, created_at, updated_at
             FROM saved ORDER BY created_at, id",
        )?;
        Ok(stmt
            .query_map([], saved_word)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn open_file(path: &Path) -> std::result::Result<Connection, OpenError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    // The first statement that reads the file tells a broken file apart.
    let mode: String = conn
        .pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))
        .map_err(classify)?;
    if !mode.eq_ignore_ascii_case("wal") {
        tracing::debug!("word book journal mode is {mode}");
    }
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(classify)?;
    if check != "ok" {
        return Err(OpenError::Corrupt(check));
    }
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// Brings the schema up to [`SCHEMA_VERSION`]. Idempotent: an up-to-date database is left
/// as it is, and each step runs in one transaction with its version bump.
fn migrate(conn: &mut Connection) -> std::result::Result<(), OpenError> {
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(classify)?;
    if version > SCHEMA_VERSION {
        return Err(OpenError::Newer(version));
    }
    if version < 1 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(V1).map_err(classify)?;
        tx.pragma_update(None, "user_version", 1)?;
        tx.commit()?;
    }
    Ok(())
}

fn classify(e: rusqlite::Error) -> OpenError {
    match e.sqlite_error_code() {
        Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt) => {
            OpenError::Corrupt(e.to_string())
        }
        _ => OpenError::Db(e),
    }
}

fn broken_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .map_or_else(|| "history".into(), |s| s.to_string_lossy().into_owned());
    path.with_file_name(format!("{stem}.broken.sqlite"))
}

/// Moves the file and its WAL and shared-memory files aside (replacing an older broken
/// copy), so nothing is lost for good.
fn set_aside(path: &Path, aside: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let from = PathBuf::from(format!("{}{suffix}", path.display()));
        let to = PathBuf::from(format!("{}{suffix}", aside.display()));
        if from.exists()
            && let Err(e) = std::fs::rename(&from, &to)
        {
            tracing::warn!("couldn't move {} aside: {e}", from.display());
            let _ = std::fs::remove_file(&from);
        }
    }
}

fn saved_word(r: &Row<'_>) -> rusqlite::Result<SavedWord> {
    Ok(SavedWord {
        id: r.get(0)?,
        term: r.get(1)?,
        kind: kind_at(r, 2)?,
        definition: r.get(3)?,
        explanation: r.get(4)?,
        context: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
    })
}

fn kind_at(r: &Row<'_>, i: usize) -> rusqlite::Result<Kind> {
    let s: String = r.get(i)?;
    Kind::parse(&s).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            i,
            rusqlite::types::Type::Text,
            format!("unknown kind {s:?}").into(),
        )
    })
}

fn page_limit(limit: u32) -> u32 {
    limit.clamp(1, MAX_PAGE)
}

/// The key that makes "Ubiquitous", " ubiquitous " and "UBIQUITOUS" one saved word:
/// trimmed, lowercased (Unicode-aware), inner whitespace collapsed to one space.
pub fn term_key(term: &str) -> String {
    one_line(term).to_lowercase()
}

/// Whitespace (any Unicode whitespace, including line breaks) collapsed to single spaces.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most `max` characters, cut on a character boundary.
pub fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((cut, _)) => s[..cut].to_string(),
        None => s.to_string(),
    }
}

/// A stored text: trimmed, at most [`MAX_TEXT_CHARS`], and `None` when empty.
fn text_field(s: Option<&str>) -> Option<String> {
    let s = s?.trim();
    (!s.is_empty()).then(|| truncate_chars(s, MAX_TEXT_CHARS))
}

fn guess_kind(term: &str) -> Kind {
    match term.split_whitespace().count() {
        1 => Kind::Word,
        2..=4 => Kind::Phrase,
        _ => Kind::Passage,
    }
}

/// A LIKE pattern matching `search` anywhere, with the wildcards and the escape character
/// escaped; `None` for an empty search (everything matches).
fn like_pattern(search: &str) -> Option<String> {
    let needle = term_key(search);
    if needle.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(needle.len() + 2);
    out.push('%');
    for c in needle.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    Some(out)
}

// ---- Export ---------------------------------------------------------------------------

/// Spreadsheet-friendly CSV (RFC 4180: every field quoted, quotes doubled, CRLF line ends)
/// with a UTF-8 byte order mark so Excel reads Bangla and other scripts correctly. Cells
/// that a spreadsheet would run as a formula are prefixed with `'`.
pub fn to_csv(words: &[SavedWord]) -> String {
    let mut out = String::from("\u{feff}");
    let mut row = |cells: [&str; 6]| {
        out.push_str(&cells.map(csv_cell).join(","));
        out.push_str("\r\n");
    };
    row([
        "term",
        "kind",
        "definition",
        "explanation",
        "context",
        "saved",
    ]);
    for w in words {
        row([
            &w.term,
            w.kind.as_str(),
            w.definition.as_deref().unwrap_or(""),
            w.explanation.as_deref().unwrap_or(""),
            w.context.as_deref().unwrap_or(""),
            &iso_date(w.created_at),
        ]);
    }
    out
}

fn csv_cell(value: &str) -> String {
    let formula = matches!(
        value.chars().next(),
        Some('=' | '+' | '-' | '@' | '\t' | '\r')
    );
    let value = if formula {
        format!("'{value}")
    } else {
        value.to_string()
    };
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Anki's plain-text import: one note per line, front and back separated by a tab, with
/// HTML in the fields (escaped text, line breaks as `<br>`). The header lines tell Anki
/// (2.1.55 and later) the separator and that fields are HTML.
pub fn to_anki(words: &[SavedWord]) -> String {
    let mut out = String::from("#separator:tab\n#html:true\n");
    for w in words {
        let back: Vec<String> = [w.definition.as_deref(), w.explanation.as_deref()]
            .into_iter()
            .flatten()
            .map(anki_html)
            .collect();
        out.push_str(&anki_html(&w.term));
        out.push('\t');
        out.push_str(&back.join("<br><br>"));
        out.push('\n');
    }
    out
}

fn anki_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.trim().replace("\r\n", "\n").chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("<br>"),
            '\t' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// `YYYY-MM-DD` (UTC) for unix milliseconds.
fn iso_date(ms: i64) -> String {
    // Howard Hinnant's days-to-civil algorithm.
    let z = ms.div_euclid(DAY_MS) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};

    const T0: i64 = 1_760_000_000_000;
    const KEEP_30: Retention = Retention {
        keep: true,
        days: 30,
    };

    fn word(term: &str) -> NewWord {
        NewWord {
            term: term.into(),
            kind: Some(Kind::Word),
            ..Default::default()
        }
    }

    /// A book in a temporary folder with a clock the test moves.
    fn book() -> (tempfile::TempDir, History, Arc<AtomicI64>) {
        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(AtomicI64::new(T0));
        let c = clock.clone();
        let h = History::open(&dir.path().join("history.sqlite"))
            .with_clock(move || c.load(Ordering::SeqCst));
        assert!(h.is_persistent());
        (dir, h, clock)
    }

    #[test]
    fn creates_the_schema_with_wal_and_version() {
        let (_dir, h, _) = book();
        let version: i32 = h
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let mode: String = h
            .conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        let fk: bool = h
            .conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert!(fk);
    }

    #[test]
    fn opening_twice_migrates_once_and_keeps_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        {
            let mut h = History::open(&path);
            h.save(&word("lessee")).unwrap();
            h.record("lessee", Kind::Word, KEEP_30).unwrap();
        }
        let mut h = History::open(&path);
        assert!(h.is_persistent());
        assert!(h.is_saved("lessee").unwrap());
        assert_eq!(h.history(10, 0).unwrap().total, 1);
        // Running the migration again on an up-to-date database changes nothing.
        migrate(&mut h.conn).unwrap();
        migrate(&mut h.conn).unwrap();
        assert_eq!(h.words("", 10, 0).unwrap().total, 1);
        assert!(!dir.path().join("history.broken.sqlite").exists());
    }

    #[test]
    fn a_v0_database_with_other_tables_is_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE unrelated (x INTEGER); INSERT INTO unrelated VALUES (7);")
            .unwrap();
        drop(conn);
        let h = History::open(&path);
        assert!(h.is_persistent());
        let x: i64 = h
            .conn
            .query_row("SELECT x FROM unrelated", [], |r| r.get(0))
            .unwrap();
        assert_eq!(x, 7);
        assert!(!h.is_saved("anything").unwrap());
    }

    #[test]
    fn upsert_keeps_created_refreshes_updated_and_fills_only_given_fields() {
        let (_dir, mut h, clock) = book();
        let first = h
            .save(&NewWord {
                term: "Ubiquitous".into(),
                kind: Some(Kind::Word),
                definition: Some("being present everywhere at once".into()),
                explanation: None,
                context: Some("Smartphones are ubiquitous.".into()),
            })
            .unwrap();
        assert_eq!(first.created_at, T0);
        assert_eq!(first.updated_at, T0);
        assert_eq!(first.explanation, None);

        clock.store(T0 + 5_000, Ordering::SeqCst);
        let second = h
            .save(&NewWord {
                term: "  ubiquitous ".into(),
                kind: Some(Kind::Word),
                definition: None,
                explanation: Some("Found almost everywhere.".into()),
                context: Some("   ".into()),
            })
            .unwrap();
        assert_eq!(second.id, first.id);
        assert_eq!(second.term, "ubiquitous");
        assert_eq!(second.created_at, T0);
        assert_eq!(second.updated_at, T0 + 5_000);
        assert_eq!(
            second.definition.as_deref(),
            Some("being present everywhere at once")
        );
        assert_eq!(
            second.explanation.as_deref(),
            Some("Found almost everywhere.")
        );
        assert_eq!(
            second.context.as_deref(),
            Some("Smartphones are ubiquitous.")
        );
        assert_eq!(h.words("", 10, 0).unwrap().total, 1);
        assert_eq!(h.get("UBIQUITOUS").unwrap(), Some(second));
    }

    #[test]
    fn remove_and_is_saved_use_the_normalised_key() {
        let (_dir, mut h, _) = book();
        h.save(&word("Carte  Blanche")).unwrap();
        assert!(h.is_saved("carte blanche").unwrap());
        assert!(h.is_saved("\tCARTE\nblanche ").unwrap());
        assert!(h.remove(" carte BLANCHE").unwrap());
        assert!(!h.remove("carte blanche").unwrap());
        assert!(!h.is_saved("Carte Blanche").unwrap());
    }

    #[test]
    fn keys_fold_case_and_whitespace_across_scripts() {
        assert_eq!(term_key("  Hello   World "), "hello world");
        assert_eq!(term_key("ÉCOLE"), "école");
        assert_eq!(term_key("STRASSE"), "strasse");
        assert_eq!(term_key("ΟΔΥΣΣΕΥΣ"), "οδυσσευς");
        // No-break space and ideographic space are whitespace too.
        assert_eq!(term_key("a\u{a0}b\u{3000}c"), "a b c");
        // Scripts without case are kept as they are.
        assert_eq!(term_key(" অভিধান "), "অভিধান");
        assert_eq!(term_key("中文"), "中文");
    }

    #[test]
    fn unicode_terms_round_trip() {
        let (_dir, mut h, _) = book();
        h.save(&word("অভিধান")).unwrap();
        h.save(&word("Éclair")).unwrap();
        assert!(h.is_saved("অভিধান").unwrap());
        assert!(h.is_saved("éCLAIR").unwrap());
        let found = h.words("ÉCL", 10, 0).unwrap();
        assert_eq!(found.total, 1);
        assert_eq!(found.items[0].term, "Éclair");
        assert_eq!(h.words("অভি", 10, 0).unwrap().total, 1);
    }

    #[test]
    fn search_treats_wildcards_literally() {
        let (_dir, mut h, _) = book();
        for t in ["50%", "500", "a_b", "axb", "back\\slash", "backslash"] {
            h.save(&word(t)).unwrap();
        }
        let terms = |s: &str| -> Vec<String> {
            let mut v: Vec<String> = h
                .words(s, 50, 0)
                .unwrap()
                .items
                .into_iter()
                .map(|w| w.term)
                .collect();
            v.sort();
            v
        };
        assert_eq!(terms("50%"), vec!["50%"]);
        assert_eq!(terms("%"), vec!["50%"]);
        assert_eq!(terms("a_b"), vec!["a_b"]);
        assert_eq!(terms("_"), vec!["a_b"]);
        assert_eq!(terms("k\\s"), vec!["back\\slash"]);
        assert_eq!(terms("'; DROP TABLE saved; --"), Vec::<String>::new());
        assert_eq!(terms("").len(), 6);
        assert_eq!(terms("   ").len(), 6);
    }

    #[test]
    fn search_finds_definitions_and_counts_matches() {
        let (_dir, mut h, clock) = book();
        for (i, t) in ["alpha", "beta", "gamma"].iter().enumerate() {
            clock.store(T0 + i as i64, Ordering::SeqCst);
            h.save(&NewWord {
                definition: Some(format!("letter number {}", i + 1)),
                ..word(t)
            })
            .unwrap();
        }
        let page = h.words("LETTER", 2, 0).unwrap();
        assert_eq!(page.total, 3);
        // Newest first.
        assert_eq!(
            page.items.iter().map(|w| &w.term[..]).collect::<Vec<_>>(),
            ["gamma", "beta"]
        );
        let rest = h.words("letter", 2, 2).unwrap();
        assert_eq!(rest.items.len(), 1);
        assert_eq!(rest.items[0].term, "alpha");
        assert_eq!(h.words("number 2", 10, 0).unwrap().items[0].term, "beta");
    }

    #[test]
    fn page_sizes_are_capped() {
        let (_dir, mut h, _) = book();
        for i in 0..(MAX_PAGE + 5) {
            h.save(&word(&format!("w{i}"))).unwrap();
        }
        assert_eq!(
            h.words("", u32::MAX, 0).unwrap().items.len(),
            MAX_PAGE as usize
        );
        assert_eq!(h.words("", 0, 0).unwrap().items.len(), 1);
        assert!(h.words("", 10, u32::MAX).unwrap().items.is_empty());
        assert_eq!(h.all_words().unwrap().len(), MAX_PAGE as usize + 5);
    }

    #[test]
    fn limits_are_enforced_at_the_boundary() {
        let (_dir, mut h, _) = book();
        assert!(matches!(h.save(&word("  ")), Err(Error::EmptyTerm)));
        let long = "ক".repeat(MAX_TERM_CHARS + 1);
        assert!(matches!(h.save(&word(&long)), Err(Error::TermTooLong)));
        let exact = "ক".repeat(MAX_TERM_CHARS);
        assert!(h.save(&word(&exact)).is_ok());

        let huge = "ভা".repeat(MAX_TEXT_CHARS);
        let saved = h
            .save(&NewWord {
                definition: Some(huge.clone()),
                explanation: Some(huge.clone()),
                context: Some(huge),
                ..word("big")
            })
            .unwrap();
        for field in [saved.definition, saved.explanation, saved.context] {
            let field = field.unwrap();
            assert_eq!(field.chars().count(), MAX_TEXT_CHARS);
            assert!(field.starts_with("ভা"));
        }

        let query = "ষ".repeat(1000);
        h.record(&query, Kind::Passage, KEEP_30).unwrap();
        let kept = &h.history(1, 0).unwrap().items[0];
        assert_eq!(kept.query.chars().count(), MAX_QUERY_CHARS);
    }

    #[test]
    fn the_schema_rejects_bad_rows_written_around_the_api() {
        let (_dir, h, _) = book();
        let bad_kind = h.conn.execute(
            "INSERT INTO lookups (query, kind, created_at) VALUES ('x', 'empty', 1)",
            [],
        );
        assert!(bad_kind.is_err());
        let bad_type = h.conn.execute(
            "INSERT INTO lookups (query, kind, created_at) VALUES ('x', 'word', 'soon')",
            [],
        );
        assert!(bad_type.is_err());
        let duplicate = h.conn.execute_batch(
            "INSERT INTO saved (term, term_key, kind, created_at, updated_at) VALUES ('a', 'a', 'word', 1, 1);
             INSERT INTO saved (term, term_key, kind, created_at, updated_at) VALUES ('A', 'a', 'word', 1, 1);",
        );
        assert!(duplicate.is_err());
    }

    #[test]
    fn history_is_one_line_newest_first_and_off_means_nothing_kept() {
        let (_dir, mut h, clock) = book();
        assert!(h.record("first\n\nline", Kind::Phrase, KEEP_30).unwrap());
        clock.store(T0 + 1, Ordering::SeqCst);
        assert!(h.record("second", Kind::Word, KEEP_30).unwrap());
        assert!(!h.record("   ", Kind::Word, KEEP_30).unwrap());
        let off = Retention {
            keep: false,
            days: 30,
        };
        assert!(!h.record("secret", Kind::Word, off).unwrap());
        let page = h.history(10, 0).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.items[0].query, "second");
        assert_eq!(page.items[1].query, "first line");
        assert_eq!(page.items[1].kind, Kind::Phrase);
        assert_eq!(h.clear_history().unwrap(), 2);
        assert_eq!(h.history(10, 0).unwrap().total, 0);
    }

    #[test]
    fn old_lookups_are_pruned_on_insert() {
        let (_dir, mut h, clock) = book();
        h.record("old", Kind::Word, KEEP_30).unwrap();
        clock.store(T0 + 10 * DAY_MS, Ordering::SeqCst);
        h.record("recent", Kind::Word, KEEP_30).unwrap();
        // 30 days and a moment after the first: it goes, the second stays.
        clock.store(T0 + 30 * DAY_MS + 1, Ordering::SeqCst);
        h.record("now", Kind::Word, KEEP_30).unwrap();
        let queries: Vec<String> = h
            .history(10, 0)
            .unwrap()
            .items
            .into_iter()
            .map(|l| l.query)
            .collect();
        assert_eq!(queries, ["now", "recent"]);
        // A shorter retention prunes more; out-of-range days are clamped (0 is one day).
        clock.store(T0 + 30 * DAY_MS + 2, Ordering::SeqCst);
        h.record(
            "latest",
            Kind::Word,
            Retention {
                keep: true,
                days: 0,
            },
        )
        .unwrap();
        assert_eq!(h.history(10, 0).unwrap().total, 2);
    }

    #[test]
    fn history_is_capped() {
        let (_dir, mut h, clock) = book();
        let total = MAX_HISTORY_ROWS + 25;
        for i in 0..total {
            clock.store(T0 + i64::from(i), Ordering::SeqCst);
            h.record(&format!("q{i}"), Kind::Word, KEEP_30).unwrap();
        }
        let page = h.history(1, 0).unwrap();
        assert_eq!(page.total, i64::from(MAX_HISTORY_ROWS));
        assert_eq!(page.items[0].query, format!("q{}", total - 1));
        let last = h.history(1, MAX_HISTORY_ROWS - 1).unwrap();
        assert_eq!(last.items[0].query, "q25");
    }

    #[test]
    fn a_corrupt_file_is_set_aside_and_a_new_book_starts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        std::fs::write(
            &path,
            b"this is not a database at all, just text ".repeat(200),
        )
        .unwrap();
        let mut h = History::open(&path);
        assert!(h.is_persistent());
        assert!(dir.path().join("history.broken.sqlite").exists());
        h.save(&word("fresh")).unwrap();
        drop(h);
        assert!(History::open(&path).is_saved("fresh").unwrap());
    }

    #[test]
    fn a_damaged_database_is_set_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        {
            let mut h = History::open(&path);
            for i in 0..300 {
                h.save(&NewWord {
                    definition: Some("x".repeat(500)),
                    ..word(&format!("w{i}"))
                })
                .unwrap();
            }
            h.conn
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
                .unwrap();
        }
        // Overwrite pages in the middle of the file.
        let mut bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 40_000);
        for b in &mut bytes[8192..32_768] {
            *b = 0xA5;
        }
        std::fs::write(&path, bytes).unwrap();
        let h = History::open(&path);
        assert!(h.is_persistent());
        assert!(dir.path().join("history.broken.sqlite").exists());
        assert_eq!(h.words("", 10, 0).unwrap().total, 0);
    }

    #[test]
    fn a_newer_schema_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        drop(conn);
        let mut h = History::open(&path);
        assert!(!h.is_persistent());
        // It still works for this session, in memory.
        h.save(&word("temporary")).unwrap();
        assert!(h.is_saved("temporary").unwrap());
        drop(h);
        assert!(!dir.path().join("history.broken.sqlite").exists());
        let conn = Connection::open(&path).unwrap();
        let v: i32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, 99);
    }

    #[test]
    fn an_unwritable_place_falls_back_to_memory() {
        let dir = tempfile::tempdir().unwrap();
        // A file where the folder should be: the database can't be created.
        let blocker = dir.path().join("folder");
        std::fs::write(&blocker, b"").unwrap();
        let mut h = History::open(&blocker.join("history.sqlite"));
        assert!(!h.is_persistent());
        h.record("still works", Kind::Word, KEEP_30).unwrap();
        assert_eq!(h.history(10, 0).unwrap().total, 1);
    }

    #[test]
    fn kinds_follow_the_card_or_are_guessed() {
        assert_eq!(Kind::of(SelectionKind::Empty), None);
        assert_eq!(Kind::of(SelectionKind::Phrase), Some(Kind::Phrase));
        let (_dir, mut h, _) = book();
        let guess = |h: &mut History, t: &str| {
            h.save(&NewWord {
                term: t.into(),
                ..Default::default()
            })
            .unwrap()
            .kind
        };
        assert_eq!(guess(&mut h, "word"), Kind::Word);
        assert_eq!(guess(&mut h, "carte blanche"), Kind::Phrase);
        assert_eq!(guess(&mut h, "one two three four five"), Kind::Passage);
        let parsed: NewWord = serde_json::from_str(r#"{"term":"x","kind":"phrase"}"#).unwrap();
        assert_eq!(parsed.kind, Some(Kind::Phrase));
        assert!(serde_json::from_str::<NewWord>(r#"{"term":"x","kind":"empty"}"#).is_err());
    }

    fn sample(term: &str, definition: Option<&str>, explanation: Option<&str>) -> SavedWord {
        SavedWord {
            id: 1,
            term: term.into(),
            kind: Kind::Word,
            definition: definition.map(String::from),
            explanation: explanation.map(String::from),
            context: None,
            created_at: T0,
            updated_at: T0,
        }
    }

    #[test]
    fn csv_is_quoted_has_a_bom_and_defuses_formulas() {
        let csv = to_csv(&[
            sample("say \"hi\", ok", Some("line one\nline two"), None),
            sample("=HYPERLINK(\"http://x\")", Some("+1"), Some("-2")),
            sample("@SUM(A1)", Some("\tTabbed"), Some("a-b")),
            sample("অভিধান", Some("a book of words"), None),
        ]);
        assert!(csv.starts_with("\u{feff}\"term\",\"kind\",\"definition\""));
        let lines: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(
            lines[1],
            "\"say \"\"hi\"\", ok\",\"word\",\"line one\nline two\",\"\",\"\",\"2025-10-09\""
        );
        assert!(lines[2].starts_with("\"'=HYPERLINK(\"\"http://x\"\")\",\"word\",\"'+1\",\"'-2\""));
        assert!(lines[3].starts_with("\"'@SUM(A1)\",\"word\",\"'\tTabbed\",\"a-b\""));
        assert!(lines[4].starts_with("\"অভিধান\""));
        assert!(csv.ends_with("\r\n"));
    }

    #[test]
    fn anki_is_tab_separated_html() {
        let tsv = to_anki(&[
            sample(
                "<b>bold</b> & co",
                Some("a \"quoted\"\tdefinition"),
                Some("first\nsecond"),
            ),
            sample("solo", None, None),
        ]);
        let lines: Vec<&str> = tsv.lines().collect();
        assert_eq!(lines[0], "#separator:tab");
        assert_eq!(lines[1], "#html:true");
        assert_eq!(
            lines[2],
            "&lt;b&gt;bold&lt;/b&gt; &amp; co\ta &quot;quoted&quot; definition<br><br>first<br>second"
        );
        assert_eq!(lines[3], "solo\t");
        assert_eq!(lines.len(), 4);
        for line in &lines[2..] {
            assert_eq!(line.matches('\t').count(), 1);
        }
    }

    #[test]
    fn dates_are_utc_calendar_days() {
        assert_eq!(iso_date(0), "1970-01-01");
        assert_eq!(iso_date(951_782_400_000), "2000-02-29");
        assert_eq!(iso_date(1_760_486_399_999), "2025-10-14");
        assert_eq!(iso_date(-1), "1969-12-31");
    }
}
