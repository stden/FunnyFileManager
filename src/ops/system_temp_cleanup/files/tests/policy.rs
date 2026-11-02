//! Purpose: Retention boundaries, root identity and invalid sources keep temporary files safe.
use super::*;
use crate::models::temp_cleanup::TempCutoff;

#[test]
fn only_files_strictly_older_than_seven_days_are_eligible() {
    let temp = tempfile::TempDir::new().unwrap();
    let now = fixed_now();
    let cutoff = now - TempCutoff::RETENTION;
    for (name, modified) in [
        ("exactly-seven-days", cutoff),
        ("older", cutoff - Duration::from_secs(2)),
        ("newer", cutoff + Duration::from_secs(2)),
        ("future", now + Duration::from_secs(2)),
    ] {
        timed_file(&temp.path().join(name), modified);
    }
    let plan = scan(&source(temp.path()), now).unwrap();
    assert_eq!(plan.files.len(), 1);
    assert_eq!(
        plan.files[0].path.file_name(),
        Some(std::ffi::OsStr::new("older"))
    );
    assert_eq!(remove(&plan).removed, 1);
    for name in ["exactly-seven-days", "newer", "future"] {
        assert_eq!(fs::read(temp.path().join(name)).unwrap(), b"payload");
    }
}

#[test]
fn replacing_the_root_preserves_even_an_original_planned_file_moved_back_into_it() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("temporary");
    let parked = temp.path().join("original-root");
    let now = SystemTime::now();
    fs::create_dir(&root).unwrap();
    old_file(&root.join("eligible"), b"original", now);
    fs::write(root.join("retained"), b"fresh original-root file").unwrap();
    let plan = scan(&source(&root), now).unwrap();
    assert_eq!(plan.files.len(), 1);
    replace_root_preserving_candidate(&root, &parked);
    let outcome = remove(&plan);
    assert_eq!(outcome.removed, 0);
    assert_eq!(outcome.freed.bytes(), 0);
    assert_eq!(fs::read(root.join("eligible")).unwrap(), b"original");
    assert_eq!(fs::read(root.join("new-file")).unwrap(), b"new root file");
    assert_eq!(
        fs::read(parked.join("retained")).unwrap(),
        b"fresh original-root file"
    );
    assert!(root.is_dir() && parked.is_dir());
}

#[test]
fn a_clock_too_early_for_the_retention_period_refuses_scanning() {
    let temp = tempfile::TempDir::new().unwrap();
    fs::write(temp.path().join("keep"), b"untouched").unwrap();
    assert!(matches!(
        scan(&source(temp.path()), earliest_representable_second()),
        Err(TempError::Clock)
    ));
    assert_eq!(fs::read(temp.path().join("keep")).unwrap(), b"untouched");
}

#[test]
fn relative_and_non_directory_sources_are_refused_without_changing_the_file() {
    let temp = tempfile::TempDir::new().unwrap();
    let file = temp.path().join("not-a-directory");
    fs::write(&file, b"unchanged").unwrap();
    for root in [Path::new("relative-temp"), file.as_path()] {
        assert!(matches!(
            scan(&source(root), fixed_now()),
            Err(TempError::UnsafeRoot)
        ));
    }
    assert_eq!(fs::read(file).unwrap(), b"unchanged");
}

fn fixed_now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

fn timed_file(path: &Path, modified: SystemTime) {
    fs::write(path, b"payload").unwrap();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
}

fn replace_root_preserving_candidate(root: &Path, parked: &Path) {
    fs::rename(root, parked).unwrap();
    fs::create_dir(root).unwrap();
    fs::rename(parked.join("eligible"), root.join("eligible")).unwrap();
    fs::write(root.join("new-file"), b"new root file").unwrap();
}

fn earliest_representable_second() -> SystemTime {
    let (mut lower, mut upper) = (0, u64::MAX);
    while lower < upper {
        let middle = lower + (upper - lower).div_ceil(2);
        if SystemTime::UNIX_EPOCH
            .checked_sub(Duration::from_secs(middle))
            .is_some()
        {
            lower = middle;
        } else {
            upper = middle - 1;
        }
    }
    SystemTime::UNIX_EPOCH
        .checked_sub(Duration::from_secs(lower))
        .unwrap()
}

#[cfg(windows)]
struct VarsGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

#[cfg(windows)]
impl Drop for VarsGuard {
    fn drop(&mut self) {
        for (k, v) in self.0.drain(..) {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}

#[cfg(windows)]
#[test]
fn windows_system_temp_source_validates_appdata_local_temp_without_touching_tmp() {
    let sandbox = tempfile::TempDir::new().unwrap();
    let (home, other) = (sandbox.path().join("home"), sandbox.path().join("other"));
    let local = home.join("AppData").join("Local");
    fs::create_dir_all(&local).unwrap();
    fs::create_dir_all(&other).unwrap();
    let _lock = crate::test_support::env_lock();
    let keys = ["HOME", "USERPROFILE", "LOCALAPPDATA"];
    let _restore = VarsGuard(keys.into_iter().map(|k| (k, std::env::var_os(k))).collect());
    let sys_err = || super::super::policy::root(&TempSource::System).unwrap_err();
    drop(super::super::policy::root(&TempSource::System));
    for k in ["HOME", "USERPROFILE"] {
        std::env::set_var(k, &home);
    }
    std::env::set_var("LOCALAPPDATA", &other);
    assert_eq!(sys_err(), TempError::UnsafeRoot);
    std::env::set_var("LOCALAPPDATA", &local);
    assert_eq!(sys_err(), TempError::UnsafeRoot);
    fs::create_dir(local.join("Temp")).unwrap();
    assert_eq!(sys_err(), TempError::UnsafeRoot);
    for k in ["HOME", "USERPROFILE"] {
        std::env::remove_var(k);
    }
    assert_eq!(sys_err(), TempError::UnsafeRoot);
}
