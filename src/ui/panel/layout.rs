//! Purpose: How a panel's interior width is shared between stripes, names and
//! columns — the layout the renderer draws and the mouse reads, so a click
//! lands on what was drawn.

use super::super::Cells;
use super::columns::{table_columns, PanelColumn, COLUMN_SEPARATOR_WIDTH};
use crate::models::PanelView;

/// Narrowest name column the panel will render.
///
/// Without a floor, the fixed-width size column has absolute priority and a
/// narrow terminal (30 columns → 15 per panel) leaves the name exactly zero
/// cells, so `fit_name` returns "" for every entry and the panel becomes a
/// column of icons and sizes with no filenames at all. A file manager that
/// cannot show file names is useless, so the size column yields instead.
pub(super) const MIN_NAME_WIDTH: Cells = Cells(8);

/// How one panel's interior width is shared between names and columns.
///
/// The panel is `stripes` tables side by side — Brief's three, Medium's two,
/// one for Full and Wide — each a name followed by `columns`. The entries run
/// down the first stripe and on into the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui) struct PanelLayout {
    pub(in crate::ui) stripes: usize,
    /// The name width in every stripe but the last.
    pub(in crate::ui) name: Cells,
    /// The last stripe's name width. It also takes the cells an uneven split
    /// leaves over, so every row reaches the frame.
    pub(in crate::ui) last_name: Cells,
    pub(in crate::ui) columns: &'static [PanelColumn],
}

impl PanelLayout {
    pub(super) const fn name_width(self, stripe: usize) -> Cells {
        if stripe + 1 == self.stripes {
            self.last_name
        } else {
            self.name
        }
    }

    /// The cells `stripe` spans: its name and every column behind its rule.
    fn stripe_width(self, stripe: usize) -> Cells {
        self.name_width(stripe) + columns_width(self.columns)
    }
}

/// The cells `columns` take behind a name, each with its rule.
fn columns_width(columns: &[PanelColumn]) -> Cells {
    columns
        .iter()
        .map(|column| column.width() + COLUMN_SEPARATOR_WIDTH)
        .sum()
}

/// The layout of a `view` panel `interior` cells wide.
///
/// The stripes share what the rules between them leave. Within a stripe, the
/// widest column set whose leftovers still clear [`MIN_NAME_WIDTH`] is kept:
/// Far shows Name | Size | Date | Time, a 40-column panel cannot, and the name
/// is the column a file manager exists to show, so the trailing columns are
/// dropped from the right — Time first, then Date, then Size — until the name
/// fits. Dropping from the left instead would leave a Time column beside
/// nameless rows.
pub(in crate::ui) fn panel_layout(view: PanelView, interior: Cells) -> PanelLayout {
    let stripes = view.columns();
    let rules = Cells::new(COLUMN_SEPARATOR_WIDTH.get() * (stripes - 1));
    let names = interior.saturating_sub(rules).get();
    let name = Cells::new(names / stripes);
    let last_name = Cells::new(names - name.get() * (stripes - 1));

    let declared = table_columns(view).unwrap_or_default();
    let columns = (0..=declared.len())
        .rev()
        .filter_map(|taken| declared.get(..taken))
        .find(|columns| name.saturating_sub(columns_width(columns)) >= MIN_NAME_WIDTH)
        .unwrap_or_default();
    let spent = columns_width(columns);
    PanelLayout {
        stripes,
        name: name.saturating_sub(spent),
        last_name: last_name.saturating_sub(spent),
        columns,
    }
}

/// Which stripe of a `view` panel `interior` cells wide covers interior cell
/// `x` — the mouse's reading of [`panel_layout`], so a click lands on what was
/// drawn. The rule between two stripes belongs to the one on its left.
pub fn stripe_at(view: PanelView, interior: Cells, x: usize) -> usize {
    let layout = panel_layout(view, interior);
    let mut end = 0;
    for stripe in 0..layout.stripes {
        end += layout.stripe_width(stripe).get() + COLUMN_SEPARATOR_WIDTH.get();
        if x < end {
            return stripe;
        }
    }
    layout.stripes - 1
}
