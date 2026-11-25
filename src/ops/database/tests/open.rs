//! Purpose: Pin how the dialog opens: the connections `databases.yaml` lists, or
//! a notice (also shown as an error in the status bar) saying why there are none.

use std::path::{Path, PathBuf};

use super::super::open;
use crate::config::databases::FILE_NAME;
use crate::models::database::{DatabaseExplorer, Row};
use crate::models::{App, Overlay, Panel};
use crate::status::Status;
use crate::test_support::{EnvVarGuard, EnvVarsGuard, CONFIG_HOME};

const ENTRIES: &str = "\
- name: shop
  kind: sqlite
  path: /data/shop.db
- name: warehouse
  kind: postgres
  database: analytics
  user: reader
";

fn app_in(dir: &Path) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    App::from_panels(panel.clone(), panel)
}

/// Open the dialog with a config directory that holds `text` as `databases.yaml`
/// (no such file when `None`); also the file's path.
fn opened(text: Option<&str>) -> (App, PathBuf) {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let _config = EnvVarGuard::set(CONFIG_HOME, temp.path());
    let file = crate::config::dir()
        .expect("a config directory")
        .join(FILE_NAME);
    if let Some(text) = text {
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("config dir");
        std::fs::write(&file, text).expect("databases.yaml");
    }
    let mut app = app_in(temp.path());
    open(&mut app);
    (app, file)
}

fn explorer(app: &App) -> &DatabaseExplorer {
    let Overlay::Database(explorer) = &app.overlay else {
        panic!("the explorer is open")
    };
    explorer
}

/// The dialog opened without connections, its notice, and the same text in the status bar.
fn assert_notice(app: &App, expected: &str) {
    assert!(explorer(app).connections().is_empty());
    assert_eq!(explorer(app).notice(), Some(expected));
    assert_eq!(app.status, Status::Error(expected.to_owned()));
}

#[test]
fn the_dialog_lists_the_connections_of_the_config_file() {
    let (app, _) = opened(Some(ENTRIES));
    let explorer = explorer(&app);
    assert_eq!(explorer.connections().len(), 2);
    assert_eq!(explorer.name(Row::Connection(0)), Some("shop"));
    assert_eq!(explorer.name(Row::Connection(1)), Some("warehouse"));
    assert_eq!(explorer.notice(), None);
    assert_eq!(app.status, Status::KeyHints);
}

#[test]
fn without_a_config_file_the_notice_says_where_to_create_it() {
    let (app, file) = opened(None);
    assert_notice(
        &app,
        &format!(
            "No connections: create {} with one entry per database",
            file.display()
        ),
    );
}

#[test]
fn a_file_without_entries_says_it_lists_no_connections() {
    let (app, file) = opened(Some("  \n"));
    assert_notice(&app, &format!("{} lists no connections", file.display()));
}

#[test]
fn a_refused_file_gives_the_rule_it_breaks_after_its_path() {
    let (app, file) = opened(Some("- name: shop\n  kind: sqlite\n"));
    assert_notice(
        &app,
        &format!(
            "{}: databases.yaml: a sqlite connection needs a path",
            file.display()
        ),
    );
}

#[test]
fn without_a_config_directory_the_notice_says_so() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let mut app = app_in(temp.path());
    let unset = Path::new("");
    let _environment = EnvVarsGuard::set(&[
        ("APPDATA", unset),
        ("XDG_CONFIG_HOME", unset),
        ("HOME", unset),
        ("USERPROFILE", unset),
    ]);
    open(&mut app);
    assert_notice(&app, "No config directory, so no databases.yaml to read");
}
