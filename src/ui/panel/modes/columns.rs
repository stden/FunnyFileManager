//! Detail columns retained from menu/viewmodes, fitted to the current panel
//! renderer and its cached metadata. Narrow panels give filenames priority.

use crate::models::{DirEntry, Modified, Panel, PanelView};

use super::super::{
    abbreviate_bytes, center_cells, clip_name, group_digits, head_cells, pad_cells, right_cells,
    size_field, Cells,
};

#[derive(Clone, Copy)]
enum Column {
    Size,
    GroupedSize,
    Date,
    Allocated,
    Modified,
    Created,
    Accessed,
    Attributes,
    Description,
    Owner,
    Links,
}

impl Column {
    const fn width(self) -> Option<Cells> {
        let width = match self {
            Self::Size | Self::Allocated => 6,
            Self::Date => 8,
            Self::Modified | Self::Created | Self::Accessed => 14,
            Self::Attributes if cfg!(windows) => 6,
            Self::GroupedSize | Self::Attributes => 10,
            Self::Owner => 12,
            // A link detail may contain a symbolic target as well as a count.
            Self::Description | Self::Links => return None,
        };
        Some(Cells::new(width))
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Size | Self::GroupedSize => "Size",
            Self::Date => "Date",
            Self::Allocated => "Alloc",
            Self::Modified => "Modified",
            Self::Created => "Created",
            Self::Accessed => "Accessed",
            Self::Attributes => "Attr",
            Self::Description => "Description",
            Self::Owner => "Owner",
            Self::Links => "Links",
        }
    }

    /// The text of one detail cell.
    ///
    /// Description, Owner and Links carry text from `descript.ion` files, from
    /// the passwd database and from symlink targets, so they get the same
    /// treatment a name does: nothing the terminal refuses to draw survives
    /// into a width the row is budgeted by.
    fn cell(self, panel: &Panel, entry: &DirEntry) -> String {
        crate::ui::printable_name(&self.raw_cell(panel, entry)).into_owned()
    }

    fn raw_cell(self, panel: &Panel, entry: &DirEntry) -> String {
        match self {
            Self::Size => size_field(entry),
            Self::GroupedSize => sized(entry, grouped_bytes(entry.size.bytes())),
            Self::Allocated => sized(entry, fitted_bytes(entry.allocated.bytes())),
            Self::Date => entry.modified.date(),
            Self::Modified => stamp(entry.modified),
            Self::Created => stamp(entry.created),
            Self::Accessed => stamp(entry.accessed),
            Self::Attributes => entry.attributes.to_string(),
            Self::Description => panel
                .details
                .descriptions
                .get(&entry.name)
                .or(entry.description.as_ref())
                .cloned()
                .unwrap_or_default(),
            Self::Owner => panel
                .details
                .owners
                .get(&entry.name)
                .or(entry.owner.as_ref())
                .cloned()
                .unwrap_or_default(),
            Self::Links => panel
                .details
                .links
                .get(&entry.name)
                .cloned()
                .unwrap_or_else(|| entry.links.to_string()),
        }
    }

    fn align(self, text: &str, width: Cells) -> String {
        match self {
            Self::Attributes | Self::Description | Self::Owner => {
                pad_cells(&head_cells(text, width), width)
            }
            _ => right_cells(text, width),
        }
    }
}

const fn columns(view: PanelView) -> &'static [Column] {
    use Column::{Accessed, Allocated, Attributes, Created, Date, Description};
    use Column::{GroupedSize, Links, Modified, Owner, Size};
    match view {
        PanelView::Detailed => &[Size, Allocated, Modified, Created, Accessed, Attributes],
        PanelView::Descriptions => &[Description],
        PanelView::LongDescriptions => &[Size, Description],
        PanelView::Owners => &[Size, Owner],
        PanelView::Links => &[Size, Links],
        PanelView::Alternative => &[GroupedSize, Date],
        PanelView::Brief | PanelView::Medium | PanelView::Full | PanelView::Wide => &[],
    }
}

struct Placed {
    column: Column,
    width: Cells,
}

pub(super) struct Layout {
    name: Cells,
    columns: Vec<Placed>,
}

impl Layout {
    pub(super) fn new(view: PanelView, width: Cells) -> Self {
        let wanted = columns(view);
        (1..=wanted.len())
            .rev()
            .filter_map(|taken| wanted.get(..taken))
            .map(|columns| Self::fit(view, columns, width))
            .find(|layout| layout.name >= Cells::new(8))
            .unwrap_or_else(|| Self::fit(view, &[], width))
    }

    fn fit(view: PanelView, columns: &[Column], width: Cells) -> Self {
        let fixed: Cells = columns
            .iter()
            .map(|column| column.width().unwrap_or(Cells::ZERO) + Cells::new(1))
            .sum();
        let remaining = width.saturating_sub(fixed);
        let name = if columns.iter().any(|column| column.width().is_none()) {
            let divisor = if view == PanelView::LongDescriptions {
                3
            } else {
                2
            };
            Cells::new(remaining.get().div_ceil(divisor))
        } else {
            remaining
        };
        let columns = columns
            .iter()
            .map(|&column| Placed {
                column,
                width: column
                    .width()
                    .unwrap_or_else(|| remaining.saturating_sub(name)),
            })
            .collect();
        Self { name, columns }
    }

    pub(super) fn header(&self) -> String {
        let mut text = center_cells("Name", self.name);
        for placed in &self.columns {
            text.push('│');
            text.push_str(&center_cells(placed.column.title(), placed.width));
        }
        text
    }

    pub(super) fn row(&self, panel: &Panel, entry: &DirEntry, name: &str) -> String {
        let mut text = pad_cells(&clip_name(name, self.name), self.name);
        for placed in &self.columns {
            text.push('│');
            let cell = placed.column.cell(panel, entry);
            text.push_str(&placed.column.align(&cell, placed.width));
        }
        text
    }
}

fn sized(entry: &DirEntry, bytes: String) -> String {
    if entry.name == ".." || entry.kind.is_dir() {
        size_field(entry)
    } else {
        bytes
    }
}

fn fitted_bytes(bytes: u64) -> String {
    let exact = bytes.to_string();
    if exact.len() <= 6 {
        exact
    } else {
        abbreviate_bytes(bytes)
    }
}

fn grouped_bytes(bytes: u64) -> String {
    let grouped = group_digits(bytes);
    if grouped.len() <= 10 {
        grouped
    } else {
        abbreviate_bytes(bytes)
    }
}

fn stamp(at: Modified) -> String {
    let date = at.date();
    if date.is_empty() {
        String::new()
    } else {
        format!("{date} {}", at.time())
    }
}
