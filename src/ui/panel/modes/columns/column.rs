//! Purpose: One detail column of the one-column panel views — its width,
//! title and alignment, and its cell's text from the cached directory facts,
//! made printable — and the columns each view asks for.

use crate::models::{DirEntry, EntryKind, Modified, Panel, PanelView};

use super::super::super::{
    abbreviate_bytes, group_digits, head_cells, pad_cells, right_cells, size_field, Cells,
};

#[derive(Clone, Copy)]
pub(super) enum Column {
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
    pub(super) const fn width(self) -> Option<Cells> {
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

    pub(super) const fn title(self) -> &'static str {
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
    pub(super) fn cell(self, panel: &Panel, entry: &DirEntry) -> String {
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

    pub(super) fn align(self, text: &str, width: Cells) -> String {
        match self {
            Self::Attributes | Self::Description | Self::Owner => {
                pad_cells(&head_cells(text, width), width)
            }
            _ => right_cells(text, width),
        }
    }
}

pub(super) const fn columns(view: PanelView) -> &'static [Column] {
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

/// `bytes` where a byte count describes the row: a file, or a link to one.
/// The rest say what they are, as the Size column does.
fn sized(entry: &DirEntry, bytes: String) -> String {
    match (entry.kind, entry.directory_size) {
        (EntryKind::File | EntryKind::SymlinkToFile, _)
        | (
            EntryKind::Directory | EntryKind::SymlinkToDirectory,
            crate::models::DirectorySize::Measured(_),
        ) => bytes,
        (
            EntryKind::Directory | EntryKind::SymlinkToDirectory,
            crate::models::DirectorySize::Unmeasured,
        )
        | (EntryKind::BrokenSymlink, _) => size_field(entry),
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

#[cfg(test)]
mod tests_widths;
