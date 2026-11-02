use super::*;

#[test]
fn invalid_bookmark_names_are_refused_before_storage_is_opened() {
    let root = tempfile::tempdir().unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    for name in [String::new(), "x".repeat(257), "line\nbreak".into()] {
        let error = server
            .set_bookmark(Parameters(BookmarkParams {
                name,
                path: Some(".".into()),
            }))
            .unwrap_err();
        assert_eq!(error.code, rmcp::model::ErrorCode::INVALID_PARAMS);
        assert_eq!(error.message, "invalid bookmark name");
    }
    drop(server);
}

#[cfg(unix)]
fn home(root: &tempfile::TempDir) {
    std::env::set_var("HOME", root.path());
    std::env::set_var("USERPROFILE", root.path());
}

#[cfg(unix)]
#[test]
fn full_bookmark_storage_allows_updates_but_refuses_a_new_name() {
    crate::tests::faults::run(|| {
        let root = tempfile::tempdir().unwrap();
        home(&root);
        let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
        let bookmarks: serde_json::Map<_, _> = (0..1000)
            .map(|index| {
                (
                    format!("name{index}"),
                    json!(root.path().display().to_string()),
                )
            })
            .collect();
        save_bookmarks(&bookmarks).unwrap();
        server
            .set_bookmark(Parameters(BookmarkParams {
                name: "name0".into(),
                path: Some(".".into()),
            }))
            .unwrap();
        let error = server
            .set_bookmark(Parameters(BookmarkParams {
                name: "extra".into(),
                path: Some(".".into()),
            }))
            .unwrap_err();
        drop(server);
        assert_eq!(error.message, "maximum of 1000 bookmarks reached");
        assert_eq!(load_bookmarks().len(), 1000);
    });
}

#[cfg(unix)]
#[test]
fn bookmark_storage_failures_do_not_look_like_successful_writes() {
    crate::tests::faults::run(|| {
        let root = tempfile::tempdir().unwrap();
        home(&root);
        fs::write(root.path().join(".ffm"), b"existing file").unwrap();
        let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
        let error = server
            .set_bookmark(Parameters(BookmarkParams {
                name: "work".into(),
                path: Some(".".into()),
            }))
            .unwrap_err();
        drop(server);
        assert_eq!(error.code, rmcp::model::ErrorCode::INTERNAL_ERROR);
        assert_eq!(
            fs::read(root.path().join(".ffm")).unwrap(),
            b"existing file"
        );
        std::env::remove_var("HOME");
        std::env::remove_var("USERPROFILE");
        assert_eq!(
            save_bookmarks(&serde_json::Map::new()).unwrap_err(),
            "cannot determine home directory"
        );
    });
}
