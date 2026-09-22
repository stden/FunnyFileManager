//! Purpose: The pages the shell around the panels reads: dialogs, menus,
//! the command line, autocomplete, and confirmations.

use serde::{Deserialize, Serialize};

/// Dialog settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DialogSettings {
    /// Name and mask prompts reopen with their last value.
    pub remember_history: bool,
    pub click_outside_closes: bool,
}

impl Default for DialogSettings {
    fn default() -> Self {
        Self {
            remember_history: true,
            click_outside_closes: false,
        }
    }
}

/// Menu settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MenuSettings {
    /// Hotkey letters underlined.
    pub far_letters: bool,
    /// `Up` on the first row wraps to the last.
    pub wrap_cursor: bool,
}

impl Default for MenuSettings {
    fn default() -> Self {
        Self {
            far_letters: true,
            wrap_cursor: true,
        }
    }
}

/// Command line settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CommandLineSettings {
    /// Far's prompt segments: `$p` path, `$b` branch, `$u` user, `$h` host,
    /// `$g` the sigil.
    pub prompt_format: String,
}

impl Default for CommandLineSettings {
    fn default() -> Self {
        Self {
            prompt_format: "$p$g".to_owned(),
        }
    }
}

/// `AutoComplete` settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoCompleteSettings {
    /// Ask the fish shell for completions.
    pub fish: bool,
    /// The grey suggestion after the caret.
    pub ghost: bool,
    /// The drop-down on `Tab`.
    pub menu: bool,
}

impl Default for AutoCompleteSettings {
    fn default() -> Self {
        Self {
            fish: false,
            ghost: true,
            menu: true,
        }
    }
}

/// Confirmations — each read where its question is asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfirmationSettings {
    pub copy_overwrite: bool,
    pub move_overwrite: bool,
    pub delete: bool,
    pub delete_folders: bool,
    pub exit: bool,
    pub clear_history: bool,
}

impl Default for ConfirmationSettings {
    fn default() -> Self {
        Self {
            copy_overwrite: true,
            move_overwrite: true,
            delete: true,
            delete_folders: true,
            // `F10` has always quit at once here; keep it so until asked.
            exit: false,
            clear_history: true,
        }
    }
}
