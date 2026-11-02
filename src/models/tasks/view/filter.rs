//! Purpose: Which processes the view admits by owner and by text: the user
//! scope `u` chooses and the live filter `F4` edits.

use crate::casefold::contains_ignore_case;

use super::super::{Owner, ProcessSample};

/// Whose processes are listed: `u` chooses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UserScope {
    #[default]
    All,
    Only(Owner),
}

impl UserScope {
    pub(super) fn admits(&self, owner: &Owner) -> bool {
        match self {
            Self::All => true,
            Self::Only(only) => only == owner,
        }
    }
}

/// The live filter (`F4`): fixed strings separated by `|`, any of which a
/// process's name or command line must contain, ignoring case.
///
/// Keeps the line as typed beside the terms it parsed to, so the bar reopens
/// on what the user wrote and the footer names it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterTerms {
    line: String,
    /// Trimmed, lowercased and never empty.
    terms: Vec<String>,
}

impl FilterTerms {
    /// The filter `line` spells. Blank terms are dropped, so `fire||` is one
    /// term and a line of bars and blanks filters nothing.
    #[must_use]
    pub fn parse(line: &str) -> Self {
        let terms = line
            .split('|')
            .map(str::trim)
            .filter(|term| !term.is_empty())
            .map(str::to_lowercase);
        Self {
            line: line.to_owned(),
            terms: terms.collect(),
        }
    }

    /// The line as typed.
    #[must_use]
    pub fn line(&self) -> &str {
        &self.line
    }

    /// Whether the filter lets every process through.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Whether any term is part of `sample`'s name or command line.
    #[must_use]
    pub fn matches(&self, sample: &ProcessSample) -> bool {
        self.is_empty()
            || self.terms.iter().any(|term| {
                contains_ignore_case(&sample.name, term)
                    || contains_ignore_case(&sample.command, term)
            })
    }
}
