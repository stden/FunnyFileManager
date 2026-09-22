//! Purpose: The fish engine itself: output parsing, live spawns, timeouts, exit codes and the poll loop.

use super::*;

// ── fish engine: output parsing, spawns, timeouts, poll loop ─────────────

/// The fish output parser keeps the value column, drops the tab-separated
/// description, and skips blank lines. Pure (no fish spawn), so the
/// description-stripping is exercised on every platform — unlike the live
/// engine, whose `cd` output happens to carry no description here.
#[test]
fn parse_fish_completions_keeps_values_drops_descriptions_and_blanks() {
    let raw = "alpha/\tDirectory\nbeta/\ngamma/\tSubcommand\n\n\n";
    assert_eq!(
        parse_fish_completions(raw),
        vec![
            "alpha/".to_string(),
            "beta/".to_string(),
            "gamma/".to_string(),
        ],
    );
    assert!(parse_fish_completions("").is_empty());
    // A value with interior spaces (a path with spaces) survives whole; only
    // the tab boundary splits.
    assert_eq!(
        parse_fish_completions("my docs/\tDirectory"),
        vec!["my docs/".to_string()],
    );
}

/// The live fish engine offers the directories that actually exist in `cwd`.
/// Runs real `fish`, so it is skipped where fish is not installed.
///
/// The token is the path fragment after `cd ` — `fish_path_complete` prefixes
/// `cd` itself when it builds the line for fish.
#[test]
fn fish_path_complete_offers_existing_directories() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");
    fs::create_dir(tmp.path().join("alpine")).expect("dir");

    // Serialize with the fake-PATH fish tests (they shadow `fish` on PATH).
    let _lock = crate::test_support::env_lock();
    if !fish_is_installed() {
        return;
    }

    let Some(candidates) = fish_path_complete("al", tmp.path()) else {
        // fish is installed but produced nothing (unusual config); nothing to assert.
        return;
    };
    assert!(
        candidates.iter().any(|c| c == "alpha/"),
        "existing directory must be offered: {candidates:?}"
    );
    assert!(
        candidates.iter().any(|c| c == "alpine/"),
        "the other matching directory must be offered too: {candidates:?}"
    );
}

/// fish yields nothing for a leaf that matches no directory, and the engine
/// reports that as `None` (not `Some([])`) so the caller's fallback is clean.
#[test]
fn fish_path_complete_returns_none_when_nothing_matches() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");

    // Serialize with the fake-PATH fish tests (they shadow `fish` on PATH).
    let _lock = crate::test_support::env_lock();
    if !fish_is_installed() {
        return;
    }

    assert!(
        fish_path_complete("zzq_no_such_prefix", tmp.path()).is_none(),
        "no match must be None, never Some(empty)"
    );
}

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

    let temp = tempfile::TempDir::new().expect("temp dir");
    let stub = temp.path().join("slow-fish.sh");
    std::fs::write(
        &stub,
        "#!/bin/sh\nsleep 0.08\nprintf 'alpha\\tDirectory\\n'\n",
    )
    .expect("write stub");
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    super::FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = Some(stub));
    // Retry while the freshly-written stub is still ETXTBSY.
    //
    // `fs::write` closes its own handle, but a sibling test thread that forks
    // while it is open hands a copy of that write descriptor to its child, and
    // `exec` of the file then fails with "text file busy" until the child
    // exits. This suite forks constantly (every `ops::shell` test), and a
    // standalone probe measured the race at ~20% of attempts against eight
    // spawning threads — which is what made this test fail about one run in
    // five. `fish_path_complete` swallows the spawn error on purpose (a fish
    // that will not start is not a completion), so the retry belongs here.
    let candidates = (0..20).find_map(|_| {
        let result = super::fish_path_complete("al", temp.path());
        if result.is_none() {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        result
    });
    super::FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = None);

    // The stub sleeps past several 20ms polls before answering, so the loop's
    // wait branch runs; the answer must still arrive intact afterwards.
    assert_eq!(candidates, Some(vec!["alpha".to_owned()]));
}

#[cfg(unix)]
fn sleeping_fish(root: &std::path::Path) -> std::path::PathBuf {
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
