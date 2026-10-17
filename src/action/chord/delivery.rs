//! Purpose: where a chord does not arrive as itself — the byte collisions of
//! terminals without the kitty protocol (the parity doc's "Terminal Key
//! Encoding"), the keys a terminal or the desktop takes first, Windows
//! `AltGr`, and the symbols Shift produces — so that the keymap report can
//! warn about a chord the file introduces and a default's comment can say
//! what a terminal sends instead.

use super::{lowercase, Key, KeyChord, Modifier, Modifiers, NamedKey};

/// Who commonly takes a chord before the program sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taker {
    /// The terminal emulator: F10 opens GNOME Terminal's menu, F11 its full
    /// screen and Alt+Enter some others', Shift+PgUp/PgDn scroll its
    /// history. Its Ctrl+Shift+C/V/T/N/W clipboard and tab keys are not
    /// here: every Ctrl+Shift+letter is already a [`Delivery::LegacyAs`].
    Terminal,
    /// The window manager or an input method: Alt+F4, Alt+Space,
    /// Ctrl+Alt+Del; Ctrl+Space switches input sources (`IBus`, macOS).
    Desktop,
    /// macOS: Option types a character unless "Option as Meta" is on, and
    /// Ctrl+arrows belong to Mission Control.
    MacOs,
}

/// Whether every terminal delivers a chord as itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Every terminal sends the chord as itself.
    Everywhere,
    /// A terminal without the kitty protocol sends this chord instead:
    /// Ctrl+I is Tab's byte, Ctrl+] is Ctrl+5's, Ctrl+Shift+K loses Shift.
    LegacyAs(KeyChord),
    /// A terminal without the kitty protocol sends nothing FFM can read:
    /// xterm's modified F3 is a cursor-position report; F13 and above are
    /// not among its keys.
    LegacyLost,
    /// The chord arrives where nothing else took it first.
    TakenFirst(Taker),
    /// On Windows, Ctrl+Alt with a symbol is what `AltGr` types.
    AltGr,
    /// Shift produces the symbol, and terminals disagree about the report:
    /// kitty sends `+` under Ctrl, Windows `=` with Shift, a terminal
    /// without the protocol no control byte at all.
    ShiftedSymbol,
}

impl Delivery {
    /// Whether a terminal without the kitty protocol delivers the chord as
    /// itself — the test an exit action's last key must pass.
    #[must_use]
    pub const fn on_legacy(self) -> bool {
        matches!(self, Self::Everywhere | Self::TakenFirst(_) | Self::AltGr)
    }
}

impl KeyChord {
    /// Where this chord does not arrive as itself. One answer per chord,
    /// the worst first: a key that never arrives outranks one the desktop
    /// may take, which outranks a symbol terminals report differently.
    #[must_use]
    pub fn delivery(self) -> Delivery {
        legacy(self)
            .or_else(|| taken_first(self))
            .or_else(|| alt_gr(self))
            .or_else(|| shifted_symbol(self))
            .unwrap_or(Delivery::Everywhere)
    }
}

/// What a terminal without the kitty protocol makes of the chord.
fn legacy(chord: KeyChord) -> Option<Delivery> {
    let modifiers = chord.modifiers;
    match chord.key {
        Key::F(function_key) if function_key.number() > 12 => Some(Delivery::LegacyLost),
        Key::F(function_key) if function_key.number() == 3 && !modifiers.is_empty() => {
            Some(Delivery::LegacyLost)
        }
        Key::Char(character) if modifiers.contains(Modifier::Ctrl) => {
            legacy_control(character, modifiers)
        }
        _ => None,
    }
}

/// Ctrl and a character as VT bytes: Shift does not reach the byte, so a
/// letter loses it, and a key whose byte crossterm reads as another key
/// arrives as that key with or without Shift.
fn legacy_control(character: char, modifiers: Modifiers) -> Option<Delivery> {
    let unshifted = modifiers.without(Modifier::Shift);
    let shifted = unshifted != modifiers;
    if shifted && lowercase(character).is_none() {
        // With Shift a symbol key types another symbol: `shifted_symbol`'s case.
        return None;
    }
    let sent = control_byte(character, unshifted).or_else(|| {
        shifted.then_some(KeyChord {
            key: Key::Char(character),
            modifiers: unshifted,
        })
    })?;
    Some(Delivery::LegacyAs(sent))
}

