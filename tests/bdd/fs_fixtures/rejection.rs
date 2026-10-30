//! Purpose: Steps that reject a write at the filesystem root and name a missing age fixture.

use std::panic::{catch_unwind, AssertUnwindSafe};

use super::*;

#[cfg(windows)]
fn filesystem_root() -> &'static str {
    "C:\\"
}

#[cfg(not(windows))]
fn filesystem_root() -> &'static str {
    "/"
}

/// `parent()` is `None` only for a filesystem root. Writing there fails, and
/// the step reports that failure instead of leaving a broken scenario.
#[given("a write at the filesystem root is rejected")]
fn given_root_write_rejected(world: &mut FfmWorld) {
    let root = filesystem_root().to_owned();
    refuse(
        given_file(world, root.clone(), "x".to_owned()),
        "write file",
    );
    refuse(
        given_file_in_src_subpath(world, root, "x".to_owned()),
        "write file",
    );
    let failed_age = set_file_age(
        Path::new(filesystem_root()),
        SystemTime::UNIX_EPOCH + Duration::from_secs(1),
    );
    bdd_runner::expect_error(failed_age, "open ");
}

#[then("a missing age fixture names the file")]
fn then_missing_age_names_file(world: &mut FfmWorld) -> StepResult {
    let missing_source = catch_unwind(AssertUnwindSafe(|| {
        drop(arrange_source_age(world, "missing-age", true));
    }));
    assert!(
        missing_source.is_err(),
        "a missing source fixture should fail"
    );
    fs::write(world.src_path()?.join("missing-age"), "x").context("write source")?;
    let missing_destination = catch_unwind(AssertUnwindSafe(|| {
        drop(arrange_source_age(world, "missing-age", false));
    }));
    assert!(
        missing_destination.is_err(),
        "a missing destination fixture should fail"
    );
    Ok(())
}

fn refuse(result: StepResult, needle: &str) {
    bdd_runner::expect_error(result, needle);
}
