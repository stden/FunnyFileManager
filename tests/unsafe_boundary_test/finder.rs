//! Purpose: Finding unsafe code, in one parsed file and across the scanned source trees.

use super::*;

/// One piece of unsafe code, or a permission for it.
#[derive(Debug)]
pub struct Site {
    pub line: usize,
    pub what: String,
}

#[derive(Default)]
pub struct Finder {
    pub sites: Vec<Site>,
}

impl Finder {
    fn record(&mut self, span: Span, what: impl Into<String>) {
        self.sites.push(Site {
            line: span.start().line,
            what: what.into(),
        });
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.record(node.unsafe_token.span, "unsafe block");
        visit::visit_expr_unsafe(self, node);
    }

    fn visit_signature(&mut self, node: &'ast syn::Signature) {
        if let syn::Safety::Unsafe(token) = node.safety {
            self.record(token.span, format!("unsafe fn {}", node.ident));
        }
        visit::visit_signature(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if let Some(token) = node.unsafety {
            self.record(token.span, "unsafe impl");
        }
        visit::visit_item_impl(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if let Some(token) = node.unsafety {
            self.record(token.span, format!("unsafe trait {}", node.ident));
        }
        visit::visit_item_trait(self, node);
    }

    fn visit_item_foreign_mod(&mut self, node: &'ast syn::ItemForeignMod) {
        self.record(node.abi.extern_token.span, "extern block");
        visit::visit_item_foreign_mod(self, node);
    }

    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if permits_unsafe_code(node) {
            self.record(node.span(), "allow(unsafe_code)");
        }
        visit::visit_attribute(self, node);
    }

    /// syn does not parse macro bodies, so an `unsafe` inside one is found by
    /// its tokens.
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let words = idents(node.tokens.clone());
        if words
            .iter()
            .any(|word| word == "unsafe" || word == "unsafe_code")
        {
            self.record(node.span(), "macro that mentions unsafe");
        }
        visit::visit_macro(self, node);
    }
}

/// Every identifier in `tokens`, inside groups too. Literals and comments are
/// not identifiers, so a string saying "unsafe" is not a finding.
fn idents(tokens: TokenStream) -> Vec<String> {
    tokens
        .into_iter()
        .flat_map(|tree| match tree {
            TokenTree::Ident(ident) => vec![ident.to_string()],
            TokenTree::Group(group) => idents(group.stream()),
            TokenTree::Punct(_) | TokenTree::Literal(_) => Vec::new(),
        })
        .collect()
}

/// `allow`, `expect` or `warn` of `unsafe_code`, directly or via `cfg_attr`.
fn permits_unsafe_code(attribute: &syn::Attribute) -> bool {
    let syn::Meta::List(list) = &attribute.meta else {
        return false;
    };
    let mut words = idents(list.tokens.clone());
    words.extend(list.path.get_ident().map(ToString::to_string));
    let names_lint = words.iter().any(|word| word == "unsafe_code");
    names_lint
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "allow" | "expect" | "warn"))
}

/// Every unsafe site in `source`, which is named `path` in a parse failure.
#[expect(
    clippy::panic,
    reason = "a source the scan cannot parse fails the test; unparsable_source_fails_loudly pins it"
)]
pub fn sites_in(path: &str, source: &str) -> Vec<Site> {
    let file = syn::parse_file(source).unwrap_or_else(|error| panic!("parse {path}: {error}"));
    let mut finder = Finder::default();
    finder.visit_file(&file);
    finder.sites
}

#[expect(
    clippy::expect_used,
    reason = "test support: a source tree the scan cannot read fails the test that asked for it"
)]
fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// `(repository-relative path, unsafe sites)` for every file under `roots`.
#[expect(
    clippy::expect_used,
    reason = "test support: a source tree the scan cannot read fails the test that asked for it"
)]
pub fn scan(roots: &[&str]) -> Vec<(String, Vec<Site>)> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for root in roots {
        rust_files(&repository.join(root), &mut files);
    }
    files.sort();
    files
        .iter()
        .map(|file| {
            let relative = file.strip_prefix(repository).expect("below the repository");
            let relative = relative.to_string_lossy().replace('\\', "/");
            let source = fs::read_to_string(file).expect("read Rust source");
            let sites = sites_in(&relative, &source);
            (relative, sites)
        })
        .collect()
}
