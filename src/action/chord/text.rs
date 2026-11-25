//! Purpose: the text syntax of a chord — `Ctrl+Alt+Shift+Key` — read from
//! `ffm.yml` and written to the file, the report, menus, key bars and hints.
//!
//! The canonical text (`Display`) round-trips through `FromStr`: modifiers
//! in the order Ctrl, Alt, Shift; a letter under Ctrl or Alt upper-case,
//! as Far writes it.

use std::fmt;
use std::str::FromStr;

use super::{lowercase, uppercase, FunctionKey, Key, KeyChord, Modifier, Modifiers, NamedKey};

/// Why a text is not a chord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// An empty string.
    Empty,
    /// `Ctrl+`: a modifier with nothing after it.
    NoKey(Modifier),
    /// `Ctrl+Ctrl+X`.
    Repeated(Modifier),
    /// `Shift+1`, `Shift+Space`: Shift is part of the character it types,
    /// so the character itself is what to write.
    ShiftedCharacter(Key),
    /// `F0`, `F25`.
    FunctionKeyRange(String),
    /// A space, a control character or another key that cannot be typed.
    NotPrintable(String),
    /// A name nobody knows: `Foo`, `Ctrl+Alt`.
    UnknownKey(String),
}

impl fmt::Display for KeyError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => out.write_str("no key"),
            Self::NoKey(modifier) => write!(out, "no key after {modifier}+"),
            Self::Repeated(modifier) => write!(out, "{modifier} is given twice"),
            Self::ShiftedCharacter(key) => write!(
                out,
                "Shift+{key}: Shift is part of the character; write the character itself"
            ),
            Self::FunctionKeyRange(text) => {
                write!(
                    out,
                    "{text}: function keys are F1 to F{}",
                    FunctionKey::LAST
                )
            }
            Self::NotPrintable(text) => {
                write!(
                    out,
                    "'{text}' is not a printable key (the space bar is Space)"
                )
            }
            Self::UnknownKey(text) => write!(out, "unknown key '{text}'"),
        }
    }
}

impl std::error::Error for KeyError {}

impl FromStr for KeyChord {
    type Err = KeyError;

    fn from_str(text: &str) -> Result<Self, KeyError> {
        if text.is_empty() {
            return Err(KeyError::Empty);
        }
        let (modifiers, key_text) = split_modifiers(text)?;
        let key = parse_key(key_text)?;
        fold_shift(key, modifiers)
    }
}

/// The leading `Ctrl+`, `Alt+` and `Shift+` (any case, any order, each at
/// most once) and the text after them, which is the key: what follows the
/// last separating `+`, so `Alt++` is Alt and the plus key.
fn split_modifiers(text: &str) -> Result<(Modifiers, &str), KeyError> {
    let mut modifiers = Modifiers::NONE;
    let mut rest = text;
    while let Some((modifier, after)) = leading_modifier(rest) {
        if modifiers.contains(modifier) {
            return Err(KeyError::Repeated(modifier));
        }
        if after.is_empty() {
            return Err(KeyError::NoKey(modifier));
        }
        modifiers = modifiers.with(modifier);
        rest = after;
    }
    Ok((modifiers, rest))
}

/// The modifier `text` starts with, when a `+` follows it, and the text
/// after that `+`.
fn leading_modifier(text: &str) -> Option<(Modifier, &str)> {
    Modifier::ALL.into_iter().find_map(|modifier| {
        let name = modifier.name();
        let (head, after) = text.split_at_checked(name.len())?;
        if !head.eq_ignore_ascii_case(name) {
            return None;
        }
        after.strip_prefix('+').map(|after| (modifier, after))
    })
}

/// The key the text after the modifiers names.
fn parse_key(text: &str) -> Result<Key, KeyError> {
    if let Some(named) = NamedKey::parse(text) {
        return Ok(Key::Named(named));
    }
    if let Some(modifier) = Modifier::parse(text) {
        return Err(KeyError::NoKey(modifier));
    }
    let mut characters = text.chars();
    match (characters.next(), characters.next()) {
        (Some(character), None) if is_printable(character) => Ok(Key::Char(character)),
        (Some(_), None) => Err(KeyError::NotPrintable(text.to_owned())),
        _ => function_key(text),
    }
}

/// A character a binding can name: not a control character, not a blank,
/// and not one of the invisible format characters — a soft hyphen, a
/// zero-width space or joiner, a bidi mark, a word joiner, a byte-order
/// mark — that a copy from a browser or an editor pastes and
/// `char::is_control` does not name.
fn is_printable(character: char) -> bool {
    let invisible = matches!(
        character,
        '\u{AD}' | '\u{200B}'..='\u{200F}' | '\u{2028}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}'
    );
    !character.is_control() && !character.is_whitespace() && !invisible
}

/// `F` and one or two digits, F1 to F24.
fn function_key(text: &str) -> Result<Key, KeyError> {
    let digits = text
        .strip_prefix(['F', 'f'])
        .filter(|digits| (1..=2).contains(&digits.len()))
        .filter(|digits| digits.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| KeyError::UnknownKey(text.to_owned()))?;
    digits
        .parse::<u8>()
        .ok()
        .and_then(FunctionKey::new)
        .map(Key::F)
        .ok_or_else(|| KeyError::FunctionKeyRange(text.to_owned()))
}

/// Shift's place in the chord. Under Ctrl a letter's case does not matter
/// and Shift stays written out; so does a letter under Alt. Any other Shift
/// is part of the character: `Shift+k` is `K`, and `Shift+1`, `Shift+Space`
/// or `Alt+Shift+1` is an error, because what Shift produces there depends
/// on the layout, and every terminal reports the character it produced.
fn fold_shift(key: Key, modifiers: Modifiers) -> Result<KeyChord, KeyError> {
    let key = match key {
        Key::Char(character) if modifiers.ctrl_or_alt() => {
            Key::Char(lowercase(character).unwrap_or(character))
        }
        other => other,
    };
    if !modifiers.contains(Modifier::Shift) || modifiers.contains(Modifier::Ctrl) {
        return Ok(KeyChord { key, modifiers });
    }
    match key {
        Key::Char(character) if modifiers.contains(Modifier::Alt) => lowercase(character)
            .map(|_| KeyChord { key, modifiers })
            .ok_or(KeyError::ShiftedCharacter(key)),
        Key::Char(character) => uppercase(character)
            .map(|upper| KeyChord {
                key: Key::Char(upper),
                modifiers: Modifiers::NONE,
            })
            .ok_or(KeyError::ShiftedCharacter(key)),
        Key::Named(NamedKey::Space) => Err(KeyError::ShiftedCharacter(key)),
        Key::F(_) | Key::Named(_) => Ok(KeyChord { key, modifiers }),
    }
}

impl fmt::Display for KeyChord {
    /// The canonical text: `Ctrl+Alt+Shift+Key`, a letter under Ctrl or
    /// Alt upper-case.
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        for modifier in self.modifiers.iter() {
            write!(out, "{modifier}+")?;
        }
        match self.key {
            Key::Char(character) if self.modifiers.ctrl_or_alt() => {
                write!(out, "{}", uppercase(character).unwrap_or(character))
            }
            key => write!(out, "{key}"),
        }
    }
}
