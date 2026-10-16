//! The Microsoft Edge WebView2 runtime that draws the app's windows. An old one (a PC
//! behind on Windows Update) makes the app look half broken, so the app checks it at
//! start and asks for an update instead (the installer also updates it when it's older
//! than this: `minimumWebview2Version` in `tauri.conf.json`, keep the two the same).

/// The oldest WebView2 the app runs on (February 2023).
pub const MIN_VERSION: &str = "110.0.1587.40";

/// Whether `version` ("120.0.2210.91") is older than [`MIN_VERSION`]. A version that can't
/// be read counts as too old: the app can't tell it will work.
pub fn too_old(version: &str) -> bool {
    match (parse(version), parse(MIN_VERSION)) {
        (Some(found), Some(min)) => found < min,
        _ => true,
    }
}

fn parse(version: &str) -> Option<Vec<u32>> {
    let parts: Vec<u32> = version
        .trim()
        .split('.')
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    (!parts.is_empty()).then_some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_compared_part_by_part() {
        assert!(!too_old("120.0.2210.91"));
        assert!(!too_old(MIN_VERSION));
        assert!(!too_old("110.0.1587.41"));
        assert!(!too_old("110.1"));
        assert!(too_old("110.0.1587.39"));
        assert!(too_old("109.0.1518.140"));
        // 9 is older than 110, even though "9" sorts after "1" as text.
        assert!(too_old("96.0.1054.62"));
    }

    #[test]
    fn unreadable_versions_count_as_too_old() {
        assert!(too_old(""));
        assert!(too_old("not a version"));
        assert!(too_old("120.x.1"));
    }
}
