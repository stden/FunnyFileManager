//! Purpose: S3 paths, AWS profiles, size formatting and unique names.

use super::*;

// VirtualPath
#[when(expr = "I parse path {string}")]
fn when_parse_path(world: &mut FfmWorld, path: String) {
    world.parsed_path = Some(VirtualPath::parse(&path));
}

#[when("I list AWS profiles")]
fn when_list_profiles(world: &mut FfmWorld) {
    world.aws_profiles = list_aws_profiles().into_iter().map(|p| p.name).collect();
}

// Utilities
#[when(expr = "I format size {int}")]
fn when_format_size(world: &mut FfmWorld, size: u64) {
    world.formatted_size = Some(FileSize::from_bytes(size).to_string());
}

#[when(expr = "I request unique name for {string}")]
fn when_unique_name(world: &mut FfmWorld, name: String) -> StepResult {
    let result = unique_destination(world.temp_path()?.as_path(), OsStr::new(&name))
        .context("unique name")?;
    world.unique_name = Some(
        result
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    );
    Ok(())
}

// VirtualPath
#[then(expr = "the path type should be Local")]
fn then_local(world: &mut FfmWorld) {
    assert!(matches!(world.parsed_path, Some(VirtualPath::Local(_))));
}

#[then(expr = "the path type should be S3")]
fn then_s3(world: &mut FfmWorld) {
    assert!(matches!(world.parsed_path, Some(VirtualPath::S3 { .. })));
}

#[then(expr = "the display should be {string}")]
fn then_display(world: &mut FfmWorld, s: String) -> StepResult {
    assert_eq!(world.parsed_path.as_ref().context("path")?.display(), s);
    Ok(())
}

#[then(expr = "the profile should be {string}")]
fn then_profile(world: &mut FfmWorld, p: String) -> StepResult {
    match world.parsed_path.as_ref().context("path")? {
        VirtualPath::S3 { profile, .. } => assert_eq!(profile, &p),
        VirtualPath::Local(_) => return Err("expected S3 path".to_owned()),
    }
    Ok(())
}

#[then(expr = "the bucket should be {string}")]
fn then_bucket(world: &mut FfmWorld, b: String) -> StepResult {
    match world.parsed_path.as_ref().context("path")? {
        VirtualPath::S3 { bucket, .. } => assert_eq!(bucket.as_deref(), Some(b.as_str())),
        VirtualPath::Local(_) => return Err("expected S3 path".to_owned()),
    }
    Ok(())
}

#[then("the bucket should be empty")]
fn then_bucket_empty(world: &mut FfmWorld) -> StepResult {
    match world.parsed_path.as_ref().context("path")? {
        VirtualPath::S3 { bucket, .. } => assert!(bucket.is_none()),
        VirtualPath::Local(_) => return Err("expected S3 path".to_owned()),
    }
    Ok(())
}

#[then(expr = "the prefix should be {string}")]
fn then_prefix(world: &mut FfmWorld, p: String) -> StepResult {
    match world.parsed_path.as_ref().context("path")? {
        VirtualPath::S3 { prefix, .. } => assert_eq!(prefix, &p),
        VirtualPath::Local(_) => return Err("expected S3 path".to_owned()),
    }
    Ok(())
}

#[then(expr = "the profiles should include {string}")]
fn then_profiles_include(world: &mut FfmWorld, name: String) {
    assert!(world.aws_profiles.iter().any(|p| p == &name));
}

// Utilities
#[then(expr = "the formatted size should be {string}")]
fn then_formatted(world: &mut FfmWorld, s: String) {
    assert_eq!(world.formatted_size.as_deref(), Some(s.as_str()));
}

#[then(expr = "the unique name should be {string}")]
fn then_unique(world: &mut FfmWorld, s: String) {
    assert_eq!(world.unique_name.as_deref(), Some(s.as_str()));
}
