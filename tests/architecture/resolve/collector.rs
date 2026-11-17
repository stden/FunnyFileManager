//! Purpose: Walking one file's syntax tree, collecting every path it names with its scope.

use super::*;

/// Names a module or block brings into scope, mapped to the path they stand for.
pub struct Scope {
    names: HashMap<String, Vec<String>>,
    /// Modules do not see their parent's imports; blocks do.
    is_module: bool,
}

pub struct Collector {
    module: Vec<String>,
    scopes: Vec<Scope>,
    pub found: Vec<Dependency>,
}

impl Collector {
    pub fn new(file: &SourceFile) -> Self {
        Self {
            module: file.module.clone(),
            scopes: Vec::new(),
            found: Vec::new(),
        }
    }

    pub fn enter<'i>(&mut self, items: impl IntoIterator<Item = &'i Item>, is_module: bool) {
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

    pub fn resolve(&self, segments: &[String], hops: usize) -> Vec<String> {
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
                #[expect(
                    clippy::indexing_slicing,
                    reason = "`depth` counts a prefix of `segments` and `kept` never exceeds the module depth"
                )]
                let (parent, below) = (&self.module[..kept], &segments[depth..]);
                crate_path(parent, below)
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
    /// Attributes (`cfg`, `doc`, `derive`, ...) configure code; they are not scanned.
    fn visit_attribute(&mut self, _: &'ast Attribute) {}

    fn visit_block(&mut self, block: &'ast Block) {
        let items = block.stmts.iter().filter_map(|statement| match statement {
            Stmt::Item(item) => Some(item),
            _ => None,
        });
        self.enter(items, false);
        visit::visit_block(self, block);
        self.scopes.pop();
    }

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

    fn visit_item_use(&mut self, declaration: &'ast ItemUse) {
        for leaf in use_leaves(declaration) {
            self.record(&leaf.path, leaf.span);
        }
    }

    fn visit_macro(&mut self, invocation: &'ast Macro) {
        self.visit_path(&invocation.path);
        let mut scanner = TokenPaths::default();
        scanner.scan(invocation.tokens.clone());
        for (segments, span) in scanner.paths {
            self.record(&segments, span);
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
}

/// `crate` followed by `module` and `rest`.
fn crate_path(module: &[String], rest: &[String]) -> Vec<String> {
    let mut path = vec!["crate".to_owned()];
    path.extend_from_slice(module);
    path.extend_from_slice(rest);
    path
}
