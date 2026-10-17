//! Purpose: The WSL dialog's geometry, shared by the renderer and clicks:
//! the centred window, its status and list rows, and the buttons wrapped to
//! fit — full labels first, then compact ones, then only Esc.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;

use crate::models::{WslActivity, WslDialog};

use super::super::{far_block, Cells};

#[derive(Clone, Copy)]
pub(super) struct Control {
    pub(super) label: &'static str,
    pub(super) key: KeyCode,
}

impl Control {
    pub(super) fn enabled(self, dialog: &WslDialog) -> bool {
        self.key == KeyCode::Esc || !matches!(dialog.activity, WslActivity::Busy { .. })
    }
}

const CONTROLS: [Control; 5] = [
    Control {
        label: "[Enter Open root]",
        key: KeyCode::Enter,
    },
    Control {
        label: "[F2 Refresh list]",
        key: KeyCode::F(2),
    },
    Control {
        label: "[F5 Update WSL]",
        key: KeyCode::F(5),
    },
    Control {
        label: "[F6 Update packages]",
        key: KeyCode::F(6),
    },
    Control {
        label: "[Esc Close]",
        key: KeyCode::Esc,
    },
];

const COMPACT: [Control; 5] = [
    Control {
        label: "Open",
        key: KeyCode::Enter,
    },
    Control {
        label: "F2",
        key: KeyCode::F(2),
    },
    Control {
        label: "F5",
        key: KeyCode::F(5),
    },
    Control {
        label: "F6",
        key: KeyCode::F(6),
    },
    Control {
        label: "Esc",
        key: KeyCode::Esc,
    },
];

pub(super) struct Button {
    pub(super) control: Control,
    pub(super) area: Rect,
}

pub(super) struct Layout {
    pub(super) area: Rect,
    pub(super) inner: Rect,
    pub(super) status_rows: u16,
    pub(super) list_height: u16,
    pub(super) buttons: Vec<Button>,
}

pub(super) fn layout(screen: Rect, dialog: &WslDialog) -> Layout {
    let width = screen.width.min(74);
    let full_rows = control_rows(&CONTROLS, width.saturating_sub(2));
    let wanted = dialog.distributions().len().clamp(1, 10) + full_rows.len() + 5;
    let height = screen.height.min(u16::try_from(wanted).unwrap_or(u16::MAX));
    let area = Rect::new(
        screen.x + screen.width.saturating_sub(width) / 2,
        screen.y + screen.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let inner = far_block().inner(area);
    let status_rows = inner.height.min(if inner.height >= 7 { 2 } else { 1 });
    let reserve_list = u16::from(inner.height >= 3);
    let control_budget = inner.height.saturating_sub(status_rows + reserve_list);
    let rows = responsive_controls(inner, control_budget, full_rows);
    let control_height = u16::try_from(rows.len()).unwrap_or(control_budget);
    let list_height = inner.height.saturating_sub(status_rows + control_height);
    let buttons = place_buttons(inner, control_height, rows);
    Layout {
        area,
        inner,
        status_rows,
        list_height,
        buttons,
    }
}

fn place_buttons(inner: Rect, control_height: u16, rows: Vec<Vec<Control>>) -> Vec<Button> {
    let mut buttons = Vec::new();
    for (row, controls) in rows.into_iter().enumerate() {
        let mut x = inner.x;
        let y = inner.bottom() - control_height + u16::try_from(row).unwrap_or(0);
        for control in controls {
            let text_width = u16::try_from(Cells::of(control.label).get()).unwrap_or(u16::MAX);
            let width = text_width.min(inner.right().saturating_sub(x));
            buttons.push(Button {
                control,
                area: Rect::new(x, y, width, 1),
            });
            x = x.saturating_add(width).saturating_add(1);
        }
    }
    buttons
}

fn responsive_controls(
    inner: Rect,
    control_budget: u16,
    full_rows: Vec<Vec<Control>>,
) -> Vec<Vec<Control>> {
    if inner.width == 0 || control_budget == 0 {
        Vec::new()
    } else if full_rows.len() <= usize::from(control_budget) {
        full_rows
    } else {
        let compact_rows = control_rows(&COMPACT, inner.width);
        if compact_rows.len() <= usize::from(control_budget) {
            compact_rows
        } else {
            vec![vec![COMPACT[4]]]
        }
    }
}

fn control_rows(controls: &[Control], width: u16) -> Vec<Vec<Control>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut used = 0;
    for &control in controls {
        let length = Cells::of(control.label).get();
        if !row.is_empty() && used + length + 1 > usize::from(width) {
            rows.push(std::mem::take(&mut row));
            used = 0;
        }
        used += length + usize::from(!row.is_empty());
        row.push(control);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}
