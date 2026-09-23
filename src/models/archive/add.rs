//! Purpose: `Shift+F1`'s Add to archive dialog — its tab ring, the format radio that renames the archive, and the name it offers.

use std::path::Path;

use super::ArchiveFormat;

/// Which control in the Add to archive dialog has the cursor.
///
/// An enum rather than an index so the tab order is stated once, here, and a
/// control added in the middle cannot silently renumber the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchiveField {
    #[default]
    Name,
    Format,
    DeleteAfter,
    Add,
    Cancel,
}

impl ArchiveField {
    /// The tab order, top to bottom and then along the buttons.
    const ORDER: [Self; 5] = [
        Self::Name,
        Self::Format,
        Self::DeleteAfter,
        Self::Add,
        Self::Cancel,
    ];

    #[must_use]
    pub fn next(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, true)
    }

    #[must_use]
    pub fn previous(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, false)
    }

    /// Whether this control is a checkbox, and so answers to the space bar.
    #[must_use]
    pub const fn is_checkbox(self) -> bool {
        matches!(self, Self::DeleteAfter)
    }
}

/// `Shift+F1`: what to call the archive, how to pack it, and whether the
/// originals go.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchivePrompt {
    /// The archive to create, offered as `<name>.tar.gz` in the other panel.
    /// A relative name lands in the active panel's folder.
    pub name: String,
    pub format: ArchiveFormat,
    /// Far's "Delete files after archiving": the sources go once the archive
    /// is written, and only then.
    pub delete_after: bool,
    pub field: ArchiveField,
    /// The formats this machine can write, in radio order. `zip` is offered
    /// only where it is installed: a radio button that fails when pressed is
    /// worse than one that is not there.
    pub offered: Vec<ArchiveFormat>,
    /// What goes in, as names relative to the active panel's folder.
    pub sources: Vec<String>,
    /// `name` is an archive that already exists and the sources join it —
    /// the Archive commands row, rather than `Shift+F1`'s new archive.
    pub append: bool,
}

impl ArchivePrompt {
    /// Open the dialog offering `<stem>.<first format>` inside `folder`.
    #[must_use]
    pub fn new(
        sources: Vec<String>,
        stem: &str,
        folder: &Path,
        offered: Vec<ArchiveFormat>,
    ) -> Self {
        let format = offered.first().copied().unwrap_or_default();
        let name = folder
            .join(format!("{stem}.{}", format.extension()))
            .display()
            .to_string();
        Self {
            name,
            format,
            offered,
            sources,
            ..Self::default()
        }
    }

    /// Open the dialog on an existing archive the sources are to join. The
    /// radio offers the archive's own format and nothing else: turning it
    /// would rename an archive that is already there.
    #[must_use]
    pub fn appending(sources: Vec<String>, archive: &Path, format: ArchiveFormat) -> Self {
        Self {
            name: archive.display().to_string(),
            format,
            offered: vec![format],
            sources,
            append: true,
            ..Self::default()
        }
    }

    /// The name Far's archiver plugin would offer: one file's name without
    /// its extension, one directory's whole name, and for several the folder
    /// they are in — the one name that describes all of them.
    #[must_use]
    pub fn suggest_stem(sources: &[String], single_is_dir: bool, folder_name: &str) -> String {
        match sources {
            [one] if single_is_dir => one.clone(),
            [one] => one
                .rsplit_once('.')
                // A leading dot starts a name, not an extension.
                .filter(|(stem, _)| !stem.is_empty())
                .map_or_else(|| one.clone(), |(stem, _)| stem.to_owned()),
            _ => folder_name.to_owned(),
        }
    }

    /// The caption over the name field: the one thing being packed, or how
    /// many.
    #[must_use]
    pub fn caption(&self) -> String {
        match self.sources.as_slice() {
            [one] => format!("Add \"{one}\" to the archive:"),
            many => format!("Add {} items to the archive:", many.len()),
        }
    }

    /// Where the chosen format sits among the offered ones, for the radio.
    #[must_use]
    pub fn format_index(&self) -> usize {
        self.offered
            .iter()
            .position(|format| *format == self.format)
            .unwrap_or(0)
    }

    /// Tab / Shift+Tab: the next or previous control, wrapping.
    pub fn advance(&mut self, forward: bool) {
        self.field = if forward {
            self.field.next()
        } else {
            self.field.previous()
        };
    }

    /// Space on the checkbox. Does nothing elsewhere, so the space bar stays
    /// a space in the name field.
    pub fn toggle(&mut self) {
        if self.field.is_checkbox() {
            self.delete_after = !self.delete_after;
        }
    }

    /// Left/Right on the radio: the next offered format, and the name's
    /// extension follows it. The name and the format must agree, and making
    /// the user retype the extension is how they stop agreeing.
    pub fn cycle(&mut self, forward: bool) {
        if self.field != ArchiveField::Format {
            return;
        }
        let previous = self.format;
        self.format = crate::cycle::step(&self.offered, previous, forward);
        let old_suffix = format!(".{}", previous.extension());
        if let Some(stem) = self.name.strip_suffix(&old_suffix) {
            self.name = format!("{stem}.{}", self.format.extension());
        }
    }

    /// Whether the dialog can be accepted. An empty name cannot.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        !self.name.trim().is_empty()
    }
}
