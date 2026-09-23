//! Purpose: Function, attribute and facade queries over parsed sources.

use super::SourceFile;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Block, Expr, ExprLit, ImplItemFn, Item, ItemFn, Lit, LitStr, Macro, Meta, MetaList,
    Signature, Visibility,
};

/// How a `cfg` selecting the `test-helpers` feature prints as tokens.
const TEST_HELPERS: &str = "feature = \"test-helpers\"";

/// A function definition: free, associated, or inside an inline module.
pub struct Function {
    pub name: String,
    /// Declared `pub` or `pub(crate)`.
    pub exported: bool,
    /// From the `fn` keyword to the closing brace.
    pub lines: std::ops::RangeInclusive<usize>,
    /// An attribute or macro on or inside it names `feature = "test-helpers"`.
    pub selects_test_helpers: bool,
}

impl Function {
    fn new(visibility: &Visibility, signature: &Signature, body: &Block, mentions: bool) -> Self {
        let exported = match visibility {
            Visibility::Public(_) => true,
            Visibility::Restricted(restricted) => restricted.path.is_ident("crate"),
            Visibility::Inherited => false,
        };
        Self {
            name: signature.ident.to_string(),
            exported,
            lines: signature.fn_token.span.start().line..=body.brace_token.span.close().end().line,
            selects_test_helpers: mentions,
        }
    }
}

/// Every function outside `#[cfg(test)]` items, in source order.
pub fn functions(file: &SourceFile) -> Vec<Function> {
    let mut collector = Functions::default();
    collector.visit_file(&file.syntax());
    collector.found
}

#[derive(Default)]
struct Functions {
    found: Vec<Function>,
}

impl<'ast> Visit<'ast> for Functions {
    fn visit_item(&mut self, item: &'ast Item) {
        if !item_attributes(item).iter().any(is_cfg_test) {
            visit::visit_item(self, item);
        }
    }

    fn visit_item_fn(&mut self, function: &'ast ItemFn) {
        let mut mentions = TestHelpersMention::default();
        mentions.visit_item_fn(function);
        let found = Function::new(&function.vis, &function.sig, &function.block, mentions.0);
        self.found.push(found);
        visit::visit_item_fn(self, function);
    }

    fn visit_impl_item_fn(&mut self, function: &'ast ImplItemFn) {
        let mut mentions = TestHelpersMention::default();
        mentions.visit_impl_item_fn(function);
        let found = Function::new(&function.vis, &function.sig, &function.block, mentions.0);
        self.found.push(found);
        visit::visit_impl_item_fn(self, function);
    }
}

/// Finds `feature = "test-helpers"` in `cfg`, `cfg_attr` and `cfg!` input.
#[derive(Default)]
struct TestHelpersMention(bool);

impl<'ast> Visit<'ast> for TestHelpersMention {
    fn visit_meta_list(&mut self, list: &'ast MetaList) {
        self.0 |= list.tokens.to_string().contains(TEST_HELPERS);
        visit::visit_meta_list(self, list);
    }

    fn visit_macro(&mut self, invocation: &'ast Macro) {
        self.0 |= invocation.tokens.to_string().contains(TEST_HELPERS);
    }
}

/// Lines of `#[test]`-style attributes (`test`, `tokio::test`, ...), test code included.
pub fn test_attribute_lines(file: &SourceFile) -> Vec<usize> {
    let mut collector = TestAttributes::default();
    collector.visit_file(&file.syntax());
    collector.0
}

#[derive(Default)]
struct TestAttributes(Vec<usize>);

impl<'ast> Visit<'ast> for TestAttributes {
    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
        {
            self.0.push(attribute.span().start().line);
        }
    }
}

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
struct IncludedFiles(Vec<(usize, String)>);

impl<'ast> Visit<'ast> for IncludedFiles {
    fn visit_macro(&mut self, invocation: &'ast Macro) {
        let includes = ["include", "include_str", "include_bytes"]
            .iter()
            .any(|name| invocation.path.is_ident(name));
        if let (true, Ok(path)) = (includes, invocation.parse_body::<LitStr>()) {
            self.0.push((path.span().start().line, path.value()));
        }
    }

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
}

pub fn is_cfg_test(attribute: &Attribute) -> bool {
    attribute.path().is_ident("cfg")
        && matches!(&attribute.meta, syn::Meta::List(list) if list.tokens.to_string() == "test")
}

pub fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(item) => &item.attrs,
        Item::Enum(item) => &item.attrs,
        Item::ExternCrate(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Macro(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Static(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        Item::Use(item) => &item.attrs,
        _ => &[],
    }
}
