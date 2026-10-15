//! Web sources: sites the card can show in a tab, next to the local Explain and Dictionary
//! tabs (`docs/SOURCES_PLAN.md`). A source is a URL template where `{query}` becomes the
//! selected text and `{lang}` the language chosen for translations. Nothing here goes
//! online: the card loads a source's page in its open tab (the first search, unless the
//! person picks another), and never while "Offline only" is on.

use reqwest::Url;
use serde::{Deserialize, Serialize};

/// Longest selection put into a URL, in characters (search and translation sites accept
/// far more, but very long URLs are refused by some servers).
pub const MAX_QUERY_CHARS: usize = 1500;
/// Most sources the settings keep.
pub const MAX_SOURCES: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSource {
    /// Stable id ("wikipedia", or "custom-…" for ones the person added).
    pub id: String,
    pub name: String,
    /// `https://…{query}…`; `{lang}` is optional.
    pub url: String,
    pub enabled: bool,
    /// Shown for single words and short terms.
    pub words: bool,
    /// Shown for sentences and longer.
    pub passages: bool,
}

/// Which language search sites show their pages in (their menus and AI answers; results
/// stay in the language they were written in).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "code", rename_all = "snake_case")]
pub enum WebLanguage {
    /// The site decides (it guesses from where the person is).
    Automatic,
    /// The main translation language (`translate_to`).
    SameAsTranslation,
    /// A language code ("en", "bn").
    Fixed(String),
}

impl Default for WebLanguage {
    /// English (the owner's choice, 15 October 2026).
    fn default() -> Self {
        WebLanguage::Fixed("en".into())
    }
}

impl WebLanguage {
    /// The language code to ask for, if any.
    pub fn code<'a>(&'a self, translate_to: &'a str) -> Option<&'a str> {
        match self {
            WebLanguage::Automatic => None,
            WebLanguage::SameAsTranslation => Some(translate_to),
            WebLanguage::Fixed(code) => Some(code),
        }
    }

    /// A fixed code that isn't a plain language code ("en", "zh-CN") becomes English.
    pub fn sanitized(self) -> Self {
        match self {
            WebLanguage::Fixed(code) => {
                let code = code.trim();
                if is_language_code(code) {
                    WebLanguage::Fixed(code.to_string())
                } else {
                    WebLanguage::default()
                }
            }
            other => other,
        }
    }
}

/// "en", "bn", "zh-CN": letters, digits and hyphens, at most 12 characters.
pub fn is_language_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 12
        && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Asks a search site for its page in `lang`: Google's `hl`, Bing's `setlang`. Other sites
/// (and the translators, which have their own target language) are left as they are, and
/// so is an address that already names a language.
pub fn with_page_language(mut url: Url, lang: &str) -> Url {
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let google = host == "google.com" || host.starts_with("www.google.");
    let bing =
        (host == "www.bing.com" || host == "bing.com") && !url.path().starts_with("/translator");
    let (param, code) = match (google, bing) {
        (true, _) => ("hl", lang),
        (_, true) => ("setlang", bing_lang(lang)),
        _ => return url,
    };
    if !is_language_code(code) || url.query_pairs().any(|(k, _)| k == param) {
        return url;
    }
    url.query_pairs_mut().append_pair(param, code);
    url
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    #[error("the address must start with https:// or http://")]
    Scheme,
    #[error("the address must contain {{query}} where the selected text goes")]
    NoQuery,
    #[error("that isn't a web address")]
    Invalid,
}

fn source(
    id: &str,
    name: &str,
    url: &str,
    enabled: bool,
    words: bool,
    passages: bool,
) -> WebSource {
    WebSource {
        id: id.into(),
        name: name.into(),
        url: url.into(),
        enabled,
        words,
        passages,
    }
}

/// The built-in sources: free sites that need no account or key.
pub fn defaults() -> Vec<WebSource> {
    vec![
        source(
            "google",
            "Google",
            "https://www.google.com/search?q={query}",
            true,
            true,
            true,
        ),
        source(
            "bing",
            "Bing",
            "https://www.bing.com/search?q={query}",
            true,
            true,
            true,
        ),
        source(
            "duckduckgo",
            "DuckDuckGo",
            "https://duckduckgo.com/?q={query}",
            true,
            true,
            true,
        ),
        source(
            "wikipedia",
            "Wikipedia",
            "https://en.wikipedia.org/w/index.php?search={query}",
            true,
            true,
            false,
        ),
        source(
            "translate",
            "Translate",
            "https://translate.google.com/?sl=auto&tl={lang}&text={query}&op=translate",
            true,
            true,
            true,
        ),
        source(
            "bing-translator",
            "Bing Translator",
            "https://www.bing.com/translator?from=auto&to={lang}&text={query}",
            false,
            true,
            true,
        ),
        source(
            "wiktionary",
            "Wiktionary",
            "https://en.wiktionary.org/w/index.php?search={query}",
            false,
            true,
            false,
        ),
        source(
            "cambridge",
            "Cambridge",
            "https://dictionary.cambridge.org/search/english/direct/?q={query}",
            false,
            true,
            false,
        ),
        source(
            "brave",
            "Brave",
            "https://search.brave.com/search?q={query}",
            false,
            true,
            true,
        ),
        source(
            "images",
            "Images",
            "https://www.bing.com/images/search?q={query}",
            false,
            true,
            false,
        ),
    ]
}

