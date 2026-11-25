//! Purpose: The crate-root facade: what it may declare, and the files it includes.

use super::*;

/// A facade only declares modules and re-exports: every other item, and any
/// `#[cfg(test)]` code, is a violation.
pub fn facade_violations(file: &SourceFile) -> Vec<String> {
    let mut messages = Vec::new();
    facade_items(file, &file.syntax().items, &mut messages);
    messages
}

fn facade_items(file: &SourceFile, items: &[Item], messages: &mut Vec<String>) {
    for item in items {
        let line = item.span().start().line;
        if item_attributes(item).iter().any(is_cfg_test) {
            messages.push(format!(
                "{}:{line}: facade carries `#[cfg(test)]` code",
                file.relative
            ));
        }
        match item {
            Item::Use(_) => {}
            Item::Mod(module) => {
                if let Some((_, nested)) = &module.content {
                    facade_items(file, nested, messages);
                }
            }
            _ => messages.push(format!(
                "{}:{line}: facade owns an item other than `mod` or `use`",
                file.relative
            )),
        }
    }
}

/// Files compiled into this one: `include!`, `include_str!`, `include_bytes!`
/// arguments and `#[path = "..."]` module paths, with their lines.
pub fn included_files(file: &SourceFile) -> Vec<(usize, String)> {
    let mut collector = IncludedFiles::default();
    collector.visit_file(&file.syntax());
    collector.0
}

#[derive(Default)]
pub struct IncludedFiles(pub Vec<(usize, String)>);

impl<'ast> Visit<'ast> for IncludedFiles {
    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if let Meta::NameValue(pair) = &attribute.meta {
            if let (
                true,
                Expr::Lit(ExprLit {
                    lit: Lit::Str(path),
                    ..
                }),
            ) = (pair.path.is_ident("path"), &pair.value)
            {
                self.0.push((path.span().start().line, path.value()));
            }
        }
    }

    fn visit_macro(&mut self, invocation: &'ast Macro) {
        let includes = ["include", "include_str", "include_bytes"]
            .iter()
            .any(|name| invocation.path.is_ident(name));
        if let (true, Ok(path)) = (includes, invocation.parse_body::<LitStr>()) {
            self.0.push((path.span().start().line, path.value()));
        }
    }
}
