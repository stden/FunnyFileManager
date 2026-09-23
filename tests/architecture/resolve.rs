//! Purpose: Resolve every path a Rust source names to where it points.

use super::items::{is_cfg_test, item_attributes};
use super::tokens::{TokenPaths, ABSOLUTE};
use super::SourceFile;
use proc_macro2::Span;
use std::collections::HashMap;
use syn::visit::{self, Visit};
use syn::{Attribute, Block, Item, ItemMod, ItemUse, Macro, Stmt, UseTree, Visibility};

/// Imports that point at other imports stop resolving after this many hops.
const MAX_ALIAS_HOPS: usize = 8;

/// A path a source file names, resolved: `crate::ops::refresh`, `std::fs::read`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub path: String,
    pub line: usize,
    pub column: usize,
}

/// Every path `file` names outside `#[cfg(test)]` items, in source order.
pub fn dependencies(file: &SourceFile) -> Vec<Dependency> {
    let syntax = file.syntax();
    let mut collector = Collector::new(file);
    collector.visit_file(&syntax);
    collector
        .found
        .sort_by_key(|found| (found.line, found.column));
    collector.found
}

/// Imports at the top of a crate root, as `(crate::alias, target)`: every
/// module may name them through `crate::`, public or not.
/// `pub use application::run_app;` gives `("crate::run_app", "crate::application::run_app")`.
pub fn root_imports(root: &SourceFile) -> Vec<(String, String)> {
    let syntax = root.syntax();
    let mut collector = Collector::new(root);
    collector.enter(&syntax.items, true);
    let imports = syntax.items.iter().filter_map(|item| match item {
        Item::Use(declaration) if !is_test_only(item) => Some(declaration),
        _ => None,
    });
    imports
        .flat_map(use_leaves)
        .filter_map(|leaf| {
            let alias = leaf.alias?;
            let target = collector.resolve(&leaf.path, MAX_ALIAS_HOPS).join("::");
            Some((format!("crate::{alias}"), target))
        })
        .collect()
}

/// Names a module or block brings into scope, mapped to the path they stand for.
struct Scope {
    names: HashMap<String, Vec<String>>,
    /// Modules do not see their parent's imports; blocks do.
    is_module: bool,
}

struct Collector {
    module: Vec<String>,
    scopes: Vec<Scope>,
    found: Vec<Dependency>,
}

impl Collector {
    fn new(file: &SourceFile) -> Self {
        Self {
            module: file.module.clone(),
            scopes: Vec::new(),
            found: Vec::new(),
        }
    }

