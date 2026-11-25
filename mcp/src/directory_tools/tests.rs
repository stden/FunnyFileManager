use super::*;

#[test]
fn list_drives_returns_consistent_local_entries_across_repeated_calls() {
    let first = crate::tests::tool_json(&list_drives());
    let second = crate::tests::tool_json(&list_drives());
    assert_eq!(first["drives"], second["drives"]);
    assert_ne!(first["drives"].as_array().unwrap().len(), 0);

    #[cfg(target_os = "windows")]
    {
        let refreshed = cached_windows_drives(std::time::Duration::ZERO);
        let warm = cached_windows_drives(std::time::Duration::from_secs(60));
        assert_eq!(refreshed, warm);
        assert_ne!(warm.len(), 0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut mounts = Vec::new();
        append_unix_mounts(&mut mounts, &["/", "/no_such_ffm_mount_dir"]);
        assert_eq!(mounts, vec![json!({ "type": "mount", "path": "/" })]);
    }
}