/// A web source as the card lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WebTab {
    pub id: String,
    pub name: String,
}

/// The web tabs for one selection, in the person's order: none while "Offline only" is on.
pub fn tabs_for(sources: &[WebSource], offline_only: bool, word: bool) -> Vec<WebTab> {
    if offline_only {
        return Vec::new();
    }
    sources
        .iter()
        .filter(|s| s.enabled && if word { s.words } else { s.passages })
        .map(|s| WebTab {
            id: s.id.clone(),
            name: s.name.clone(),
        })
        .collect()
}

/// Checks a template the person typed.
pub fn validate(template: &str) -> Result<(), SourceError> {
    let t = template.trim();
    let lower = t.to_ascii_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err(SourceError::Scheme);
    }
    if !t.contains("{query}") {
        return Err(SourceError::NoQuery);
    }
    // The site must be fixed by the template: the selection may never choose the host.
    let host_with = |q: &str| -> Result<String, SourceError> {
        let url = Url::parse(&t.replace("{query}", q).replace("{lang}", "en"))
            .map_err(|_| SourceError::Invalid)?;
        url.host_str()
            .filter(|h| !h.is_empty())
            .map(str::to_string)
            .ok_or(SourceError::Invalid)
    };
    if host_with("one")? != host_with("two")? {
        return Err(SourceError::Invalid);
    }
    Ok(())
}

/// The selection as it goes into a URL: whitespace collapsed, at most
/// [`MAX_QUERY_CHARS`] characters.
pub fn query_text(selection: &str) -> String {
    let collapsed = selection.split_whitespace().collect::<Vec<_>>().join(" ");
    match collapsed.char_indices().nth(MAX_QUERY_CHARS) {
        Some((cut, _)) => collapsed[..cut].trim_end().to_string(),
        None => collapsed,
    }
}

/// The page to open: the template with the selection and language filled in.
pub fn build_url(template: &str, selection: &str, lang: &str) -> Result<Url, SourceError> {
    validate(template)?;
    let filled = template
        .trim()
        .replace("{query}", &encode(&query_text(selection)))
        .replace("{lang}", &encode(lang_for(template, lang.trim())));
    Url::parse(&filled).map_err(|_| SourceError::Invalid)
}

/// The language code a site expects: Bing names Chinese by script ("zh-Hans"), where the
/// setting uses Google's region codes ("zh-CN").
fn lang_for<'a>(template: &str, lang: &'a str) -> &'a str {
    if Url::parse(template).is_ok_and(|u| u.host_str() == Some("www.bing.com")) {
        bing_lang(lang)
    } else {
        lang
    }
}

fn bing_lang(lang: &str) -> &str {
    match lang {
        "zh-CN" => "zh-Hans",
        "zh-TW" => "zh-Hant",
        _ => lang,
    }
}

/// Percent-encodes everything but RFC 3986 unreserved characters, so the text is safe in a
/// path or a query either way.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Keeps the list usable whatever a settings file says: unique, non-empty ids and names,
/// invalid templates switched off, and at most [`MAX_SOURCES`].
pub fn sanitize(sources: &mut Vec<WebSource>) {
    let mut seen = std::collections::HashSet::new();
    sources.retain(|s| !s.id.trim().is_empty() && seen.insert(s.id.clone()));
    sources.truncate(MAX_SOURCES);
    for s in sources.iter_mut() {
        s.name = s.name.trim().chars().take(40).collect();
        if s.name.is_empty() {
            s.name = s.id.clone();
        }
        if validate(&s.url).is_err() {
            s.enabled = false;
        }
    }
    // Built-in sources can't be removed, so one missing was added in a later version:
    // it joins the end of the list as it ships (Bing Translator, for one, starts off).
    for d in defaults() {
        if sources.len() < MAX_SOURCES && !sources.iter().any(|s| s.id == d.id) {
            sources.push(d);
        }
    }
}

#[cfg(test)]
mod tests {
    fn page(template: &str, lang: &str) -> String {
        with_page_language(build_url(template, "a swing", "bn").unwrap(), lang).to_string()
    }

