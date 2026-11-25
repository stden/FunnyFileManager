//! Purpose: a terminal paste into the task list's open bar — control characters dropped, cut at a limit — and nowhere without one.

use super::{find, jump};
use crate::models::{App, Modal, SearchFrom};

/// The most characters one paste puts into a bar.
const PASTE_LIMIT: usize = 256;

/// A terminal paste into the open bar, its control characters dropped and
/// cut at [`PASTE_LIMIT`]; with no bar open it goes nowhere.
pub(in crate::popup::tasks) fn paste(app: &mut App, text: &str) {
    let viewport = app.viewport;
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let page = list.page_for(viewport);
    let clean: String = text
        .chars()
        .filter(|ch| !ch.is_control())
        .take(PASTE_LIMIT)
        .collect();
    match list.modal_mut() {
        Modal::Filter(bar) => {
            bar.line_mut().insert_str(&clean);
            let text = bar.line().text().to_owned();
            list.set_filter(&text);
        }
        Modal::Search(bar) => {
            bar.line_mut().insert_str(&clean);
            let query = bar.line().text().to_owned();
            find(list, &query, SearchFrom::Cursor, page);
        }
        Modal::PidJump(query) => {
            clean.chars().for_each(|digit| query.push(digit));
            let pid = query.pid();
            jump(list, pid, page);
        }
        _ => {}
    }
}
