//! Far metacharacters, parsed longest first. A list always ends with `!`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameForm {
    Long,
    Short,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamePart {
    Full,
    Stem,
    Extension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelReference {
    Active,
    Passive,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    Logical,
    Resolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteMode {
    Always,
    Spaces,
    Never,
}

impl QuoteMode {
    pub(crate) fn apply(self, text: &str) -> String {
        if self == Self::Always || self == Self::Spaces && text.contains([' ', '\t']) {
            format!("\"{text}\"")
        } else {
            text.to_owned()
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ListEncoding {
    #[default]
    Oem,
    Ansi,
    Utf8,
    Utf16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListOptions {
    pub form: NameForm,
    pub quote: QuoteMode,
    pub full_paths: bool,
    pub forward_slashes: bool,
    pub encoding: ListEncoding,
}

impl ListOptions {
    fn parse(form: NameForm, modifiers: &str) -> Self {
        let mut options = Self {
            form,
            quote: QuoteMode::Never,
            full_paths: false,
            forward_slashes: false,
            encoding: ListEncoding::default(),
        };
        for modifier in modifiers.chars() {
            match modifier {
                'Q' => options.quote = QuoteMode::Always,
                'F' => options.full_paths = true,
                'S' => options.forward_slashes = true,
                'A' => options.encoding = ListEncoding::Ansi,
                'U' => options.encoding = ListEncoding::Utf8,
                'W' => options.encoding = ListEncoding::Utf16,
                _ => {}
            }
        }
        options
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    Bang,
    Name(NameForm, NamePart),
    PreserveName,
    Names(NameForm, QuoteMode),
    Folder(NameForm, PathKind),
    Drive,
    Description,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token<'a> {
    Text(&'a str),
    Panel(PanelReference),
    Symbol(Symbol),
    List(ListOptions),
}

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
            (at, Token::Text(&self.rest[..at]))
        };
        self.rest = &self.rest[length..];
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
    let form = if text.starts_with("!@") {
        NameForm::Long
    } else if text.starts_with("!$") {
        NameForm::Short
    } else {
        return None;
    };
    let end = text[2..].find('!')? + 2;
    Some((
        end + 1,
        Token::List(ListOptions::parse(form, &text[2..end])),
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
