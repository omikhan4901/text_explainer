//! App rules: apps where the card never reads a selection ("Don't open in these apps"),
//! such as password managers. An app is named by its program file (`keepass.exe`), the
//! way Windows reports the foreground process; names are compared without case.

/// Most apps the list keeps.
pub const MAX_APPS: usize = 50;
/// Longest program file name accepted, in characters.
pub const MAX_NAME_CHARS: usize = 64;

/// The apps on the list until the person changes it: password managers.
pub fn defaults() -> Vec<String> {
    [
        "keepass.exe",
        "keepassxc.exe",
        "1password.exe",
        "bitwarden.exe",
    ]
    .map(String::from)
    .to_vec()
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuleError {
    #[error("type the app's program name, such as notepad.exe")]
    Empty,
    #[error("the program name must end in .exe")]
    NotExe,
    #[error("the program name is too long")]
    TooLong,
    #[error("the program name has characters a file name can't have")]
    BadCharacters,
}

/// The program file name as the list keeps it: trimmed, lowercase, without its folder.
/// `C:\Program Files\KeePass\KeePass.EXE` → `keepass.exe`.
pub fn normalize(name: &str) -> Result<String, RuleError> {
    let file = name
        .trim()
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_default()
        .trim();
    if file.is_empty() {
        return Err(RuleError::Empty);
    }
    if file
        .chars()
        .any(|c| c.is_control() || r#"<>:"|?*"#.contains(c))
    {
        return Err(RuleError::BadCharacters);
    }
    let lower = file.to_lowercase();
    let Some(stem) = lower.strip_suffix(".exe") else {
        return Err(RuleError::NotExe);
    };
    if stem.trim_matches(['.', ' ']).is_empty() {
        return Err(RuleError::NotExe);
    }
    if lower.chars().count() > MAX_NAME_CHARS {
        return Err(RuleError::TooLong);
    }
    Ok(lower)
}

/// Whether the card stays closed in `exe` (a program file name or full path).
pub fn is_excluded(list: &[String], exe: &str) -> bool {
    let Ok(exe) = normalize(exe) else {
        return false;
    };
    list.iter()
        .any(|entry| normalize(entry).is_ok_and(|e| e == exe))
}

/// The list as settings keep it: valid names only, normalized, each once, in the order
/// given, at most [`MAX_APPS`].
pub fn sanitize(list: Vec<String>) -> Vec<String> {
    let mut clean: Vec<String> = Vec::with_capacity(list.len().min(MAX_APPS));
    for name in list.iter().filter_map(|n| normalize(n).ok()) {
        if clean.len() == MAX_APPS {
            break;
        }
        if !clean.contains(&name) {
            clean.push(name);
        }
    }
    clean
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_lowercase_file_names() {
        assert_eq!(normalize("KeePass.exe").unwrap(), "keepass.exe");
        assert_eq!(normalize("  notepad.EXE \n").unwrap(), "notepad.exe");
        assert_eq!(normalize("Code.Exe").unwrap(), "code.exe");
        assert_eq!(
            normalize(r"C:\Program Files\KeePassXC\KeePassXC.exe").unwrap(),
            "keepassxc.exe"
        );
        assert_eq!(normalize("/opt/apps/Thing.exe").unwrap(), "thing.exe");
        assert_eq!(normalize(r"\\server\share\Tool.exe").unwrap(), "tool.exe");
        assert_eq!(normalize("my app.exe").unwrap(), "my app.exe");
        assert_eq!(normalize("app.v2.exe").unwrap(), "app.v2.exe");
    }

    #[test]
    fn unicode_names_are_lowercased() {
        assert_eq!(normalize("Блокнот.EXE").unwrap(), "блокнот.exe");
        assert_eq!(normalize("ÉDITEUR.exe").unwrap(), "éditeur.exe");
        assert_eq!(normalize("বাংলা.exe").unwrap(), "বাংলা.exe");
    }

    #[test]
    fn bad_names_are_refused() {
        assert_eq!(normalize(""), Err(RuleError::Empty));
        assert_eq!(normalize("   "), Err(RuleError::Empty));
        assert_eq!(normalize(r"C:\Apps\"), Err(RuleError::Empty));
        assert_eq!(normalize("notepad"), Err(RuleError::NotExe));
        assert_eq!(normalize("notepad.exe.txt"), Err(RuleError::NotExe));
        assert_eq!(normalize("notepad.ex"), Err(RuleError::NotExe));
        assert_eq!(normalize(".exe"), Err(RuleError::NotExe));
        assert_eq!(normalize("..exe"), Err(RuleError::NotExe));
        assert_eq!(normalize(" .exe"), Err(RuleError::NotExe));
        assert_eq!(normalize("bad|name.exe"), Err(RuleError::BadCharacters));
        assert_eq!(normalize("what?.exe"), Err(RuleError::BadCharacters));
        assert_eq!(normalize("tab\there.exe"), Err(RuleError::BadCharacters));
        assert_eq!(normalize("C:notepad.exe"), Err(RuleError::BadCharacters));
    }

    #[test]
    fn names_are_at_most_64_characters() {
        let longest = format!("{}.exe", "a".repeat(60));
        assert_eq!(normalize(&longest).unwrap(), longest);
        assert_eq!(
            normalize(&format!("{}.exe", "a".repeat(61))),
            Err(RuleError::TooLong)
        );
        // Characters, not bytes: 60 two-byte letters still fit.
        let cyrillic = format!("{}.exe", "ж".repeat(60));
        assert_eq!(normalize(&cyrillic).unwrap(), cyrillic);
    }

    #[test]
    fn matching_ignores_case_and_folders() {
        let list = defaults();
        assert!(is_excluded(&list, "KeePass.exe"));
        assert!(is_excluded(
            &list,
            r"C:\Program Files\Bitwarden\Bitwarden.EXE"
        ));
        assert!(is_excluded(&list, "1PASSWORD.exe"));
        assert!(!is_excluded(&list, "notepad.exe"));
        assert!(!is_excluded(&list, "keepass"));
        assert!(!is_excluded(&list, "keepass.exe.bak"));
        assert!(!is_excluded(&list, ""));
        assert!(!is_excluded(&[], "keepass.exe"));
        // Entries written by hand in the settings file match too.
        assert!(!is_excluded(&["  Word.EXE ".into()], "WINWORD.exe"));
        assert!(is_excluded(&["  WinWord.EXE ".into()], "winword.exe"));
    }

    #[test]
    fn junk_entries_never_match() {
        let list = vec!["".into(), "notepad".into(), "*".into()];
        assert!(!is_excluded(&list, "notepad.exe"));
        assert!(!is_excluded(&list, ".exe"));
    }

    #[test]
    fn sanitize_cleans_dedupes_and_caps() {
        let list = vec![
            "KeePass.exe".into(),
            "keepass.exe".into(),
            "".into(),
            "notes".into(),
            r"C:\Tools\KEEPASS.EXE".into(),
            "Code.exe".into(),
        ];
        assert_eq!(sanitize(list), vec!["keepass.exe", "code.exe"]);
        assert_eq!(sanitize(Vec::new()), Vec::<String>::new());
        assert_eq!(sanitize(defaults()), defaults());

        let many: Vec<String> = (0..80).map(|i| format!("app{i}.exe")).collect();
        let kept = sanitize(many);
        assert_eq!(kept.len(), MAX_APPS);
        assert_eq!(kept[0], "app0.exe");
        assert_eq!(kept[MAX_APPS - 1], "app49.exe");

        // Duplicates don't use up the cap.
        let mut repeated: Vec<String> = vec!["same.exe".into(); 60];
        repeated.push("other.exe".into());
        assert_eq!(sanitize(repeated), vec!["same.exe", "other.exe"]);
    }
}
