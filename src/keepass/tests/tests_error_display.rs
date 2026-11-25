//! Purpose: `KeepassError` displays fixed text for each variant.

#[test]
fn keepass_error_display_is_fixed_text_per_variant() {
    use super::KeepassError;
    let io_error = KeepassError::Io(std::io::Error::other("disk gone"));
    let cases: &[(KeepassError, &str)] = &[
        (
            KeepassError::Open("bad header".into()),
            "failed to open kdbx: bad header",
        ),
        (
            KeepassError::Save("read-only".into()),
            "failed to save kdbx: read-only",
        ),
        (io_error, "kdbx I/O error: disk gone"),
        (KeepassError::EntryNotFound, "entry not found"),
        (KeepassError::AlreadyExists, "destination already exists"),
        (
            KeepassError::Serialize("bad utf8".into()),
            "failed to serialize export: bad utf8",
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), *expected);
    }
}
