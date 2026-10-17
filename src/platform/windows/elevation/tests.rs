use super::*;

#[test]
fn shell_execute_info_suppresses_blocking_shell_error_dialogs() {
    let verb = [0u16];
    let file = [0u16];
    let args = [0u16];
    let info = shell_execute_info(&verb, &file, &args);
    assert_ne!(info.fMask & SEE_MASK_FLAG_NO_UI, 0);
    assert_ne!(info.fMask & SEE_MASK_NOCLOSEPROCESS, 0);
    assert_ne!(info.fMask & SEE_MASK_NOASYNC, 0);
    assert_eq!(info.nShow, SW_HIDE);
}
