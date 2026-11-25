//! Purpose: SQLite and databases.yaml fixtures for database explorer acceptance tests.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::application::database_jobs::DatabaseJobs;
use crate::config::databases::FILE_NAME;
use crate::test_support::{EnvVarGuard, CONFIG_HOME};

pub struct Fixture {
    pub cfg_home: PathBuf,
    pub db_path: PathBuf,
    pub jobs: DatabaseJobs,
}

impl Fixture {
    pub(super) fn new(root: &Path) -> Self {
        let cfg_home = root.join("db_cfg");
        let db_path = root.join("shop.db");
        Self {
            cfg_home,
            db_path,
            jobs: DatabaseJobs::default(),
        }
    }

    pub(super) fn with_config_env<T>(&self, run: impl FnOnce() -> T) -> T {
        let _guard = EnvVarGuard::set(CONFIG_HOME, &self.cfg_home);
        run()
    }

    pub(super) fn write_yaml(&self, contents: &str) {
        let dir = self.with_config_env(|| crate::config::dir().expect("config dir"));
        std::fs::create_dir_all(&dir).expect("create config dir");
        std::fs::write(dir.join(FILE_NAME), contents).expect("write databases.yaml");
    }

    pub(super) fn yaml_db_path(&self) -> String {
        self.db_path.to_string_lossy().replace('\\', "/")
    }

    pub(super) fn write_sqlite_yaml(&self, name: &str) {
        let yaml = format!(
            "- name: {name}\n  kind: sqlite\n  path: {}\n",
            self.yaml_db_path()
        );
        self.write_yaml(&yaml);
    }

    pub(super) fn seed_sample_db(&self) {
        let conn = Connection::open(&self.db_path).expect("open sqlite db");
        conn.execute_batch(
            "CREATE TABLE items (id INTEGER PRIMARY KEY NOT NULL, title TEXT NOT NULL);
             INSERT INTO items (id, title) VALUES (1, 'widget'), (2, 'sprocket');
             CREATE VIEW active_items AS SELECT id, title FROM items;",
        )
        .expect("seed sqlite db");
    }

    pub(super) fn seed_paged_db(&self, count: usize) {
        let mut conn = Connection::open(&self.db_path).expect("open paged sqlite db");
        conn.execute_batch(
            "CREATE TABLE items (id INTEGER PRIMARY KEY NOT NULL, title TEXT NOT NULL);",
        )
        .expect("create table");
        let tx = conn.transaction().expect("begin tx");
        {
            let mut stmt = tx
                .prepare("INSERT INTO items (id, title) VALUES (?1, ?2)")
                .expect("prepare");
            for idx in 1..=count {
                let label = format!("row-{idx:03}");
                let id = i64::try_from(idx).expect("row id fits i64");
                stmt.execute(rusqlite::params![id, label]).expect("insert");
            }
        }
        tx.commit().expect("commit");
    }

    pub(super) fn add_orders_table(&self) {
        let conn = Connection::open(&self.db_path).expect("reopen sqlite db");
        conn.execute_batch("CREATE TABLE orders (id INTEGER PRIMARY KEY NOT NULL);")
            .expect("create orders");
    }
}
