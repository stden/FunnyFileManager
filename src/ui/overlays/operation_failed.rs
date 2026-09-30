//! Purpose: Far's "Cannot copy" warning: the copy, move or delete stopped,
//! at which entry, why, and how far it had got.

use ratatui::Frame;

use super::super::{dialog, printable_name, tail_cells, Cells};
use crate::models::OperationFailed;
use crate::progress::Operation;

/// The widest a path is shown before its start gives way to `…`: the file
/// name at its end is the part worth reading.
const PATH_CELLS: u16 = 64;

pub(super) fn render_operation_failed(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    failure: &OperationFailed,
) {
    let width = f.area().width.saturating_sub(12).min(PATH_CELLS);
    let path = printable_name(&crate::path_display::label(failure.failed().path())).into_owned();
    let mut message = vec![
        heading(failure.operation()).to_owned(),
        tail_cells(&path, Cells::columns(width)),
        failure.failed().error().to_owned(),
    ];
    if let Some(denial) = failure.denial() {
        message.push(denial.explanation());
    }
    if failure.done() > 0 || failure.left() > 0 {
        message.push(format!(
            "Stopped: {} done, {} not started",
            failure.done(),
            failure.left()
        ));
    }
    let retry = failure.retry().is_some();
    let focused = failure.retry_focused();
    let mut buttons = Vec::new();
    if retry {
        buttons.push(
            dialog::Button::new("Retry as &administrator")
                .default_button()
                .focused(focused),
        );
    }
    let ok = dialog::Button::new("OK").focused(!focused);
    buttons.push(if retry { ok } else { ok.default_button() });
    dialog::render_warning(f, tracker, "Error", &message, &buttons);
}

const fn heading(operation: Operation) -> &'static str {
    match operation {
        Operation::Copy => "Cannot copy",
        Operation::Move => "Cannot move",
        Operation::Delete => "Cannot delete",
    }
}
