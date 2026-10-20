//! Purpose: F9 Commands > Compare folders: mark the files that are missing
//! from the other panel, or have another size there, in both panels.

use crate::models::App;

pub(super) fn compare(app: &mut App) {
    let (left, right) = (&mut app.left_panel, &mut app.right_panel);
    left.marked.clear();
    right.marked.clear();
    mark_different(left, right);
    mark_different(right, left);
    app.set_status(crate::status::Status::CommandOutput(
        "Marked missing or different-sized files in both panels".to_owned(),
    ));
}

fn mark_different(panel: &mut crate::models::Panel, other: &crate::models::Panel) {
    let other_entries: std::collections::HashSet<(&str, u64)> = other
        .entries
        .iter()
        .filter(|candidate| !candidate.kind.is_dir())
        .map(|candidate| (candidate.name.as_str(), candidate.size.bytes()))
        .collect();
    for entry in &panel.entries {
        if !entry.kind.is_dir()
            && !other_entries.contains(&(entry.name.as_str(), entry.size.bytes()))
        {
            panel
                .marked
                .insert(crate::models::FileName::from(entry.name.as_str()));
        }
    }
}
