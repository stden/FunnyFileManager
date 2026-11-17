//! Purpose: One detail column of the one-column panel views — its width,
//! title and alignment, and its cell's text from the cached directory facts,
//! made printable — and the columns each view asks for.

use std::borrow::Cow;

use crate::models::{DirEntry, Panel, PanelView};

use super::super::super::{
    abbreviate_bytes,
    fields::{fitted_bytes, non_numeric_size_label},
    group_digits, pad_head_cells, right_cells, size_field, Cells,
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

    pub(super) const fn is_flexible(self) -> bool {
        self.width().is_none()
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

    /// The text of one detail cell, skipping `printable_name` on guaranteed-ASCII fields.
    pub(super) fn cell<'a>(self, panel: &'a Panel, entry: &'a DirEntry) -> Cow<'a, str> {
        match self {
            Self::Size => size_field(entry),
            Self::GroupedSize => sized(entry, entry.size.bytes(), grouped_bytes),
            Self::Allocated => sized(entry, entry.allocated.bytes(), fitted_bytes),
            Self::Date => Cow::Owned(entry.modified.date()),
            Self::Modified => Cow::Owned(entry.modified.stamp()),
            Self::Created => Cow::Owned(entry.created.stamp()),
            Self::Accessed => Cow::Owned(entry.accessed.stamp()),
            Self::Attributes => Cow::Owned(entry.attributes.to_string()),
            Self::Description => panel
                .details
                .descriptions
                .get(&entry.name)
                .or(entry.description.as_ref())
                .map_or(Cow::Borrowed(""), |s| crate::ui::printable_name(s)),
            Self::Owner => panel
                .details
                .owners
                .get(&entry.name)
                .or(entry.owner.as_ref())
                .map_or(Cow::Borrowed(""), |s| crate::ui::printable_name(s)),
            Self::Links => match panel.details.links.get(&entry.name) {
                Some(link) => crate::ui::printable_name(link),
                None => Cow::Owned(entry.links.to_string()),
            },
        }
    }

    pub(super) fn align(self, text: &str, width: Cells) -> String {
        match self {
            Self::Attributes | Self::Description | Self::Owner => pad_head_cells(text, width),
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

/// Format `bytes` where a byte count describes the row, or return the static
/// kind label (`"Up"`, `"Folder"`, `"Link"`, `"Broken"`) without allocating.
fn sized(entry: &DirEntry, bytes: u64, format_bytes: fn(u64) -> String) -> Cow<'static, str> {
    if let Some(label) = non_numeric_size_label(entry) {
        return Cow::Borrowed(label);
    }
    Cow::Owned(format_bytes(bytes))
}

fn grouped_bytes(bytes: u64) -> String {
    let grouped = group_digits(bytes);
    if grouped.len() <= 10 {
        grouped
    } else {
        abbreviate_bytes(bytes)
    }
}

#[cfg(test)]
mod tests_widths;
