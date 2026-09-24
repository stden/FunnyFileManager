//! Unit tests: the settings round trip through `settings.yaml`, the read-only
//! migration from `settings.json`, and the saves that must not happen.

use std::{collections::BTreeMap, fs};

use tempfile::TempDir;

use crate::config::*;
use crate::test_support::{EnvVarGuard, CONFIG_HOME};

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

fn populate_shell(settings: &mut Settings) {
    settings.system = SystemSettings {
        auto_save_setup: true,
        wipe_on_delete: true,
        history_size: 7,
    };
    settings.interface = InterfaceSettings {
        clock: !settings.interface.clock,
        key_bar: !settings.interface.key_bar,
        menu_bar: !settings.interface.menu_bar,
        mouse: !settings.interface.mouse,
    };
    settings.languages = LanguageSettings {
        main: Lang::Ru,
        help: Lang::Ru,
    };
    settings.plugins_configuration.disabled = tricky();
    settings.plugins_manager.scan_project_mcp = !settings.plugins_manager.scan_project_mcp;
    settings.plugins_manager.show_status_on_start = !settings.plugins_manager.show_status_on_start;
    settings.dialog.remember_history = !settings.dialog.remember_history;
    settings.dialog.click_outside_closes = !settings.dialog.click_outside_closes;
    settings.menu.far_letters = !settings.menu.far_letters;
    settings.menu.wrap_cursor = !settings.menu.wrap_cursor;
    settings.command_line.prompt_format = "{cwd} ~ #> ".to_owned();
    settings.auto_complete.fish = !settings.auto_complete.fish;
    settings.auto_complete.ghost = !settings.auto_complete.ghost;
    settings.auto_complete.menu = !settings.auto_complete.menu;
    settings.confirmations.delete_folders = !settings.confirmations.delete_folders;
    settings.confirmations.clear_history = !settings.confirmations.clear_history;
}

fn populate_panels(settings: &mut Settings) {
    settings.panel = PanelSettings {
        show_hidden: false,
        dirs_first: false,
        case_sensitive_sort: true,
        show_column_titles: false,
        show_status_line: false,
        show_free_size: false,
        highlight_files: false,
    };
    settings.tree = TreeSettings {
        auto_change_folder: false,
        min_width: 12,
    };
    settings.info_panel = InfoPanelSettings {
        show_kinds: false,
        show_largest: false,
        largest_count: 3,
    };
    settings.mask_groups.groups = pairs();
    settings.panel_modes.modes[0].columns = "N,S".to_owned();
    settings.panel_modes.modes[9].fullscreen = !settings.panel_modes.modes[9].fullscreen;
    settings.descriptions.names = tricky();
    settings.descriptions.update_on_move = false;
    settings.folder_descriptions.names = tricky();
    settings.colors.theme = Theme::Grey;
    settings.highlighting.rules = pairs();
}

fn populate_viewers(settings: &mut Settings) {
    settings.viewer = ViewerSettings {
        tab_size: 16,
        wrap: true,
        default_encoding: "Windows-1251".to_owned(),
    };
    settings.editor = EditorSettings {
        tab_size: 2,
        expand_tabs: !settings.editor.expand_tabs,
        auto_indent: !settings.editor.auto_indent,
        save_position: !settings.editor.save_position,
    };
    settings.code_pages.favorites = tricky();
}

fn populate_lsp(settings: &mut Settings) {
    let environment: BTreeMap<String, String> = TRICKY
        .iter()
        .enumerate()
        .map(|(index, value)| (format!("VAR_{index}"), (*value).to_owned()))
        .collect();
    let install = InstallRecipe::Command {
        steps: vec![InstallCommand {
            command: "@x".to_owned(),
            args: tricky(),
            env: environment,
        }],
        executable: "{install_dir}/bin/zls".to_owned(),
    };
    let server = ServerSettings {
        enabled: Enablement::Disabled,
        command: "~".to_owned(),
        args: tricky(),
        extensions: vec!["zig".to_owned(), "on".to_owned()],
        root_markers: tricky(),
        install: Some(install),
    };
    settings
        .lsp
        .0
        .insert("zig: the language".to_owned(), server);
    if let Some(python) = settings.lsp.0.get_mut("python") {
        python.install = None;
    }
}

/// Every page holds values that differ from the defaults, and every string
/// field holds the strings YAML is most likely to misread.
fn populated() -> Settings {
    let mut settings = Settings::default();
    populate_shell(&mut settings);
    populate_panels(&mut settings);
    populate_viewers(&mut settings);
    populate_lsp(&mut settings);
    let tricky_paths = TRICKY.map(PathBuf::from);
    for (shortcut, path) in settings.shortcuts.iter_mut().zip(tricky_paths) {
        *shortcut = Some(path);
    }
    settings.shortcuts[9] = None;
    settings
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

#[test]
fn a_save_keeps_the_options_pages_and_keys_it_does_not_know() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "options:\n  Editor: '8'\nfrom_a_newer_build: 1\n")?;
    let mut settings = Settings::load_from(&path)?;
    settings.system.wipe_on_delete = true;
    settings.save_to(&path)?;
    let text = fs::read_to_string(&path)?;
    assert!(text.contains("\noptions:\n  Editor: '8'\n"), "{text}");
    assert!(text.contains("\nfrom_a_newer_build: 1\n"), "{text}");
    assert!(text.contains("\nsystem:\n"), "{text}");
    assert!(!text.contains("\nlsp:"), "only the changed page is written");
    Ok(())
}

/// One bad value makes the load fall back to the defaults; saving those
/// would wipe the user's language servers along with the typo.
#[test]
fn a_save_never_replaces_pages_that_do_not_parse() -> io::Result<()> {
    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    let text = "system:\n  history_size: many\nlsp:\n  python:\n    enabled: false\n";
    write_file(&path, text)?;
    assert!(Settings::load_from(&path).is_err());
    let error = Settings::default()
        .save_to(&path)
        .expect_err("the stored pages do not parse");
    let message = error.to_string();
    assert!(message.contains("system.history_size"), "{message}");
    assert!(
        message.starts_with("Not saved: settings.yaml does not parse: "),
        "{message}"
    );
    assert_eq!(fs::read_to_string(&path)?, text);
    Ok(())
}

/// A path that is not valid UTF-8 has no YAML spelling: the save fails before
/// anything is written.
#[cfg(unix)]
#[test]
fn a_shortcut_yaml_cannot_spell_leaves_the_file_untouched() -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let temp = TempDir::new()?;
    let path = settings_path(&temp);
    write_file(&path, "system:\n  history_size: 5\n")?;
    let mut settings = Settings::default();
    settings.shortcuts[0] = Some(std::ffi::OsStr::from_bytes(b"/tmp/\xff").into());
    assert!(settings.save_to(&path).is_err());
    assert_eq!(fs::read_to_string(&path)?, "system:\n  history_size: 5\n");
    Ok(())
}
