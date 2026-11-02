//! File panel presentation modes selected from the F9 panel menus.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct PanelDetails {
    pub descriptions: BTreeMap<String, String>,
    pub owners: BTreeMap<String, String>,
    pub links: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct PanelOptions {
    pub short_names: bool,
    pub hide_hidden: bool,
    pub filter: String,
    pub description_file: String,
}

impl super::Panel {
    pub(crate) fn is_filtered(&self) -> bool {
        self.filter.is_active() || !self.options.filter.is_empty()
    }

    pub(super) fn filter_listing(&self, listing: &mut super::DirectoryListing) {
        let filter = crate::mask::Mask::parse(&self.options.filter);
        listing.entries.retain(|entry| {
            let hidden = entry.name.starts_with('.')
                || listing.metadata.hidden_names.contains(entry.name.as_str());
            (!self.options.hide_hidden || !hidden)
                && (self.options.filter.is_empty()
                    || entry.kind.is_dir()
                    || filter.matches(&entry.name))
        });
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PanelView {
    Brief,
    Medium,
    #[default]
    Full,
    Wide,
    Detailed,
    Descriptions,
    LongDescriptions,
    Owners,
    Links,
    Alternative,
}

impl PanelView {
    #[must_use]
    pub const fn columns(self) -> usize {
        match self {
            Self::Brief => 3,
            Self::Medium => 2,
            _ => 1,
        }
    }
    pub const ALL: [Self; 10] = [
        Self::Brief,
        Self::Medium,
        Self::Full,
        Self::Wide,
        Self::Detailed,
        Self::Descriptions,
        Self::LongDescriptions,
        Self::Owners,
        Self::Links,
        Self::Alternative,
    ];

    /// Whether the layout takes the whole screen rather than half of it.
    #[must_use]
    pub const fn is_fullscreen(self) -> bool {
        matches!(self, Self::Detailed | Self::LongDescriptions)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Brief => "Brief",
            Self::Medium => "Medium",
            Self::Full => "Full",
            Self::Wide => "Wide",
            Self::Detailed => "Detailed",
            Self::Descriptions => "Descriptions",
            Self::LongDescriptions => "Long descriptions",
            Self::Owners => "File owners",
            Self::Links => "File links",
            Self::Alternative => "Alternative full",
        }
    }
}
