//! Purpose: The interface in the user's language — `Settings.languages`.
//!
//! One lookup, [`text`], stands between every English string on screen and
//! its translation, so that switching the language is a setting and not a
//! rebuild. Only English exists until the Languages page lands; a language
//! with no translation shows the English text, never a blank, because a
//! label that vanished is worse than one in the wrong language.

use crate::config::Lang;

/// `english`, in `lang`.
#[must_use]
pub const fn text(lang: Lang, english: &'static str) -> &'static str {
    match lang {
        Lang::En | Lang::Ru => english,
    }
}

#[cfg(test)]
mod tests;
