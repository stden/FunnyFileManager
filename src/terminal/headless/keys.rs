//! Purpose: Encode remote key events for a private terminal's input stream.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io;

pub(super) fn encode(key: KeyEvent) -> io::Result<String> {
    if key.kind == KeyEventKind::Release {
        return Ok(String::new());
    }
    let input = match key.code {
        KeyCode::Char(character) => character_input(character, key.modifiers)?,
        KeyCode::Enter => "\r".to_owned(),
        KeyCode::Esc => "\u{1b}".to_owned(),
        KeyCode::Backspace => "\u{7f}".to_owned(),
        KeyCode::Tab => "\t".to_owned(),
        KeyCode::BackTab => "\u{1b}[Z".to_owned(),
        KeyCode::Null => "\0".to_owned(),
        code => return special(code, key.modifiers),
    };
    if key.modifiers.contains(KeyModifiers::ALT) {
        Ok(format!("\u{1b}{input}"))
    } else {
        Ok(input)
    }
}

fn character_input(character: char, modifiers: KeyModifiers) -> io::Result<String> {
    if !modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(character.to_string());
    }
    let character = character.to_ascii_uppercase();
    let byte = match character {
        'с' | 'С' => 3,
        ' ' | '@' => 0,
        'A'..='Z' | '['..='_' => {
            u8::try_from(u32::from(character)).map_err(io::Error::other)? & 0x1f
        }
        '?' => 0x7f,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported control key",
            ))
        }
    };
    Ok(char::from(byte).to_string())
}

fn special(code: KeyCode, modifiers: KeyModifiers) -> io::Result<String> {
    let modifier = 1
        + u8::from(modifiers.contains(KeyModifiers::SHIFT))
        + 2 * u8::from(modifiers.contains(KeyModifiers::ALT))
        + 4 * u8::from(modifiers.contains(KeyModifiers::CONTROL));
    let sequence = match code {
        KeyCode::Up => ('A', None),
        KeyCode::Down => ('B', None),
        KeyCode::Right => ('C', None),
        KeyCode::Left => ('D', None),
        KeyCode::Home => ('H', None),
        KeyCode::End => ('F', None),
        KeyCode::Insert => ('~', Some(2)),
        KeyCode::Delete => ('~', Some(3)),
        KeyCode::PageUp => ('~', Some(5)),
        KeyCode::PageDown => ('~', Some(6)),
        KeyCode::F(number) => return function(number, modifier),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported terminal key",
            ))
        }
    };
    Ok(sequence_input(sequence, modifier))
}

fn sequence_input((suffix, number): (char, Option<u8>), modifier: u8) -> String {
    match (number, modifier) {
        (None, 1) => format!("\u{1b}[{suffix}"),
        (Some(number), 1) => format!("\u{1b}[{number}{suffix}"),
        (number, modifier) => format!("\u{1b}[{};{modifier}{suffix}", number.unwrap_or(1)),
    }
}

fn function(number: u8, modifier: u8) -> io::Result<String> {
    if (1..=4).contains(&number) {
        let suffix = char::from(b'P' + number - 1);
        return Ok(if modifier == 1 {
            format!("\u{1b}O{suffix}")
        } else {
            sequence_input((suffix, None), modifier)
        });
    }
    let number = match number {
        5 => 15,
        6 => 17,
        7 => 18,
        8 => 19,
        9 => 20,
        10 => 21,
        11 => 23,
        12 => 24,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported function key",
            ))
        }
    };
    Ok(sequence_input(('~', Some(number)), modifier))
}

#[cfg(test)]
mod tests;