    #[test]
    fn search_pages_are_asked_for_the_chosen_language() {
        assert_eq!(
            page("https://www.google.com/search?q={query}", "en"),
            "https://www.google.com/search?q=a%20swing&hl=en"
        );
        assert_eq!(
            page("https://www.bing.com/search?q={query}", "bn"),
            "https://www.bing.com/search?q=a%20swing&setlang=bn"
        );
        assert_eq!(
            page("https://www.bing.com/images/search?q={query}", "zh-CN"),
            "https://www.bing.com/images/search?q=a%20swing&setlang=zh-Hans"
        );
        assert_eq!(
            page("https://www.google.co.uk/search?q={query}", "en"),
            "https://www.google.co.uk/search?q=a%20swing&hl=en"
        );
    }

    #[test]
    fn other_pages_and_named_languages_are_left_alone() {
        // A language the address already names wins (a site the person added).
        assert_eq!(
            page("https://www.google.com/search?q={query}&hl=bn", "en"),
            "https://www.google.com/search?q=a%20swing&hl=bn"
        );
        // Translators have their own target; other sites have no such switch.
        for t in [
            "https://translate.google.com/?sl=auto&tl={lang}&text={query}&op=translate",
            "https://www.bing.com/translator?from=auto&to={lang}&text={query}",
            "https://duckduckgo.com/?q={query}",
            "https://en.wikipedia.org/w/index.php?search={query}",
            "https://notgoogle.com/?q={query}",
        ] {
            let plain = build_url(t, "a swing", "bn").unwrap().to_string();
            assert_eq!(page(t, "en"), plain, "{t}");
        }
        // A code that isn't one is never added.
        assert_eq!(
            page("https://www.google.com/search?q={query}", "en&x=1"),
            "https://www.google.com/search?q=a%20swing"
        );
    }

    #[test]
    fn the_web_language_setting() {
        assert_eq!(WebLanguage::default(), WebLanguage::Fixed("en".into()));
        assert_eq!(WebLanguage::Automatic.code("bn"), None);
        assert_eq!(WebLanguage::SameAsTranslation.code("bn"), Some("bn"));
        assert_eq!(WebLanguage::Fixed("fr".into()).code("bn"), Some("fr"));
        assert_eq!(
            WebLanguage::Fixed(" bn ".into()).sanitized(),
            WebLanguage::Fixed("bn".into())
        );
        assert_eq!(
            WebLanguage::Fixed("<b>".into()).sanitized(),
            WebLanguage::default()
        );
        let json = serde_json::to_string(&WebLanguage::Fixed("bn".into())).unwrap();
        assert_eq!(json, r#"{"mode":"fixed","code":"bn"}"#);
        assert_eq!(
            serde_json::from_str::<WebLanguage>(r#"{"mode":"automatic"}"#).unwrap(),
            WebLanguage::Automatic
        );
    }

    use super::*;

    #[test]
    fn older_lists_get_new_built_in_sources() {
        let mut old: Vec<WebSource> = defaults()
            .into_iter()
            .filter(|s| s.id != "bing-translator")
            .collect();
        old[0].enabled = false;
        sanitize(&mut old);
        let added = old.last().unwrap();
        assert_eq!(added.id, "bing-translator");
        assert!(!added.enabled);
        assert!(!old[0].enabled, "the person's choices stay");
        let again = old.clone();
        sanitize(&mut old);
        assert_eq!(old, again);
    }

    #[test]
    fn bing_gets_chinese_by_script() {
        let bing = "https://www.bing.com/translator?from=auto&to={lang}&text={query}";
        let url = build_url(bing, "hello", "zh-CN").unwrap();
        assert!(url.as_str().contains("to=zh-Hans&"), "{url}");
        let google = "https://translate.google.com/?sl=auto&tl={lang}&text={query}";
        assert!(
            build_url(google, "hello", "zh-CN")
                .unwrap()
                .as_str()
                .contains("tl=zh-CN&")
        );
        assert!(
            build_url(bing, "hello", "bn")
                .unwrap()
                .as_str()
                .contains("to=bn&")
        );
    }

    #[test]
    fn built_in_sources_are_valid_and_unique() {
        let all = defaults();
        for s in &all {
            assert_eq!(validate(&s.url), Ok(()), "{}", s.id);
            assert!(s.url.starts_with("https://"), "{} must use https", s.id);
            assert!(s.words || s.passages, "{} is never shown", s.id);
        }
        let mut copy = all.clone();
        sanitize(&mut copy);
        assert_eq!(copy, all);
    }

    #[test]
    fn fills_in_the_selection_and_language() {
        let url = build_url(
            "https://translate.google.com/?sl=auto&tl={lang}&text={query}",
            "Café & “rent” 5%",
            "bn",
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://translate.google.com/?sl=auto&tl=bn&text=Caf%C3%A9%20%26%20%E2%80%9Crent%E2%80%9D%205%25"
        );
        let pairs: Vec<_> = url.query_pairs().collect();
        assert_eq!(pairs[2].1, "Café & “rent” 5%");
    }

    #[test]
    fn the_selection_cant_escape_its_place() {
        // A selection with URL syntax stays one encoded value: no new parameters, no new path.
        let url = build_url(
            "https://example.com/s?q={query}&x=1",
            "a&x=2#frag/../../admin?y",
            "en",
        )
        .unwrap();
        let pairs: Vec<_> = url.query_pairs().collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].1, "a&x=2#frag/../../admin?y");
        assert_eq!(pairs[1].1, "1");
        assert_eq!(url.fragment(), None);
        assert_eq!(url.host_str(), Some("example.com"));

        let path = build_url("https://dict.example/{query}", "../../etc", "en").unwrap();
        assert_eq!(path.path(), "/..%2F..%2Fetc");
    }

