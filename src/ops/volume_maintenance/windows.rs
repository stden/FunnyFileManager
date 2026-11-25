//! Choose only mounted local disks and decode the report from the system executable.

use super::{command, report, verify_target, VolumeOperation, VolumeTarget};
use crate::{diskfree, encoding::TextEncoding, platform};
use std::{
    borrow::Cow,
    ffi::OsString,
    io,
    os::windows::ffi::{OsStrExt, OsStringExt},
    os::windows::process::CommandExt,
    path::{Component, Path, PathBuf, Prefix},
    process::{Command, Output},
};
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

#[cfg(test)]
thread_local! {
    static PROGRAM_OVERRIDE: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

fn local_path(path: &Path) -> bool {
    path.is_absolute()
        && matches!(path.components().next(),
        Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
}

pub fn resolve(path: &Path) -> io::Result<VolumeTarget> {
    resolve_with(
        path,
        platform::volume_mount_point,
        diskfree::drive_kind,
        platform::volume_id,
    )
}

fn resolve_with(
    path: &Path,
    mount_point: impl FnOnce(&Path) -> io::Result<PathBuf>,
    drive_kind: impl FnOnce(&Path) -> diskfree::DriveKind,
    volume_id: impl FnOnce(&Path) -> io::Result<OsString>,
) -> io::Result<VolumeTarget> {
    if !local_path(path) || !std::fs::metadata(path)?.is_dir() {
        return Err(io::Error::other(
            "Choose a directory on a local Windows drive",
        ));
    }
    let mount = mount_point(path)?;
    if !local_path(&mount)
        || !matches!(
            drive_kind(&mount),
            diskfree::DriveKind::Fixed | diskfree::DriveKind::Removable
        )
    {
        return Err(io::Error::other(
            "Volume maintenance requires a local fixed or removable disk",
        ));
    }
    let id = volume_id(&mount)?;
    Ok(VolumeTarget { mount, id })
}

fn select_program(default: PathBuf) -> PathBuf {
    #[cfg(test)]
    if let Some(program) = PROGRAM_OVERRIDE.with(|slot| slot.borrow().clone()) {
        return program;
    }
    default
}

pub fn run(operation: VolumeOperation, target: &VolumeTarget) -> io::Result<String> {
    let program = select_program(platform::system_directory()?.join("defrag.exe"));
    execute(
        operation,
        target,
        &resolve(&target.mount)?,
        &program,
        Command::output,
    )
}

pub(super) fn execute(
    operation: VolumeOperation,
    target: &VolumeTarget,
    current: &VolumeTarget,
    program: &Path,
    launch: impl FnOnce(&mut Command) -> io::Result<Output>,
) -> io::Result<String> {
    verify_target(target, current)?;
    let mount = utility_mount(&target.mount);
    let output = launch(command(program, operation, &mount).creation_flags(CREATE_NO_WINDOW))?;
    report(
        output.status.success(),
        &output.status.to_string(),
        &decode(&output.stdout),
        &decode(&output.stderr),
    )
}

fn utility_mount(mount: &Path) -> Cow<'_, Path> {
    if matches!(mount.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::VerbatimDisk(_)))
    {
        // defrag.exe accepts a DOS mount point. Keep the original UTF-16 name and trailing slash.
        let units = mount.as_os_str().encode_wide().skip(4).collect::<Vec<_>>();
        Cow::Owned(OsString::from_wide(&units).into())
    } else {
        Cow::Borrowed(mount)
    }
}

fn decode(bytes: &[u8]) -> String {
    let detected = TextEncoding::detect(bytes);
    let bomless_wide = bytes.len().is_multiple_of(2)
        && bytes
            .as_chunks::<2>()
            .0
            .iter()
            .any(|&[lo, hi]| lo != 0 && hi == 0);
    match detected {
        TextEncoding::Utf8 | TextEncoding::Windows1251 if bomless_wide => {
            TextEncoding::Utf16Le.decode_lossy(bytes)
        }
        TextEncoding::Windows1251 => platform::decode_oem(bytes),
        encoding => encoding.decode_lossy(bytes),
    }
}

#[cfg(test)]
mod tests;
