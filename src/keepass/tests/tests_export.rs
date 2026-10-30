//! Purpose: structured export: JSON and YAML round trips, nested groups and expiry reporting.

use super::*;
use tempfile::TempDir;

#[test]
fn export_to_json_and_yaml_includes_all_fields() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");

    save_custom_field_vault(&path);

    // Structured export captures every field, including the custom one and
    // the plaintext password.
    let exported = export(&path, "pw", None).expect("export");
    assert_eq!(exported.database_name.as_deref(), Some("My Vault"));
    assert_eq!(exported.root.entries.len(), 1);
    let entry = &exported.root.entries[0];
    assert_eq!(
        entry.fields.get("Title").map(String::as_str),
        Some("GitHub")
    );
    assert_eq!(
        entry.fields.get("Password").map(String::as_str),
        Some("s3cr3t!")
    );
    assert_eq!(
        entry.fields.get("API-Token").map(String::as_str),
        Some("custom-value")
    );

    // JSON round-trips back to the identical structure.
    let json = export_json(&path, "pw", None).expect("json");
    assert!(json.contains("\"API-Token\"") && json.contains("s3cr3t!"));
    let from_json: KdbxExport = serde_json::from_str(&json).unwrap();
    assert_eq!(from_json, exported);

    // YAML round-trips back to the identical structure.
    let yaml = export_yaml(&path, "pw", None).expect("yaml");
    assert!(yaml.contains("API-Token"));
    let from_yaml: KdbxExport = serde_norway::from_str(&yaml).unwrap();
    assert_eq!(from_yaml, exported);
}

/// The export must mirror the real group tree, not just the root group.
#[test]
fn export_includes_nested_groups() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("v.kdbx");

    let mut db = cheap_db();
    {
        let mut root = db.root_mut();
        let mut work = root.add_group();
        work.name = "Work".to_owned();
        write_entry_fields(&mut work.add_entry(), &entry("Deploy", "", "", "", "p"));
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, &path, key).expect("save");

    let exported = export(&path, "pw", None).expect("export");
    assert_eq!(exported.root.entries.len(), 0);
    assert_eq!(exported.root.groups.len(), 1);
    assert_eq!(exported.root.groups[0].name, "Work");
    assert_eq!(exported.root.groups[0].entries.len(), 1);
}

#[test]
fn export_times_reports_expiry_only_when_the_flag_is_set() {
    let mut times = Times::default();
    times.expires = Some(false);
    times.expiry = times.creation;
    assert_eq!(
        export_times(&times).expires,
        None,
        "an expiry date without the flag means 'does not expire'"
    );

    times.expires = Some(true);
    let exported = export_times(&times);
    assert_eq!(
        exported.expires.is_some(),
        times.expiry.is_some(),
        "with the flag set the expiry timestamp is passed through"
    );
}

fn save_custom_field_vault(path: &Path) {
    let mut db = cheap_db();
    db.meta.database_name = Some("My Vault".to_owned());
    {
        let mut root = db.root_mut();
        let mut e = root.add_entry();
        e.set_unprotected(fields::TITLE, "GitHub");
        e.set_unprotected(fields::USERNAME, "alice");
        e.set_protected(fields::PASSWORD, "s3cr3t!");
        e.set_unprotected("API-Token", "custom-value"); // a non-standard field
    }
    let key = build_key("pw", None).expect("key");
    save_atomic(&db, path, key).expect("save");
}