    #[test]
    fn non_latin_selections_work() {
        let url = build_url(
            "https://bn.wikipedia.org/w/index.php?search={query}",
            "ভাড়া",
            "en",
        )
        .unwrap();
        assert_eq!(url.query_pairs().next().unwrap().1, "ভাড়া");
    }

    #[test]
    fn long_selections_are_cut_on_a_character_boundary() {
        let text = "ভা ".repeat(1000);
        let q = query_text(&text);
        assert!(q.chars().count() <= MAX_QUERY_CHARS);
        assert!(!q.ends_with(' '));
        assert_eq!(
            query_text("  two\n\nlines\tand  spaces "),
            "two lines and spaces"
        );
    }

    #[test]
    fn tabs_follow_the_selection_and_offline_only() {
        let all = defaults();
        let ids = |v: Vec<WebTab>| v.into_iter().map(|t| t.id).collect::<Vec<_>>();
        assert!(tabs_for(&all, true, true).is_empty());
        let words = ids(tabs_for(&all, false, true));
        assert_eq!(words[0], "google", "searches come first");
        assert!(words.contains(&"wikipedia".to_string()));
        let passages = ids(tabs_for(&all, false, false));
        assert!(!passages.contains(&"wikipedia".to_string()));
        assert!(passages.contains(&"translate".to_string()));
        assert!(!words.contains(&"cambridge".to_string()), "off by default");
    }

    #[test]
    fn rejects_bad_templates() {
        assert_eq!(validate("ftp://x.com/{query}"), Err(SourceError::Scheme));
        assert_eq!(
            validate("javascript:alert({query})"),
            Err(SourceError::Scheme)
        );
        assert_eq!(validate("file:///C:/{query}"), Err(SourceError::Scheme));
        assert_eq!(
            validate("https://example.com/search"),
            Err(SourceError::NoQuery)
        );
        assert_eq!(validate("https://{query}"), Err(SourceError::Invalid));
        assert_eq!(
            validate("https://{query}.example.com/"),
            Err(SourceError::Invalid)
        );
        assert_eq!(validate("https://user:{query}@example.com/"), Ok(()));
        assert_eq!(
            validate("https:// bad host/{query}"),
            Err(SourceError::Invalid)
        );
        assert_eq!(validate("  HTTPS://Example.com/?q={query}  "), Ok(()));
        assert!(build_url("https://example.com", "x", "en").is_err());
    }

    #[test]
    fn sanitize_fixes_what_a_file_can_break() {
        let mut list = vec![
            source("a", "  ", "https://a.example/?q={query}", true, true, true),
            source(
                "a",
                "Duplicate",
                "https://b.example/?q={query}",
                true,
                true,
                true,
            ),
            source(
                "",
                "No id",
                "https://c.example/?q={query}",
                true,
                true,
                true,
            ),
            source("bad", "Bad", "notaurl", true, true, true),
            source(
                "long",
                &"x".repeat(100),
                "https://d.example/?q={query}",
                true,
                true,
                true,
            ),
        ];
        sanitize(&mut list);
        let ids: Vec<_> = list.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids[..3], ["a", "bad", "long"]);
        // The built-in sources the file lacked follow.
        assert_eq!(ids.len(), 3 + defaults().len());
        assert_eq!(list[0].name, "a");
        assert!(!list[1].enabled);
        assert_eq!(list[2].name.chars().count(), 40);

        let mut many: Vec<_> = (0..40)
            .map(|i| {
                source(
                    &format!("s{i}"),
                    "S",
                    "https://e.example/?q={query}",
                    true,
                    true,
                    true,
                )
            })
            .collect();
        sanitize(&mut many);
        assert_eq!(many.len(), MAX_SOURCES);
    }
}
