//! Purpose: Then steps about files on disk after a copy, move or delete.

use super::*;

#[then(expr = "{string} should exist in the destination")]
fn then_dst_exists(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.dst_path()?.join(&n).exists(),
        "{n} should exist in dst"
    );
    Ok(())
}

#[then(expr = "{string} should exist in the source")]
fn then_src_has(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.src_path()?.join(&n).exists(),
        "{n} should exist in src"
    );
    Ok(())
}

#[then(expr = "{string} should still exist in the source")]
fn then_src_exists(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.src_path()?.join(&n).exists(),
        "{n} should exist in src"
    );
    Ok(())
}

#[then(expr = "{string} should not exist in the source")]
fn then_src_not(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(!world.src_path()?.join(&n).exists(), "{n} should not exist");
    Ok(())
}

#[then(expr = "{string} should not exist in the destination")]
fn then_dst_not(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        !world.dst_path()?.join(&n).exists(),
        "{n} should not exist in dst"
    );
    Ok(())
}

#[then(expr = "{string} in the source should have content {string}")]
fn then_source_file_has_content(
    world: &mut FfmWorld,
    name: String,
    expected: String,
) -> StepResult {
    let path = world.src_path()?.join(&name);
    let actual =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    assert_eq!(actual, expected, "unexpected content in {}", path.display());
    Ok(())
}

#[then(expr = "{string} in the destination should have content {string}")]
fn then_destination_file_has_content(
    world: &mut FfmWorld,
    name: String,
    expected: String,
) -> StepResult {
    let path = world.dst_path()?.join(&name);
    let actual =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    assert_eq!(actual, expected, "unexpected content in {}", path.display());
    Ok(())
}

#[then(expr = "{string} should be a directory in the source")]
fn then_dir_src(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.src_path()?.join(&n).is_dir(),
        "{n} should be a directory in src"
    );
    Ok(())
}

#[then(expr = "{string} should be a directory in the destination")]
fn then_dir_dst(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.dst_path()?.join(&n).is_dir(),
        "{n} should be a directory in dst"
    );
    Ok(())
}

#[then("a delete confirmation should be shown")]
fn then_del_confirm(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_delete_confirm().is_some());
    Ok(())
}

#[then("a delete confirmation should not be shown")]
fn then_no_del_confirm(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_delete_confirm().is_none());
    Ok(())
}

// Overlays
#[then(expr = "{string} should exist in the temporary directory")]
fn then_temp_exists(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(world.temp_path()?.join(&n).exists(), "{n} should exist");
    Ok(())
}
