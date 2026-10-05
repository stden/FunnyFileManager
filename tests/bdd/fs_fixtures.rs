//! Purpose: Given steps that lay out files and directories on disk, with their ages.

use super::*;

fn set_file_age(path: &Path, age: SystemTime) -> StepResult {
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|error| format!("open {} to set its timestamp: {error}", path.display()))?
        .set_modified(age)
        .map_err(|error| format!("set timestamp on {}: {error}", path.display()))
}

fn arrange_source_age(world: &FfmWorld, name: &str, source_is_newer: bool) -> StepResult {
    let source = world.src_path()?.join(name);
    let destination = world.dst_path()?.join(name);
    assert!(
        source.is_file(),
        "missing source fixture: {}",
        source.display()
    );
    assert!(
        destination.is_file(),
        "missing destination fixture: {}",
        destination.display()
    );

    let earlier = SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    let later = earlier + Duration::from_hours(24);
    let (source_age, destination_age) = if source_is_newer {
        (later, earlier)
    } else {
        (earlier, later)
    };
    set_file_age(&source, source_age)?;
    set_file_age(&destination, destination_age)?;

    assert_timestamp_order(&source, &destination, source_is_newer)
}

// ═══════════════════════════════════════════════════════════════════
// GIVEN steps
// ═══════════════════════════════════════════════════════════════════

#[given("a temporary directory")]
fn given_temp_dir(world: &mut FfmWorld) -> StepResult {
    world.temp_dir = Some(TempDir::new().context("create temp dir")?);
    Ok(())
}

#[given("a source directory")]
fn given_src_dir(world: &mut FfmWorld) -> StepResult {
    world.src_dir = Some(TempDir::new().context("create src dir")?);
    Ok(())
}

#[given("an empty destination directory")]
fn given_empty_dst_dir(world: &mut FfmWorld) -> StepResult {
    world.dst_dir = Some(TempDir::new().context("create dst dir")?);
    Ok(())
}

#[given(expr = "a file {string} with content {string}")]
fn given_file(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    let base = world.temp_path()?;
    let path = base.join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create parent dir")?;
    }
    fs::write(path, &content).context("write file")?;
    Ok(())
}

#[given(expr = "a file {string} with content {string} in the source")]
fn given_file_in_src(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    fs::write(world.src_path()?.join(&name), &content).context("write file")?;
    Ok(())
}

#[given(expr = "a file {string} with content {string} in the destination")]
fn given_file_in_dst(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    fs::write(world.dst_path()?.join(&name), &content).context("write dst file")?;
    Ok(())
}

#[given(expr = "a file {string} with content {string} in the source dir")]
fn given_file_in_src_subpath(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    let path = world.src_path()?.join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create parent dir")?;
    }
    fs::write(path, &content).context("write file")?;
    Ok(())
}

#[given(expr = "a subdirectory {string} with file {string}")]
fn given_subdir_with_file(world: &mut FfmWorld, dir: String, file: String) -> StepResult {
    let sub = world.temp_path()?.join(&dir);
    fs::create_dir_all(&sub).context("create subdir")?;
    fs::write(sub.join(&file), "content").context("write file")?;
    Ok(())
}

#[given(expr = "a subdirectory {string} in the source")]
fn given_subdir_in_src(world: &mut FfmWorld, dir: String) -> StepResult {
    fs::create_dir_all(world.src_path()?.join(&dir)).context("create subdir")?;
    Ok(())
}

#[given(expr = "a subdirectory {string} in the source with file {string}")]
fn given_subdir_in_src_with_file(world: &mut FfmWorld, dir: String, file: String) -> StepResult {
    let sub = world.src_path()?.join(&dir);
    fs::create_dir_all(&sub).context("create subdir")?;
    fs::write(sub.join(&file), "content").context("write file")?;
    Ok(())
}

#[given(expr = "the source copy of {string} is older than the destination copy")]
fn given_source_copy_is_older(world: &mut FfmWorld, name: String) -> StepResult {
    arrange_source_age(world, &name, false)
}

#[given(expr = "the source copy of {string} is newer than the destination copy")]
fn given_source_copy_is_newer(world: &mut FfmWorld, name: String) -> StepResult {
    arrange_source_age(world, &name, true)
}

#[given(expr = "{int} numbered files")]
fn given_numbered_files(world: &mut FfmWorld, count: usize) -> StepResult {
    let base = world.temp_path()?;
    for i in 0..count {
        fs::write(base.join(format!("file_{i:02}.txt")), "x").context("write")?;
    }
    Ok(())
}

fn assert_timestamp_order(source: &Path, destination: &Path, source_is_newer: bool) -> StepResult {
    let actual_source = fs::metadata(source)
        .and_then(|metadata| metadata.modified())
        .context("read source timestamp")?;
    let actual_destination = fs::metadata(destination)
        .and_then(|metadata| metadata.modified())
        .context("read destination timestamp")?;
    assert_eq!(
        actual_source > actual_destination,
        source_is_newer,
        "the filesystem did not retain the requested fixture timestamps"
    );
    Ok(())
}
