//! Purpose: Resolve every path a Rust source names to where it points.

use super::items::{is_cfg_test, item_attributes};
use super::tokens::{TokenPaths, ABSOLUTE};
use super::SourceFile;
use proc_macro2::Span;
use std::collections::HashMap;
use syn::visit::{self, Visit};
use syn::{Attribute, Block, Item, ItemMod, ItemUse, Macro, Stmt, UseTree, Visibility};

#[path = "resolve/collector.rs"]
mod collector;
use collector::Collector;

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
