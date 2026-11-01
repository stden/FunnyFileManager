//! Purpose: Far's "File already exists" question: a copy or move whose
//! destination already holds a name, the facts of both entries side by side,
//! and the answers given so far.

use std::path::PathBuf;

use crate::copy::{CopyPrompt, ExistingFiles};

mod clash;
pub use clash::{Clash, EntryFacts};

/// An answer to the question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClashAnswer {
    /// Replace what is there; two folders merge.
    Overwrite,
    /// Leave what is there; two folders merge, keeping the files already
    /// inside.
    Skip,
    /// Keep both: the new one lands beside under a free name.
    KeepBoth,
}

impl ClashAnswer {
    /// The "already existing files" choice that carries the answer out.
    #[must_use]
    pub const fn policy(self) -> ExistingFiles {
        match self {
            Self::Overwrite => ExistingFiles::Overwrite,
            Self::Skip => ExistingFiles::Skip,
            // "Ask" is what places an entry beside one already there.
            Self::KeepBoth => ExistingFiles::Ask,
        }
    }
}

/// The control with the cursor, in the order Tab visits them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClashFocus {
    Remember,
    #[default]
    Overwrite,
    Skip,
    KeepBoth,
    Cancel,
}

impl ClashFocus {
    const ORDER: [Self; 5] = [
        Self::Remember,
        Self::Overwrite,
        Self::Skip,
        Self::KeepBoth,
        Self::Cancel,
    ];

    #[must_use]
    pub fn next(self, forward: bool) -> Self {
        let len = Self::ORDER.len();
        let at = Self::ORDER.iter().position(|&it| it == self).unwrap_or(0);
        let to = if forward { at + 1 } else { at + len - 1 };
        Self::ORDER.get(to % len).copied().unwrap_or(self)
    }
}

/// A copy or move held back until every clash has an answer.
///
/// Nothing is transferred while the question is open, so Cancel leaves both
/// folders exactly as they were.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingFile {
    /// What the F5/F6 dialog was filled in with.
    pub settings: CopyPrompt,
    pub sources: Vec<PathBuf>,
    /// The folder a typed destination is relative to.
    pub base: PathBuf,
    clashes: Vec<Clash>,
    /// Index into `clashes` of the one being asked about.
    current: usize,
    answers: Vec<(PathBuf, ExistingFiles)>,
    /// "Remember choice": the answer goes for every clash still to come.
    pub remember: bool,
    pub focus: ClashFocus,
}

impl ExistingFile {
    /// Ask about `clashes`, which must not be empty, before the transfer
    /// `settings` describe.
    #[must_use]
    pub fn new(
        settings: CopyPrompt,
        sources: Vec<PathBuf>,
        base: PathBuf,
        clashes: Vec<Clash>,
    ) -> Self {
        Self {
            settings,
            sources,
            base,
            clashes,
            current: 0,
            answers: Vec::new(),
            remember: false,
            focus: ClashFocus::default(),
        }
    }

    /// The clash being asked about.
    #[must_use]
    pub fn clash(&self) -> Option<&Clash> {
        self.clashes.get(self.current)
    }

    /// How many clashes there are, and which one (from 1) is shown.
    #[must_use]
    pub fn position(&self) -> (usize, usize) {
        (self.current + 1, self.clashes.len())
    }

    /// Record `answer` for the current clash, or for it and every one after
    /// it when "Remember choice" is ticked. True when nothing is left to ask.
    pub fn answer(&mut self, answer: ClashAnswer) -> bool {
        let policy = answer.policy();
        let last = if self.remember {
            self.clashes.len()
        } else {
            self.current + 1
        };
        for clash in self.clashes.get(self.current..last).unwrap_or_default() {
            self.answers.push((clash.source.clone(), policy));
        }
        self.current = last;
        self.focus = ClashFocus::default();
        self.current >= self.clashes.len()
    }

    /// The hotkey of the `index`-th button — Overwrite, Skip, Keep both,
    /// Cancel — so a click is answered by the key that already does the job.
    #[must_use]
    pub const fn button_hotkey(index: usize) -> char {
        match index {
            0 => 'o',
            1 => 's',
            2 => 'k',
            _ => 'c',
        }
    }

    /// What was answered for each clashing source.
    #[must_use]
    pub fn answers(&self) -> &[(PathBuf, ExistingFiles)] {
        &self.answers
    }
}

#[cfg(test)]
mod tests;
