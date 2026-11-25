use super::*;
use crate::tests::tool_json;

#[test]
fn permissions_toggles_readonly_flag_across_platforms() {
    let root = tempfile::tempdir().unwrap();
    let canon = root.path().canonicalize().unwrap();
    let file = canon.join("item.txt");
    fs::write(&file, b"content").unwrap();
    let server = FfmServer::with_config(canon, false, false);

    let set_ro = server
        .permissions(Parameters(PermissionsParams {
            path: "item.txt".into(),
            mode: None,
            readonly: Some(true),
        }))
        .unwrap();
    assert_eq!(tool_json(&set_ro)["readonly"], true);

    let clear_ro = server
        .permissions(Parameters(PermissionsParams {
            path: "item.txt".into(),
            mode: None,
            readonly: Some(false),
        }))
        .unwrap();
    assert_eq!(tool_json(&clear_ro)["readonly"], false);
    drop(server);
}
