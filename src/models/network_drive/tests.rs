//! Purpose: Invalid network fields provide readable repair hints without echoing input.

use super::*;

#[test]
fn rejected_network_fields_explain_how_to_repair_them() {
    let errors = [
        (
            MountPoint::try_from("relative".to_owned()).unwrap_err(),
            "absolute path",
        ),
        (
            NetworkAddress::try_from(" \n".to_owned()).unwrap_err(),
            "control characters",
        ),
        (
            NetworkAddress::try_from("name:@fixture.test".to_owned()).unwrap_err(),
            "credentials",
        ),
    ];
    for (error, hint) in errors {
        let message = error.to_string();
        assert!(message.contains(hint), "{message}");
        assert!(!message.chars().any(char::is_control), "{message}");
        assert!(
            !message.contains("fixture.test"),
            "rejected input must stay private"
        );
    }
}
