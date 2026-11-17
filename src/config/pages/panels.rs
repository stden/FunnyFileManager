//! Purpose: The pages the file panels read: panel, tree, info panel, mask
//! groups, the ten panel modes, and file and folder descriptions.

use serde::{Deserialize, Serialize};

use crate::models::PanelView;

/// Panel settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent Far panel options, each saved as its own true/false"
)]
pub struct PanelSettings {
    pub show_hidden: bool,
    pub dirs_first: bool,
    pub case_sensitive_sort: bool,
    pub show_column_titles: bool,
    pub show_status_line: bool,
    pub show_free_size: bool,
    pub highlight_files: bool,
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            show_hidden: true,
            dirs_first: true,
            case_sensitive_sort: false,
            show_column_titles: true,
            show_status_line: true,
            show_free_size: true,
            highlight_files: true,
        }
    }
}

/// Tree settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TreeSettings {
    /// Moving the tree cursor changes the other panel's folder (Far: on).
    pub auto_change_folder: bool,
    /// Cells the tree panel never shrinks below.
    pub min_width: u32,
}

impl Default for TreeSettings {
    fn default() -> Self {
        Self {
            auto_change_folder: true,
            min_width: 20,
        }
    }
}

/// `InfoPanel` settings — the sections `Ctrl+L` shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InfoPanelSettings {
    pub show_kinds: bool,
    pub show_largest: bool,
    pub largest_count: u32,
}

impl Default for InfoPanelSettings {
    fn default() -> Self {
        Self {
            show_kinds: true,
            show_largest: true,
            largest_count: 8,
        }
    }
}

/// Groups of file masks: name → masks, usable as `<name>` in a mask.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MaskGroupSettings {
    pub groups: Vec<(String, String)>,
}

/// One of the ten file panel modes: Far's column letters and whether the
/// mode takes the whole screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelModeSettings {
    /// Far's letters: `N` name, `S` size, `P` packed size, `D` date, `T`
    /// time, `DC`/`TC` creation, `DA`/`TA` access, `A` attributes, `Z`
    /// description, `O` owner, `LN` link count.
    pub columns: String,
    pub fullscreen: bool,
}

impl Default for PanelModeSettings {
    fn default() -> Self {
        PanelView::Full.default_mode()
    }
}

impl PanelView {
    /// Far's column set for the mode, and whether it fills the screen.
    #[must_use]
    pub fn default_mode(self) -> PanelModeSettings {
        let columns = match self {
            Self::Brief | Self::Medium => "N",
            Self::Full => "N,S,D,T",
            Self::Wide => "N,S",
            Self::Detailed => "N,S,P,D,T,DC,TC,DA,TA,A",
            Self::Descriptions => "N,Z",
            Self::LongDescriptions => "N,S,Z",
            Self::Owners => "N,S,O",
            Self::Links => "N,S,LN",
            Self::Alternative => "N,S,D",
        };
        PanelModeSettings {
            columns: columns.to_owned(),
            fullscreen: self.is_fullscreen(),
        }
    }

    /// The mode's position in [`PanelView::ALL`], which is also its slot in
    /// [`PanelModesSettings::modes`].
    #[must_use]
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|view| *view == self)
            .unwrap_or_default()
    }
}

/// File panel modes, one entry per [`PanelView`] in `PanelView::ALL` order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelModesSettings {
    pub modes: [PanelModeSettings; 10],
}

impl Default for PanelModesSettings {
    fn default() -> Self {
        Self {
            modes: PanelView::ALL.map(PanelView::default_mode),
        }
    }
}

/// File descriptions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DescriptionSettings {
    /// Description files, in search order.
    pub names: Vec<String>,
    pub update_on_copy: bool,
    pub update_on_move: bool,
}

impl Default for DescriptionSettings {
    fn default() -> Self {
        Self {
            names: crate::description::default_names(),
            update_on_copy: true,
            update_on_move: true,
        }
    }
}

/// Folder description files, shown for directories in the Descriptions modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FolderDescriptionSettings {
    pub names: Vec<String>,
}

impl Default for FolderDescriptionSettings {
    fn default() -> Self {
        Self {
            names: vec!["Dirinfo".to_owned(), "Folder.txt".to_owned()],
        }
    }
}
