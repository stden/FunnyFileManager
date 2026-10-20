//! Far metacharacters, parsed longest first. A list always ends with `!`.

mod tokens;

pub use tokens::Tokens;

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
        let needs_quotes =
            self == Self::Always || (self == Self::Spaces && text.chars().any(shell_metachar));
        if needs_quotes {
            format!("\"{}\"", escape_shell(text))
        } else {
            text.to_owned()
        }
    }

    pub(crate) fn apply_list(self, text: &str) -> String {
        if self == Self::Always {
            format!("\"{}\"", text.replace('"', "\\\""))
        } else {
            text.to_owned()
        }
    }
}

#[cfg(windows)]
const fn shell_metachar(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | ';' | '&' | '|' | '"' | '\'' | '(' | ')' | '<' | '>' | '^' | '\n' | '\r'
    )
}

#[cfg(not(windows))]
const fn shell_metachar(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | ';'
            | '&'
            | '|'
            | '$'
            | '`'
            | '"'
            | '\''
            | '\\'
            | '('
            | ')'
            | '<'
            | '>'
            | '\n'
            | '\r'
    )
}

#[cfg(windows)]
fn escape_shell(text: &str) -> String {
    text.replace('"', "\\\"")
}

#[cfg(not(windows))]
fn escape_shell(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
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
    pub(super) fn parse(form: NameForm, modifiers: &str) -> Option<Self> {
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
                _ => return None,
            }
        }
        Some(options)
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
