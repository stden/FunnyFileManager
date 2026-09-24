//! Purpose: S3 paths, size formatting and unique names.

use super::*;

// VirtualPath
#[when(expr = "я разбираю путь {string}")]
fn when_parse_path(world: &mut FfmWorld, path: String) {
    world.parsed_path = Some(VirtualPath::parse(&path));
}

// Утилиты
#[when(expr = "я форматирую размер {int}")]
fn when_format_size(world: &mut FfmWorld, size: u64) {
    world.formatted_size = Some(FileSize::from_bytes(size).to_string());
}

#[when(expr = "я запрашиваю уникальное имя для {string}")]
fn when_unique_name(world: &mut FfmWorld, name: String) -> StepResult {
    let result = unique_destination(world.temp_path()?.as_path(), OsStr::new(&name))
        .context("уникальное имя")?;
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
#[then("тип пути должен быть Local")]
fn then_local(world: &mut FfmWorld) {
    assert!(matches!(world.parsed_path, Some(VirtualPath::Local(_))));
}

#[then("тип пути должен быть S3")]
fn then_s3(world: &mut FfmWorld) {
    assert!(matches!(world.parsed_path, Some(VirtualPath::S3 { .. })));
}

#[then(expr = "профиль должен быть {string}")]
fn then_profile(world: &mut FfmWorld, p: String) -> StepResult {
    match world.parsed_path.as_ref().context("путь")? {
        VirtualPath::S3 { profile, .. } => assert_eq!(profile, &p),
        VirtualPath::Local(_) => return Err("ожидался S3 путь".to_owned()),
    }
    Ok(())
}

#[then(expr = "бакет должен быть {string}")]
fn then_bucket(world: &mut FfmWorld, b: String) -> StepResult {
    match world.parsed_path.as_ref().context("путь")? {
        VirtualPath::S3 { bucket, .. } => assert_eq!(bucket.as_deref(), Some(b.as_str())),
        VirtualPath::Local(_) => return Err("ожидался S3 путь".to_owned()),
    }
    Ok(())
}

#[then("бакет должен быть пустым")]
fn then_bucket_empty(world: &mut FfmWorld) -> StepResult {
    match world.parsed_path.as_ref().context("путь")? {
        VirtualPath::S3 { bucket, .. } => assert!(bucket.is_none()),
        VirtualPath::Local(_) => return Err("ожидался S3 путь".to_owned()),
    }
    Ok(())
}

#[then(expr = "префикс должен быть {string}")]
fn then_prefix(world: &mut FfmWorld, p: String) -> StepResult {
    match world.parsed_path.as_ref().context("путь")? {
        VirtualPath::S3 { prefix, .. } => assert_eq!(prefix, &p),
        VirtualPath::Local(_) => return Err("ожидался S3 путь".to_owned()),
    }
    Ok(())
}

// Утилиты
#[then(expr = "отформатированный размер должен быть {string}")]
fn then_formatted(world: &mut FfmWorld, s: String) {
    assert_eq!(world.formatted_size.as_deref(), Some(s.as_str()));
}

#[then(expr = "уникальное имя должно быть {string}")]
fn then_unique(world: &mut FfmWorld, s: String) {
    assert_eq!(world.unique_name.as_deref(), Some(s.as_str()));
}
