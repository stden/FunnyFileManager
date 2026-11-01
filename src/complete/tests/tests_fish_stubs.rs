//! Purpose: The fish engine against stub `fish` programs: timeouts, exit codes
//! and the poll loop.

use super::*;

// ── fish engine: timeouts, exit codes, poll loop ─────────────────────────

/// A hung fish must not freeze Tab: the engine gives up after its deadline and
/// returns `None` so the built-in scan remains the only source. Uses a fake
/// `fish` on `PATH` that sleeps, so this does not need a real fish install.
#[cfg(unix)]
#[test]
fn fish_path_complete_times_out_on_a_hung_fish() {
    use std::time::{Duration, Instant};
    let tmp = empty_cwd();
    let bin = sleeping_fish(tmp.path());

    let _lock = crate::test_support::env_lock();
    let previous_path = std::env::var_os("PATH");
    let mut new_path = bin.into_os_string();
    if let Some(rest) = &previous_path {
        new_path.push(":");
        new_path.push(rest);
    }
    std::env::set_var("PATH", &new_path);

    let started = Instant::now();
    let result = fish_path_complete("al", tmp.path());

    match previous_path {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }

    assert!(
        result.is_none(),
        "a hung fish must yield None, not hang the caller"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "timed out in {:?}, expected ~1.5s",
        started.elapsed()
    );
}

/// A fish that exits non-zero is treated like "no completions" so the built-in
/// scan remains the only source rather than a hard error.
#[cfg(unix)]
#[test]
fn fish_path_complete_returns_none_when_fish_exits_nonzero() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = empty_cwd();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).expect("bin");
    let fish = bin.join("fish");
    fs::write(&fish, "#!/bin/sh\nexit 1\n").expect("write fake fish");
    let mut perms = fs::metadata(&fish).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fish, perms).expect("chmod");

    let _lock = crate::test_support::env_lock();
    let previous_path = std::env::var_os("PATH");
    let mut new_path = bin.into_os_string();
    if let Some(rest) = &previous_path {
        new_path.push(":");
        new_path.push(rest);
    }
    std::env::set_var("PATH", &new_path);

    let result = fish_path_complete("al", tmp.path());

    match previous_path {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }

    assert!(
        result.is_none(),
        "non-zero fish exit must be None: {result:?}"
    );
}

#[cfg(unix)]
#[test]
fn fish_poll_loop_waits_for_a_slow_completer() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().expect("temp dir");
    let stub = temp.path().join("slow-fish.sh");
    fs::write(
        &stub,
        "#!/bin/sh\nsleep 0.08\nprintf 'alpha\\tDirectory\\n'\n",
    )
    .expect("write stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod");

    FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = Some(stub));
    let candidates = (0..20).find_map(|_| {
        let result = fish_path_complete("al", temp.path());
        if result.is_none() {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        result
    });
    FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = None);

    // The stub sleeps past several 20ms polls before answering, so the loop's
    // wait branch runs; the answer must still arrive intact afterwards.
    assert_eq!(candidates, Some(vec!["alpha".to_owned()]));
}

/// Large completion output (>128 KiB, exceeding OS pipe buffers) must not
/// deadlock `wait_for_fish` before `try_wait` sees the process exit.
#[cfg(unix)]
#[test]
fn fish_completer_handles_output_larger_than_pipe_buffer() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().expect("temp dir");
    let payload = temp.path().join("payload.txt");
    let line = "alpha_long_directory_entry_name/\tDirectory\n";
    fs::write(&payload, line.repeat(4000)).expect("write payload");
    let stub = temp.path().join("large-fish.sh");
    fs::write(&stub, format!("#!/bin/sh\ncat '{}'\n", payload.display())).expect("write stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod");
    wait_until_executable(&stub);

    FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = Some(stub));
    let candidates = fish_path_complete("al", temp.path());
    FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = None);

    assert_eq!(candidates.as_ref().map(Vec::len), Some(4000));
}

/// fish creates `fish/` in its configuration and data homes on every run
/// (`~/.config/fish`, `~/.local/share/fish`), so a fish a test starts is given
/// homes where nothing can be created: running the suite must not write into
/// the home of whoever runs it.
#[test]
fn a_fish_started_by_a_test_gets_homes_it_cannot_write_to() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().expect("temp dir");
    let stub = temp.path().join("env-fish.sh");
    let script = "#!/bin/sh\nprintf '%s\\n%s\\n' \"$XDG_CONFIG_HOME\" \"$XDG_DATA_HOME\"\n";
    fs::write(&stub, script).expect("write stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod");
    wait_until_executable(&stub);

    FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = Some(stub));
    let homes = fish_path_complete("al", temp.path()).unwrap_or_default();
    FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = None);

    assert_eq!(homes.len(), 2, "both homes are set: {homes:?}");
    for home in homes {
        let nearest = Path::new(&home).ancestors().find(|path| path.exists());
        assert!(
            nearest.is_some_and(|path| !path.is_dir()),
            "fish could create {home}/fish"
        );
    }
}

#[cfg(unix)]
fn sleeping_fish(root: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let bin = root.join("bin");
    fs::create_dir(&bin).expect("bin");
    let fish = bin.join("fish");
    fs::write(&fish, "#!/bin/sh\nsleep 30\n").expect("write fake fish");
    let mut perms = fs::metadata(&fish).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fish, perms).expect("chmod");

    bin
}