/// The key crossterm reads Ctrl and `character`'s C0 byte as, when that is
/// not Ctrl and the character: Tab (Ctrl+I), Enter (Ctrl+M), Esc (Ctrl+[,
/// Ctrl+3), Backspace (Ctrl+8), the bare digit (Ctrl+1/9/0), NUL as
/// Ctrl+Space (Ctrl+2, Ctrl+@) and 0x1C to 0x1F as Ctrl+4 to Ctrl+7
/// (Ctrl+\, which FFM decodes back, Ctrl+], Ctrl+^, Ctrl+_).
fn control_byte(character: char, modifiers: Modifiers) -> Option<KeyChord> {
    let without_ctrl = modifiers.without(Modifier::Ctrl);
    let (key, modifiers) = match character {
        'i' => (Key::Named(NamedKey::Tab), without_ctrl),
        'm' => (Key::Named(NamedKey::Enter), without_ctrl),
        '[' | '3' => (Key::Named(NamedKey::Esc), without_ctrl),
        '8' => (Key::Named(NamedKey::Backspace), without_ctrl),
        '1' | '9' | '0' => (Key::Char(character), without_ctrl),
        '2' | '@' => (Key::Named(NamedKey::Space), modifiers),
        '4' => (Key::Char('\\'), modifiers),
        ']' => (Key::Char('5'), modifiers),
        '^' => (Key::Char('6'), modifiers),
        '_' => (Key::Char('7'), modifiers),
        _ => return None,
    };
    Some(KeyChord { key, modifiers })
}

/// The chords a terminal, the desktop or macOS commonly take first.
fn taken_first(chord: KeyChord) -> Option<Delivery> {
    const CTRL_ALT: Modifiers = Modifiers::CTRL.with(Modifier::Alt);
    let modifiers = chord.modifiers;
    let taker = match chord.key {
        Key::F(key) if modifiers.is_empty() && matches!(key.number(), 10 | 11) => Taker::Terminal,
        Key::F(key) if modifiers == Modifiers::ALT && key.number() == 4 => Taker::Desktop,
        Key::Named(NamedKey::Enter) if modifiers == Modifiers::ALT => Taker::Terminal,
        Key::Named(NamedKey::Space) if modifiers == Modifiers::ALT => Taker::Desktop,
        Key::Named(NamedKey::Space) if modifiers == Modifiers::CTRL => Taker::Desktop,
        Key::Named(NamedKey::Del) if modifiers == CTRL_ALT => Taker::Desktop,
        Key::Named(NamedKey::PgUp | NamedKey::PgDn) if modifiers == Modifiers::SHIFT => {
            Taker::Terminal
        }
        Key::Named(NamedKey::Up | NamedKey::Down | NamedKey::Left | NamedKey::Right)
            if modifiers == Modifiers::CTRL =>
        {
            Taker::MacOs
        }
        Key::Char(_)
            if modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Ctrl) =>
        {
            Taker::MacOs
        }
        _ => return None,
    };
    Some(Delivery::TakenFirst(taker))
}

/// Ctrl+Alt with anything but a letter: `AltGr`'s character on Windows.
fn alt_gr(chord: KeyChord) -> Option<Delivery> {
    let ctrl_alt =
        chord.modifiers.contains(Modifier::Ctrl) && chord.modifiers.contains(Modifier::Alt);
    match chord.key {
        Key::Char(character) if ctrl_alt && lowercase(character).is_none() => Some(Delivery::AltGr),
        _ => None,
    }
}

/// The symbols Shift produces on the US layout; another layout's differ,
/// which is why the report only warns.
const US_SHIFTED: &str = "~!@#$%^&*()_+{}|:\"<>?";

/// Ctrl with a symbol Shift produces — written out (`Ctrl+Shift+=`) or as
/// the symbol itself (`Ctrl++`).
fn shifted_symbol(chord: KeyChord) -> Option<Delivery> {
    let Key::Char(character) = chord.key else {
        return None;
    };
    if !chord.modifiers.contains(Modifier::Ctrl) || lowercase(character).is_some() {
        return None;
    }
    let shifted = chord.modifiers.contains(Modifier::Shift) || US_SHIFTED.contains(character);
    shifted.then_some(Delivery::ShiftedSymbol)
}
