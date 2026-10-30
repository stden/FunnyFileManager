//! Purpose: A Windows job object that owns a child's process tree and kills
//! it when the handle closes.

use std::io;

/// Own a child's process tree: closing the returned handle kills every
/// process in it, so a crashed or unresponsive LSP server, or a CLI stopped
/// at its time limit, cannot orphan children.
#[allow(
    unsafe_code,
    reason = "std has no job objects, the only way to end a process tree with its handle"
)]
pub fn process_tree_job(
    child: &std::process::Child,
) -> io::Result<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    // SAFETY: null security/name create a private, non-inherited job handle.
    let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: CreateJobObjectW returned a unique live handle; ownership transfers once.
    let job = unsafe { OwnedHandle::from_raw_handle(raw) };
    set_job_limit_flags(raw, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE)?;
    // SAFETY: both handles remain live throughout the call; the child is owned by the caller.
    if unsafe { AssignProcessToJobObject(raw, child.as_raw_handle()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(job)
}

/// Clear `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` before dropping the job handle so
/// intentionally backgrounded processes survive after their shell exits cleanly.
pub fn disarm_kill_on_close(job: &std::os::windows::io::OwnedHandle) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    set_job_limit_flags(job.as_raw_handle(), 0)
}

#[allow(
    unsafe_code,
    reason = "std has no job objects, the only way to end a process tree with its handle"
)]
fn set_job_limit_flags(
    handle: windows_sys::Win32::Foundation::HANDLE,
    flags: u32,
) -> io::Result<()> {
    use windows_sys::Win32::System::JobObjects::{
        JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = flags;
    let size = u32::try_from(size_of_val(&limits)).map_err(io::Error::other)?;
    // SAFETY: limits points to an initialized structure of the supplied size for this class.
    let result = unsafe {
        SetInformationJobObject(
            handle,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&limits).cast(),
            size,
        )
    };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
