//! Purpose: Language tables for source and data files, and the filename lookup that picks one.

use std::path::Path;

use super::formats::{CONFIG, DOTENV, JSON, MARKDOWN, MARKUP};
use super::shells::{BATCH, POWERSHELL, SHELL};
use super::{LineMode, Syntax};

mod kotlin;
mod typst;
use kotlin::KOTLIN;
use typst::TYPST;

const RUST: Syntax = Syntax {
    mode: LineMode::Tokens,
    line_comments: &["//"],
    block_comments: &[("/*", "*/")],
    quotes: &['"', '\''],
    keywords: &[
        "Self", "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
        "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
        "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait",
        "true", "type", "unsafe", "use", "where", "while", "yield",
    ],
    types: &[
        "Box", "Option", "Result", "String", "Vec", "bool", "char", "f32", "f64", "i128", "i16",
        "i32", "i64", "i8", "isize", "str", "u128", "u16", "u32", "u64", "u8", "usize",
    ],
    meta_prefix: None,
    section_headers: false,
    variable_prefix: None,
    word_hyphens: false,
    switch_prefix: None,
};

const PYTHON: Syntax = Syntax {
    mode: LineMode::Tokens,
    line_comments: &["#"],
    block_comments: &[("\"\"\"", "\"\"\""), ("'''", "'''")],
    quotes: &['"', '\''],
    keywords: &[
        "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
        "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
        "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return",
        "try", "while", "with", "yield",
    ],
    types: &[
        "bool", "bytes", "dict", "float", "int", "list", "set", "str", "tuple",
    ],
    meta_prefix: Some('@'),
    section_headers: false,
    variable_prefix: None,
    word_hyphens: false,
    switch_prefix: None,
};

const C_LIKE: Syntax = Syntax {
    mode: LineMode::Tokens,
    line_comments: &["//"],
    block_comments: &[("/*", "*/")],
    quotes: &['"', '\''],
    keywords: &[
        "auto",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "extern",
        "false",
        "final",
        "finally",
        "for",
        "function",
        "goto",
        "if",
        "implements",
        "import",
        "inline",
        "instanceof",
        "interface",
        "let",
        "namespace",
        "new",
        "null",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "sizeof",
        "static",
        "struct",
        "switch",
        "template",
        "this",
        "throw",
        "true",
        "try",
        "typedef",
        "typeof",
        "union",
        "using",
        "var",
        "virtual",
        "void",
        "while",
        "yield",
    ],
    types: &[
        "any", "bool", "boolean", "char", "double", "float", "int", "long", "number", "object",
        "short", "signed", "size_t", "string", "unsigned",
    ],
    meta_prefix: Some('#'),
    section_headers: false,
    variable_prefix: None,
    word_hyphens: false,
    switch_prefix: None,
};

/// The language to highlight `path` as, or `None` for plain text.
///
/// Recognized filenames and extensions. Content sniffing (shebangs, XML prologues) would guess
/// right more often, but it would also guess *wrong* silently, and a file
/// coloured as the wrong language reads worse than one not coloured at all.
#[must_use]
pub fn syntax_for(path: &Path) -> Option<Syntax> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if let Some(syntax) = syntax_for_name(name) {
        return Some(syntax);
    }
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    Some(match extension.as_deref()? {
        "rs" => RUST,
        "py" | "pyi" => PYTHON,
        "kt" | "kts" => KOTLIN,
        "c" | "h" | "cc" | "cpp" | "hpp" | "cs" | "go" | "java" | "js" | "jsx" | "ts" | "tsx"
        | "swift" | "php" => C_LIKE,
        "sh" | "bash" | "zsh" | "fish" => SHELL,
        "cmd" | "bat" => BATCH,
        "ps1" | "psm1" | "psd1" => POWERSHELL,
        "toml" | "ini" | "cfg" | "conf" | "yaml" | "yml" | "properties" => CONFIG,
        "env" => DOTENV,
        "json" => JSON,
        "html" | "htm" | "xml" | "svg" | "xhtml" | "xsd" | "xsl" | "xslt" | "wsdl" | "pom"
        | "csproj" | "props" | "targets" | "plist" | "jmx" => MARKUP,
        "md" | "markdown" => MARKDOWN,
        "typ" => TYPST,
        _ => return None,
    })
}

fn syntax_for_name(name: &str) -> Option<Syntax> {
    if name == ".env" || name.starts_with(".env.") {
        return Some(DOTENV);
    }
    match name {
        "Makefile" | "Dockerfile" | ".gitignore" | ".gitmessage" => Some(SHELL),
        // `.git/config` is the common one, and it has no extension to look at.
        "Cargo.lock" | "config" | ".gitconfig" | ".gitmodules" | ".editorconfig" | ".npmrc"
        | "hosts" | "fstab" => Some(CONFIG),
        _ => None,
    }
}
