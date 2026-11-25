//! Unit tests for duplicate extension deduplication in `owners`.

use super::*;
use crate::config::ServerSettings;

#[test]
fn owners_deduplicates_language_across_repeated_extensions() {
    let mut map = BTreeMap::new();
    map.insert(
        "rust".to_owned(),
        ServerSettings {
            extensions: vec![".rs".to_owned(), "rs".to_owned()],
            ..ServerSettings::default()
        },
    );
    let draft = LspSettings(map);
    let claimed = owners(&draft);
    assert_eq!(claimed.get("rs"), Some(&vec!["rust".to_owned()]));

    let vocabulary = Vocabulary {
        catalog: &draft,
        draft: &draft,
        programs: &[],
    };
    let rows = extensions("", &BTreeSet::new(), &vocabulary);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].hint.as_deref(), Some("rust"));
}
