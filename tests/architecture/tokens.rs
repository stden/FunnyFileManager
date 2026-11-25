//! Purpose: Find `a::b::c` path sequences in a macro's input tokens.

use proc_macro2::{Spacing, Span, TokenStream, TokenTree};

/// Marks a path written with a leading `::` (an external crate).
pub const ABSOLUTE: &str = "::";

/// Best-effort `a::b::c` sequences in a macro's input tokens.
#[derive(Default)]
pub struct TokenPaths {
    current: Vec<String>,
    start: Option<Span>,
    after_separator: bool,
    pub paths: Vec<(Vec<String>, Span)>,
}

impl TokenPaths {
    pub fn scan(&mut self, tokens: TokenStream) {
        let mut tokens = tokens.into_iter().peekable();
        while let Some(token) = tokens.next() {
            match token {
                TokenTree::Ident(ident) => self.ident(&ident),
                TokenTree::Punct(punct) if is_path_separator(&punct, tokens.peek()) => {
                    tokens.next();
                    self.separator(punct.span());
                }
                TokenTree::Group(group) => {
                    self.flush();
                    self.scan(group.stream());
                }
                TokenTree::Punct(_) | TokenTree::Literal(_) => self.flush(),
            }
        }
        self.flush();
    }

    fn ident(&mut self, ident: &proc_macro2::Ident) {
        if !self.after_separator {
            self.flush();
            self.start = Some(ident.span());
        }
        self.current.push(ident.to_string());
        self.after_separator = false;
    }

    fn separator(&mut self, span: Span) {
        if self.current.is_empty() {
            self.current.push(ABSOLUTE.to_owned());
            self.start = Some(span);
        }
        self.after_separator = true;
    }

    fn flush(&mut self) {
        self.after_separator = false;
        let path = std::mem::take(&mut self.current);
        if let Some(start) = self.start.take() {
            self.paths.push((path, start));
        }
    }
}

fn is_path_separator(punct: &proc_macro2::Punct, next: Option<&TokenTree>) -> bool {
    punct.as_char() == ':'
        && punct.spacing() == Spacing::Joint
        && matches!(next, Some(TokenTree::Punct(next)) if next.as_char() == ':')
}
