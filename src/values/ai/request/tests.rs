//! Purpose: Unit tests for `AiError` display formatting across all variants.

use super::AiError;

#[test]
fn ai_error_display_formats_every_variant() {
    assert!(AiError::Disabled.to_string().contains("Unavailable"));
    assert_eq!(
        AiError::MissingKey("missing".to_owned()).to_string(),
        "missing"
    );
    assert_eq!(AiError::Config("bad cfg".to_owned()).to_string(), "bad cfg");
    assert_eq!(AiError::Network("down".to_owned()).to_string(), "down");
    assert_eq!(
        AiError::InvalidResponse("bad json".to_owned()).to_string(),
        "bad json"
    );
    assert_eq!(
        AiError::Http(503, "busy".to_owned()).to_string(),
        "AI HTTP 503: busy"
    );
    assert_eq!(
        AiError::Worker.to_string(),
        "AI worker stopped unexpectedly"
    );
}