    fn enter<'i>(&mut self, items: impl IntoIterator<Item = &'i Item>, is_module: bool) {
        let mut names = HashMap::new();
        let mut children = Vec::new();
        for item in items.into_iter().filter(|item| !is_test_only(item)) {
            match item {
                Item::Use(declaration) => names.extend(
                    use_leaves(declaration)
                        .into_iter()
                        .filter_map(|leaf| Some((leaf.alias?, leaf.path))),
                ),
                Item::Mod(child) => children.push(child.ident.to_string()),
                _ => {}
            }
        }
        // A child module wins over an import of the same name as a path prefix.
        for child in children {
            let path = crate_path(&self.module, std::slice::from_ref(&child));
            names.insert(child, path);
        }
        self.scopes.push(Scope { names, is_module });
    }

    fn lookup(&self, name: &str) -> Option<&Vec<String>> {
        for scope in self.scopes.iter().rev() {
            if let Some(path) = scope.names.get(name) {
                return Some(path);
            }
            if scope.is_module {
                break;
            }
        }
        None
    }

    fn resolve(&self, segments: &[String], hops: usize) -> Vec<String> {
        let Some((first, rest)) = segments.split_first() else {
            return Vec::new();
        };
        match first.as_str() {
            ABSOLUTE => rest.to_vec(),
            "crate" | "$crate" => crate_path(&[], rest),
            "self" => crate_path(&self.module, rest),
            "super" => {
                let depth = segments
                    .iter()
                    .take_while(|segment| *segment == "super")
                    .count();
                let kept = self.module.len().saturating_sub(depth);
                crate_path(&self.module[..kept], &segments[depth..])
            }
            name => match self.lookup(name) {
                Some(target) if hops > 0 => {
                    let mut path = self.resolve(target, hops - 1);
                    path.extend_from_slice(rest);
                    path
                }
                _ => segments.to_vec(),
            },
        }
    }

    fn record(&mut self, segments: &[String], span: Span) {
        let path = self.resolve(segments, MAX_ALIAS_HOPS);
        if path.is_empty() {
            return;
        }
        let start = span.start();
        self.found.push(Dependency {
            path: path.join("::"),
            line: start.line,
            column: start.column,
        });
    }
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_file(&mut self, file: &'ast syn::File) {
        self.enter(&file.items, true);
        visit::visit_file(self, file);
        self.scopes.pop();
    }

    fn visit_item(&mut self, item: &'ast Item) {
        if !is_test_only(item) {
            visit::visit_item(self, item);
        }
    }

    fn visit_item_mod(&mut self, module: &'ast ItemMod) {
        let Some((_, items)) = &module.content else {
            return;
        };
        self.module.push(module.ident.to_string());
        self.enter(items, true);
        for item in items {
            self.visit_item(item);
        }
        self.scopes.pop();
        self.module.pop();
    }

    fn visit_block(&mut self, block: &'ast Block) {
        let items = block.stmts.iter().filter_map(|statement| match statement {
            Stmt::Item(item) => Some(item),
            _ => None,
        });
        self.enter(items, false);
        visit::visit_block(self, block);
        self.scopes.pop();
    }

    fn visit_item_use(&mut self, declaration: &'ast ItemUse) {
        for leaf in use_leaves(declaration) {
            self.record(&leaf.path, leaf.span);
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let mut segments = Vec::new();
        if path.leading_colon.is_some() {
            segments.push(ABSOLUTE.to_owned());
        }
        segments.extend(
            path.segments
                .iter()
                .map(|segment| segment.ident.to_string()),
        );
        let span = path
            .segments
            .first()
            .map_or_else(Span::call_site, |segment| segment.ident.span());
        self.record(&segments, span);
        visit::visit_path(self, path);
    }

    /// `pub(crate)` and `pub(super)` restrict visibility; they name no dependency.
    fn visit_visibility(&mut self, _: &'ast Visibility) {}

    /// Attributes (`cfg`, `doc`, `derive`, ...) configure code; they are not scanned.
    fn visit_attribute(&mut self, _: &'ast Attribute) {}

    fn visit_macro(&mut self, invocation: &'ast Macro) {
        self.visit_path(&invocation.path);
        let mut scanner = TokenPaths::default();
        scanner.scan(invocation.tokens.clone());
        for (segments, span) in scanner.paths {
            self.record(&segments, span);
        }
    }
}

/// `crate` followed by `module` and `rest`.
fn crate_path(module: &[String], rest: &[String]) -> Vec<String> {
    let mut path = vec!["crate".to_owned()];
    path.extend_from_slice(module);
    path.extend_from_slice(rest);
    path
}

/// One name a `use` declaration imports, with its group and rename expanded.
struct UseLeaf {
    path: Vec<String>,
    /// The name it is bound to; `None` for globs and `as _`.
    alias: Option<String>,
    span: Span,
}

fn use_leaves(declaration: &ItemUse) -> Vec<UseLeaf> {
    let mut prefix = Vec::new();
    if declaration.leading_colon.is_some() {
        prefix.push(ABSOLUTE.to_owned());
    }
    let mut leaves = Vec::new();
    expand_use(&declaration.tree, &mut prefix, &mut leaves);
    leaves
}

fn expand_use(tree: &UseTree, prefix: &mut Vec<String>, leaves: &mut Vec<UseLeaf>) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            expand_use(&path.tree, prefix, leaves);
            prefix.pop();
        }
        UseTree::Name(name) => leaves.push(use_leaf(prefix, &name.ident, None)),
        UseTree::Rename(rename) => {
            leaves.push(use_leaf(prefix, &rename.ident, Some(&rename.rename)));
        }
        UseTree::Glob(glob) => leaves.push(UseLeaf {
            path: [prefix.as_slice(), &["*".to_owned()]].concat(),
            alias: None,
            span: glob.star_token.spans[0],
        }),
        UseTree::Group(group) => {
            for tree in &group.items {
                expand_use(tree, prefix, leaves);
            }
        }
    }
}

/// `use a::b::{self}` imports `a::b`; `use a::b as _` binds no name.
fn use_leaf(prefix: &[String], name: &syn::Ident, rename: Option<&syn::Ident>) -> UseLeaf {
    let mut path = prefix.to_vec();
    if name != "self" {
        path.push(name.to_string());
    }
    let alias = match rename {
        Some(rename) if rename == "_" => None,
        Some(rename) => Some(rename.to_string()),
        None => path.last().cloned(),
    };
    UseLeaf {
        path,
        alias,
        span: name.span(),
    }
}

/// Items compiled only for unit tests: exactly `#[cfg(test)]`.
fn is_test_only(item: &Item) -> bool {
    item_attributes(item).iter().any(is_cfg_test)
}
