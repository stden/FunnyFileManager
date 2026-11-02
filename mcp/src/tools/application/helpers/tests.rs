use super::*;
use rmcp::model::ErrorCode;

fn check(error: automation::Error, code: ErrorCode, message: &str) {
    assert_eq!(error.to_string(), message);
    let actual = automation_error(error);
    assert_eq!(actual.code, code);
    assert_eq!(actual.message, message);
}

#[test]
fn application_refusals_keep_the_invalid_parameter_code() {
    use automation::Error;
    check(
        Error::Invalid("bad command"),
        ErrorCode::INVALID_PARAMS,
        "bad command",
    );
    check(
        Error::UnknownSession,
        ErrorCode::INVALID_PARAMS,
        "unknown or closed FFM session",
    );
    check(
        Error::SessionLimit,
        ErrorCode::INVALID_PARAMS,
        "maximum of four FFM sessions reached",
    );
    check(
        Error::Application {
            code: "busy".into(),
            message: "wait".into(),
        },
        ErrorCode::INVALID_PARAMS,
        "busy: wait",
    );
}

#[test]
fn transport_failures_keep_the_internal_error_code_and_recovery_message() {
    use automation::Error;
    check(
        Error::Io(std::io::Error::other("broken pipe")),
        ErrorCode::INTERNAL_ERROR,
        "FFM automation transport: broken pipe",
    );
    check(
        Error::Protocol("bad frame"),
        ErrorCode::INTERNAL_ERROR,
        "bad frame",
    );
    check(
        Error::Timeout,
        ErrorCode::INTERNAL_ERROR,
        "FFM automation response timed out; session closed",
    );
}

#[test]
fn individual_key_and_text_limits_are_checked_at_the_boundary() {
    let mut params: InputParams = serde_json::from_value(serde_json::json!({"session_id":"test","keys":["x".repeat(128)],"text":"x".repeat(1024*1024)})).unwrap();
    assert!(validate_input(&params).is_ok());
    params.keys[0].push('x');
    assert_eq!(
        validate_input(&params).unwrap_err().code,
        ErrorCode::INVALID_PARAMS
    );
    params.keys.clear();
    params.text.as_mut().unwrap().push('x');
    assert_eq!(
        validate_input(&params).unwrap_err().code,
        ErrorCode::INVALID_PARAMS
    );
}
