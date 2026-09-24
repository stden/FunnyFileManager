//! Unit-test fixture: settings whose every page differs from the defaults and
//! whose every string field holds the strings YAML is most likely to misread.

use std::collections::BTreeMap;

use super::*;

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
pub(super) fn populated() -> Settings {
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
