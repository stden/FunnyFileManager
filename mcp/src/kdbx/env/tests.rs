use super::*;

#[test]
fn passwordless_entries_are_skipped_and_untitled_passwords_use_their_uuid() {
    let mut database = Database::new();
    let id = {
        let mut root = database.root_mut();
        root.add_entry()
            .set_unprotected(fields::TITLE, "no-password");
        let mut entry = root.add_entry();
        entry.set_protected(fields::PASSWORD, "fixture");
        entry.set_unprotected(fields::TITLE, "");
        entry.id().uuid().to_string()
    };
    let result = export_passwords_as_env(&database);
    assert_eq!(
        result,
        format!("KDBX_{}_PASSWORD=\"fixture\"", env_name(&id))
    );
    assert!(!result.contains("NO_PASSWORD"));
}

#[test]
fn names_trim_trailing_separators_and_try_past_occupied_suffixes() {
    assert_eq!(env_name("...title---"), "TITLE");
    let mut names = HashSet::new();
    assert_eq!(unique_env_name("API", &mut names), "KDBX_API_PASSWORD");
    assert_eq!(unique_env_name("API", &mut names), "KDBX_API_2_PASSWORD");
    assert_eq!(unique_env_name("API", &mut names), "KDBX_API_3_PASSWORD");
    assert_eq!(quote_env_value("carriage\rreturn"), "\"carriage\\rreturn\"");
}
