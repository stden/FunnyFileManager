//! The FFI rejects null handles and catalogs, in the native shim and the stubs alike.

#[test]
fn ffi_rejects_missing_handles_and_catalogs() {
    // SAFETY: A null catalog is explicitly accepted by both the shim and stubs.
    let opened = unsafe { colorer_sys::colorer_open(std::ptr::null()) };
    assert!(opened.is_null());
    // SAFETY: Closing a null handle is a no-op in both implementations.
    unsafe { colorer_sys::colorer_close(std::ptr::null_mut()) };
    // SAFETY: Both implementations reject a null handle before reading other arguments.
    let parsed = unsafe {
        colorer_sys::colorer_parse(
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            None,
            std::ptr::null_mut(),
        )
    };
    assert_eq!(parsed, -1);
    // SAFETY: Both implementations return null without dereferencing a null handle.
    let file_type =
        unsafe { colorer_sys::colorer_file_type(std::ptr::null_mut(), std::ptr::null()) };
    assert!(file_type.is_null());
}
