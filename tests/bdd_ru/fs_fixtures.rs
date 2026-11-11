//! Purpose: Given steps that lay out files and directories on disk.

use super::*;

// ═══════════════════════════════════════════════════════════════════
// ДОПУСТИМ (Given) — шаги подготовки
// ═══════════════════════════════════════════════════════════════════

#[given("существует временный каталог")]
fn given_temp_dir(world: &mut FfmWorld) -> StepResult {
    world.temp_dir = Some(TempDir::new().context("создать временный каталог")?);
    Ok(())
}

#[given("существует исходный каталог")]
fn given_src_dir(world: &mut FfmWorld) -> StepResult {
    world.src_dir = Some(TempDir::new().context("создать исходный каталог")?);
    Ok(())
}

#[given("существует пустой каталог назначения")]
fn given_empty_dst_dir(world: &mut FfmWorld) -> StepResult {
    world.dst_dir = Some(TempDir::new().context("создать каталог назначения")?);
    Ok(())
}

#[given(expr = "файл {string} с содержимым {string}")]
fn given_file(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    let base = world.temp_path()?;
    let path = base.join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("создать родительский каталог")?;
    }
    fs::write(path, &content).context("записать файл")?;
    Ok(())
}

#[given(expr = "файл {string} с содержимым {string} в источнике")]
fn given_file_in_src(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    fs::write(world.src_path()?.join(&name), &content).context("записать файл в источник")?;
    Ok(())
}

#[given(expr = "файл {string} с содержимым {string} в назначении")]
fn given_file_in_dst(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    fs::write(world.dst_path()?.join(&name), &content).context("записать файл в назначение")?;
    Ok(())
}

#[given(expr = "файл {string} с содержимым {string} в подпути источника")]
fn given_file_in_src_subpath(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    let path = world.src_path()?.join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("создать родительский каталог")?;
    }
    fs::write(path, &content).context("записать файл")?;
    Ok(())
}

#[given(expr = "подкаталог {string} с файлом {string}")]
fn given_subdir_with_file(world: &mut FfmWorld, dir: String, file: String) -> StepResult {
    let sub = world.temp_path()?.join(&dir);
    fs::create_dir_all(&sub).context("создать подкаталог")?;
    fs::write(sub.join(&file), "content").context("записать файл")?;
    Ok(())
}

#[given(expr = "подкаталог {string} в источнике")]
fn given_subdir_in_src(world: &mut FfmWorld, dir: String) -> StepResult {
    fs::create_dir_all(world.src_path()?.join(&dir)).context("создать подкаталог в источнике")?;
    Ok(())
}

#[given(expr = "подкаталог {string} в источнике с файлом {string}")]
fn given_subdir_in_src_with_file(world: &mut FfmWorld, dir: String, file: String) -> StepResult {
    let sub = world.src_path()?.join(&dir);
    fs::create_dir_all(&sub).context("создать подкаталог")?;
    fs::write(sub.join(&file), "content").context("записать файл")?;
    Ok(())
}

#[given(expr = "{int} нумерованных файлов")]
fn given_numbered_files(world: &mut FfmWorld, count: usize) -> StepResult {
    let base = world.temp_path()?;
    for i in 0..count {
        fs::write(base.join(format!("file_{i:02}.txt")), "x").context("записать")?;
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Новые шаги для покрытия оверлеев и расширенных операций
// ═══════════════════════════════════════════════════════════════════

// --- GIVEN: новые шаги подготовки ---

#[given(expr = "файл {string} с многострочным содержимым в источнике")]
fn given_multiline_file_in_src(world: &mut FfmWorld, name: String) -> StepResult {
    let content = (0..50)
        .map(|i| format!("строка {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(world.src_path()?.join(&name), &content).context("записать файл")?;
    Ok(())
}

#[cfg(windows)]
fn filesystem_root() -> &'static str {
    "C:\\"
}

#[cfg(not(windows))]
fn filesystem_root() -> &'static str {
    "/"
}

#[given("запись в корень файловой системы отклоняется")]
fn given_root_write_rejected(world: &mut FfmWorld) {
    let root = filesystem_root().to_owned();
    refuse(
        given_file(world, root.clone(), "x".to_owned()),
        "записать файл",
    );
    refuse(
        given_file_in_src_subpath(world, root, "x".to_owned()),
        "записать файл",
    );
}

fn refuse(result: StepResult, needle: &str) {
    bdd_runner::expect_error(result, needle);
}
