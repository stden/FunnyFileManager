//! Purpose: Negotiate honest capabilities and all supported synchronization modes.

use super::*;

#[test]
fn feature_flags_accept_provider_options_and_reject_missing_support() {
    let capabilities = Capabilities::parse(&json!({
        "completionProvider": {}, "hoverProvider": true,
        "definitionProvider": false, "textDocumentSync": 1
    }))
    .unwrap();
    assert!(capabilities.supports(Feature::Completion));
    assert!(capabilities.supports(Feature::Hover));
    assert!(!capabilities.supports(Feature::Definition));
    assert!(Capabilities::parse(&Value::Null).is_err());
    let disabled = Capabilities::parse(
        &json!({"textDocumentSync":{"openClose":false,"change":0,"save":false}}),
    )
    .unwrap();
    assert!(!disabled.open_close);
    assert!(disabled.save.is_none());
}
