//! Purpose: Splitting an apply-command template into text and Far
//! metacharacters, matched longest first; a list always ends with `!`.

use super::{ListOptions, NameForm, NamePart, PanelReference, PathKind, QuoteMode, Symbol, Token};

const SYMBOLS: &[(&str, Symbol)] = &[
    ("!!", Symbol::Bang),
    ("!.!", Symbol::Name(NameForm::Long, NamePart::Full)),
    ("!-!", Symbol::Name(NameForm::Short, NamePart::Full)),
    ("!+!", Symbol::PreserveName),
    ("!`~", Symbol::Name(NameForm::Short, NamePart::Extension)),
    ("!`", Symbol::Name(NameForm::Long, NamePart::Extension)),
    ("!~", Symbol::Name(NameForm::Short, NamePart::Stem)),
    ("!=\\", Symbol::Folder(NameForm::Long, PathKind::Resolved)),
    ("!=/", Symbol::Folder(NameForm::Short, PathKind::Resolved)),
    ("!\\", Symbol::Folder(NameForm::Long, PathKind::Logical)),
    ("!/", Symbol::Folder(NameForm::Short, PathKind::Logical)),
    ("!:", Symbol::Drive),
    ("!?!", Symbol::Description),
    ("!", Symbol::Name(NameForm::Long, NamePart::Stem)),
];

pub struct Tokens<'a> {
    rest: &'a str,
}

impl<'a> Tokens<'a> {
    pub(crate) const fn new(template: &'a str) -> Self {
        Self { rest: template }
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let at = self.rest.find('!').unwrap_or(self.rest.len());
        let (length, token) = if at == 0 {
            parse_symbol(self.rest)
        } else {
            (at, Token::Text(self.rest.get(..at)?))
        };
        // Every length above ends on a char boundary inside `rest`.
        self.rest = self.rest.get(length..).unwrap_or_default();
        Some(token)
    }
}

fn parse_symbol(text: &str) -> (usize, Token<'_>) {
    if let Some(panel) = panel_reference(text) {
        return (2, Token::Panel(panel));
    }
    if let Some(list) = parse_list(text) {
        return list;
    }
    if let Some(names) = parse_names(text) {
        return names;
    }
    let (spelling, symbol) = SYMBOLS
        .iter()
        .find(|(spelling, _)| text.starts_with(spelling))
        .copied()
        .unwrap_or(("!", Symbol::Bang));
    (spelling.len(), Token::Symbol(symbol))
}

fn panel_reference(text: &str) -> Option<PanelReference> {
    [
        ("!#", PanelReference::Passive),
        ("!^", PanelReference::Active),
        ("![", PanelReference::Left),
        ("!]", PanelReference::Right),
    ]
    .into_iter()
    .find_map(|(prefix, panel)| text.starts_with(prefix).then_some(panel))
}

fn parse_list(text: &str) -> Option<(usize, Token<'_>)> {
    let (form, tail) = if let Some(tail) = text.strip_prefix("!@") {
        (NameForm::Long, tail)
    } else {
        (NameForm::Short, text.strip_prefix("!$")?)
    };
    let (options, rest) = tail.split_once('!')?;
    Some((
        text.len() - rest.len(),
        Token::List(ListOptions::parse(form, options)?),
    ))
}

fn parse_names(text: &str) -> Option<(usize, Token<'_>)> {
    let tail = text.strip_prefix("!&")?;
    let (form, tail) = tail
        .strip_prefix('~')
        .map_or((NameForm::Long, tail), |tail| (NameForm::Short, tail));
    let quote = if tail.starts_with('q') {
        QuoteMode::Spaces
    } else {
        QuoteMode::Always
    };
    let suffix = usize::from(tail.starts_with(['q', 'Q']));
    let length = text.len() - tail.len() + suffix;
    Some((length, Token::Symbol(Symbol::Names(form, quote))))
}
