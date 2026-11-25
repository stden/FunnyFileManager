//! Unit tests for `merge_onto` error handling when base defaults are not an object.

use super::merge_onto;

#[test]
fn merge_onto_rejects_non_object_base() {
    let err = merge_onto(serde_json::Value::Null, serde_json::Map::new())
        .expect_err("non-object base must fail")
        .to_string();
    assert!(
        err.contains("LSP server defaults must be an object"),
        "{err}"
    );
}
