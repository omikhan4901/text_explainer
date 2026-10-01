//! Checks against the real bundled dictionary, when it has been built
//! (`python scripts/build-dictionary.py`; check.sh and CI build it).

use std::path::PathBuf;
use std::time::Instant;

use te_core::dictionary::Dictionary;

fn real() -> Option<Dictionary> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../src-tauri/resources/dictionary.sqlite");
    path.exists()
        .then(|| Dictionary::open(&path).expect("open dictionary"))
}

#[test]
fn common_lookups_work_and_are_fast() {
    let Some(d) = real() else {
        eprintln!("dictionary not built; skipping");
        return;
    };
    for (word, lemma) in [
        ("ubiquitous", "ubiquitous"),
        ("Mice", "mouse"),
        ("went", "go"),
        ("indemnify", "indemnify"),
        ("notwithstanding", "notwithstanding"),
        ("habeas corpus", "habeas corpus"),
        ("studies", "study"),
    ] {
        let e = d.lookup(word).unwrap_or_else(|| panic!("{word} not found"));
        assert_eq!(e.lemma, lemma, "{word}");
        assert!(!e.senses.is_empty());
    }
    // British spellings resolve (directly or through the American form).
    for word in ["anaemia", "colour", "organised", "haematology", "centre"] {
        assert!(d.lookup(word).is_some(), "{word} not found");
    }
    // Capitalised contract terms are ordinary words; places and companies aren't.
    for word in [
        "Lessee",
        "Agreement",
        "Work",
        "Goods",
        "Supplier",
        "Schedule",
    ] {
        assert!(d.is_common_word(word), "{word} should be a common word");
    }
    for word in ["Cambridge", "England", "Wednesday"] {
        assert!(!d.is_common_word(word), "{word} should be a proper noun");
    }
    // Budget: an entry in under 50 ms (the card shows it before the model answers).
    let start = Instant::now();
    for _ in 0..100 {
        let _ = d.lookup("ubiquitousness");
        let _ = d.lookup("running");
    }
    let per_lookup = start.elapsed() / 200;
    assert!(per_lookup.as_millis() < 50, "lookup took {per_lookup:?}");
    eprintln!("lookup: {per_lookup:?}");
}
