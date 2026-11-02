//! Resolved panel data supplied to the pure command expander.

use std::path::{Path, MAIN_SEPARATOR};

use super::symbols::{NameForm, NamePart, PanelReference, PathKind, Symbol};
use crate::models::PanelSide;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Names {
    pub long: String,
    pub short: String,
}

impl Names {
    pub(crate) fn same(name: &str) -> Self {
        Self {
            long: name.to_owned(),
            short: name.to_owned(),
        }
    }

    pub(crate) fn get(&self, form: NameForm) -> &str {
        match form {
            NameForm::Long => &self.long,
            NameForm::Short => &self.short,
        }
    }

    fn part(&self, form: NameForm, part: NamePart) -> &str {
        let name = self.get(form);
        let (stem, extension) = name.rsplit_once('.').unwrap_or((name, ""));
        match part {
            NamePart::Full => name,
            NamePart::Stem => stem,
            NamePart::Extension => extension,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Paths {
    pub logical: Names,
    pub resolved: Names,
    pub drive: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelContext {
    pub paths: Paths,
    pub entry: Names,
    pub selected: Vec<Names>,
    pub description: String,
}

impl PanelContext {
    pub(super) fn expand(&self, symbol: Symbol) -> String {
        match symbol {
            Symbol::Bang => "!".to_owned(),
            Symbol::Name(form, part) => self.entry.part(form, part).to_owned(),
            Symbol::PreserveName => self.entry.short.clone(),
            Symbol::Folder(form, kind) => self.folder(form, kind),
            Symbol::Drive => self.paths.drive.clone(),
            Symbol::Description => self.description.clone(),
            Symbol::Names(form, quote) => self
                .selected
                .iter()
                .map(|name| quote.apply(name.get(form)))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    fn folder(&self, form: NameForm, kind: PathKind) -> String {
        let names = match kind {
            PathKind::Logical => &self.paths.logical,
            PathKind::Resolved => &self.paths.resolved,
        };
        let mut path = names.get(form).to_owned();
        if !path.ends_with(['/', '\\']) {
            path.push(MAIN_SEPARATOR);
        }
        path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub left: PanelContext,
    pub right: PanelContext,
    pub active: PanelSide,
}

impl Context {
    pub(super) fn single(entry: &Path, selected: &[String]) -> Self {
        let folder = Names::same(&entry.parent().unwrap_or(entry).display().to_string());
        let panel = PanelContext {
            paths: Paths {
                logical: folder.clone(),
                resolved: folder,
                drive: String::new(),
            },
            entry: Names::same(&entry.file_name().unwrap_or_default().to_string_lossy()),
            selected: selected.iter().map(|name| Names::same(name)).collect(),
            description: String::new(),
        };
        Self {
            left: panel.clone(),
            right: panel,
            active: PanelSide::Left,
        }
    }

    pub(crate) const fn panel(&self, side: PanelSide) -> &PanelContext {
        match side {
            PanelSide::Left => &self.left,
            PanelSide::Right => &self.right,
        }
    }

    pub(crate) const fn panel_mut(&mut self, side: PanelSide) -> &mut PanelContext {
        match side {
            PanelSide::Left => &mut self.left,
            PanelSide::Right => &mut self.right,
        }
    }

    pub(super) const fn side(&self, reference: PanelReference) -> PanelSide {
        match reference {
            PanelReference::Active => self.active,
            PanelReference::Passive => match self.active {
                PanelSide::Left => PanelSide::Right,
                PanelSide::Right => PanelSide::Left,
            },
            PanelReference::Left => PanelSide::Left,
            PanelReference::Right => PanelSide::Right,
        }
    }
}
