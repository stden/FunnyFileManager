//! Purpose: A Windows job object that owns a child's process tree and kills
//! it when the handle closes.

use std::io;

/// Own a child's process tree: closing the returned handle kills every
/// process in it, so a crashed or unresponsive LSP server, or a CLI stopped
/// at its time limit, cannot orphan children.
pub fn process_tree_job(
    child: &std::process::Child,
) -> io::Result<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::AsRawHandle;
    job_for_process(child.as_raw_handle())
}

/// Own a private PTY child's descendants without transferring its process handle.
pub fn private_terminal_job(
    child: &dyn portable_pty::Child,
) -> io::Result<std::os::windows::io::OwnedHandle> {
    let handle = child
        .as_raw_handle()
        .ok_or_else(|| io::Error::other("Private process has no handle"))?;
    job_for_process(handle)
}

fn job_for_process(
    process: std::os::windows::io::RawHandle,
) -> io::Result<std::os::windows::io::OwnedHandle> {
    job_for_raw_handle_with(process, create_anonymous_job)
}

#[allow(
    unsafe_code,
    reason = "Windows process trees require owned job objects"
)]
pub(super) fn create_anonymous_job() -> windows_sys::Win32::Foundation::HANDLE {
    use windows_sys::Win32::System::JobObjects::CreateJobObjectW;
    // SAFETY: null security/name create a private, non-inherited job handle.
    unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) }
}

#[allow(
    unsafe_code,
    reason = "Windows process trees require owned job objects"
)]
pub(super) fn job_for_raw_handle_with(
    process: std::os::windows::io::RawHandle,
    create_job: fn() -> windows_sys::Win32::Foundation::HANDLE,
) -> io::Result<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::{FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    let raw = create_job();
    if raw.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `create_job` returned a unique live handle; ownership transfers once.
    let job = unsafe { OwnedHandle::from_raw_handle(raw) };
    set_job_limit_flags(raw, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE)?;
    // SAFETY: `raw` is a live job handle; Win32 rejects invalid `process` handles cleanly.
    if unsafe { AssignProcessToJobObject(raw, process) } == 0 {
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
pub(super) fn set_job_limit_flags(
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
