//! Unit tests: the settings round trip through `settings.yaml`, the read-only
//! migration from `settings.json`, and the saves that must not happen. The
//! fully populated settings are built in `persistence/populated.rs`; what a
//! save keeps and refuses is in `persistence/saves.rs`.

use std::fs;

use tempfile::TempDir;

use crate::config::*;
use crate::test_support::{EnvVarGuard, CONFIG_HOME};

mod populated;
mod saves;

use populated::populated;

/// Strings YAML would read as something else unless the writer quotes them:
/// booleans, nulls, numbers, tags, anchors, comments, flow mappings, document
/// markers, escapes, indentation and non-ASCII text.
const TRICKY: [&str; 20] = [
    "*.rs",
    "!*.tmp",
    "@scope/pkg",
    "~",
    "null",
    "on",
    "yes",
    "",
    "123",
    "0x10",
    "{install_dir}/bin",
    "a: b",
    "#c",
    "line\nbreak",
    "\ttab",
    r"D:\Data\x",
    "Кириллица",
    " padded ",
    "---",
    "&anchor",
];

pub(in crate::config) fn tricky() -> Vec<String> {
    TRICKY.map(str::to_owned).to_vec()
}

fn pairs() -> Vec<(String, String)> {
    TRICKY
        .iter()
        .zip(TRICKY.iter().rev())
        .map(|(mask, colour)| ((*mask).to_owned(), (*colour).to_owned()))
        .collect()
}

fn settings_path(temp: &TempDir) -> PathBuf {
    temp.path().join("ffm").join("settings.yaml")
}

fn write_file(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, text)
}

#[test]
fn a_fully_populated_settings_round_trips_through_yaml() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    let settings = populated();
    assert_ne!(settings, Settings::default());
    settings.save_to(&path)?;
    assert_eq!(Settings::load_from(&path)?, settings);
    let text = fs::read_to_string(&path)?;
    assert!(text.starts_with("# FunnyFileManager settings."), "{text}");
    assert!(text.contains("\ncolors:\n  theme: Grey\n"), "{text}");
    Ok(())
}

#[test]
fn load_reads_the_legacy_json_while_there_is_no_yaml() -> io::Result<()> {
    let temp = TempDir::new()?;
    let legacy = temp.path().join("ffm").join("settings.json");
    write_file(
        &legacy,
        r#"{"system":{"wipe_on_delete":true},"Editor":"8"}"#,
    )?;
    let settings = Settings::load_from(&settings_path(&temp))?;
    assert!(settings.system.wipe_on_delete);
    assert!(
        settings.panel.show_hidden,
        "missing pages keep their defaults"
    );
    Ok(())
}

#[test]
fn settings_yaml_wins_over_the_legacy_json() -> io::Result<()> {
    let temp = TempDir::new()?;
    let legacy = temp.path().join("ffm").join("settings.json");
    write_file(&legacy, r#"{"system":{"wipe_on_delete":true}}"#)?;
    write_file(&settings_path(&temp), "system:\n  history_size: 5\n")?;
    let settings = Settings::load_from(&settings_path(&temp))?;
    assert!(!settings.system.wipe_on_delete, "the JSON file is not read");
    assert_eq!(settings.system.history_size, 5);
    Ok(())
}

#[test]
fn load_survives_a_missing_and_a_corrupt_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    assert_eq!(Settings::load(), Settings::default());
    write_file(&settings_path(&temp), "system: [\n")?;
    assert_eq!(Settings::load(), Settings::default());
    write_file(&settings_path(&temp), "system:\n  wipe_on_delete: true\n")?;
    let partial = Settings::load();
    assert!(
        partial.system.wipe_on_delete,
        "a partial file loads its fields"
    );
    assert!(partial.panel.show_hidden, "and defaults the rest");
    Ok(())
}

#[test]
fn save_then_load_round_trips_through_the_config_home() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    let mut settings = Settings::default();
    settings.system.auto_save_setup = true;
    settings.colors.theme = Theme::Grey;
    settings.save()?;
    assert!(settings_path(&temp).is_file());
    assert!(!temp.path().join("ffm/settings.json").exists());
    assert_eq!(Settings::load(), settings);
    Ok(())
}

#[test]
fn load_survives_an_unreadable_settings_file() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _home = EnvVarGuard::set(CONFIG_HOME, temp.path());
    fs::create_dir_all(settings_path(&temp))?;
    assert_eq!(Settings::load(), Settings::default());
    Ok(())
}
