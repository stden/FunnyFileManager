//! Purpose: Export tree and session rows from their visible models, never hidden local files.

use super::rows::field;
use crate::models::{Expansion, FolderTree, SessionRow, SessionsPanel};
use serde_json::{json, Value};

pub(super) fn tree(tree: &FolderTree) -> Value {
    let mut value = super::rows::listing(tree.rows(), Some(tree.cursor), |row| {
        json!({
            "name":row.name(),"path":row.path.to_string_lossy(),"is_dir":true,
            "is_symlink":false,"size":0,"marked":false,"depth":row.depth(),
            "expansion":match row.expansion {
                Expansion::Collapsed=>"collapsed",Expansion::Expanded=>"expanded",Expansion::Leaf=>"leaf"
            }
        })
    });
    field(&mut value, "kind", json!("tree"));
    field(&mut value, "path", json!(tree.root.to_string_lossy()));
    value
}

pub(super) fn sessions(panel: &SessionsPanel) -> Value {
    let mut value = super::rows::listing(panel.rows(), Some(panel.cursor()), session_row);
    field(&mut value, "kind", json!("sessions"));
    field(&mut value, "path", json!("/"));
    field(
        &mut value,
        "title",
        json!(crate::models::sessions_panel::TITLE),
    );
    value
}

fn session_row(row: &SessionRow) -> Value {
    match row {
        SessionRow::Parent => json!({"name":"..","entry_kind":"parent","is_dir":true,
            "is_symlink":false,"size":0,"marked":false}),
        SessionRow::Session { name, login } => json!({"name":name.as_str(),"login":login,
            "entry_kind":"session","is_dir":false,"is_symlink":false,"size":0,"marked":false}),
    }
}
