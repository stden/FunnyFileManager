//! Purpose: Identify the keymap currently consuming input and reject calls behind a modal.

use std::io;

use crate::{
    action::viewer::ViewerCommand,
    automation::catalog::Invocation,
    keymap::Context,
    models::{App, Modal, Overlay, PanelKind, Recall, ViewMode},
};

#[expect(
    clippy::redundant_pub_crate,
    reason = "pub(crate) inside a pub(super) ancestor module: the snapshot crate module imports this via crate::application::automation::current_context"
)]
pub(crate) fn current_context(app: &App) -> Option<Context> {
    match &app.overlay {
        Overlay::None => Some(panel_context(app)),
        Overlay::Viewer(_) => Some(Context::Viewer),
        Overlay::Editor(editor) if editor.modal().is_none() => Some(Context::Editor),
        Overlay::CommandLine(line) => Some(line_context(line)),
        Overlay::Tasks(list) => task_context(list.modal()),
        _ => management_context(&app.overlay),
    }
}

fn management_context(overlay: &Overlay) -> Option<Context> {
    match overlay {
        Overlay::Wsl(_) => Some(Context::WslManager),
        Overlay::LspEditor(editor)
            if matches!(
                editor.mode(),
                crate::models::lsp_editor::Mode::Browse | crate::models::lsp_editor::Mode::View(_)
            ) =>
        {
            Some(Context::LanguageServers)
        }
        Overlay::PathEditor(editor) if matches!(editor.mode, crate::path_env::Mode::Browse) => {
            Some(Context::PathEditor)
        }
        #[cfg(feature = "keepass")]
        Overlay::Kdbx(_) => Some(Context::Vault),
        _ => None,
    }
}

fn panel_context(app: &App) -> Context {
    if app.view != ViewMode::Panels {
        return Context::Panels;
    }
    match app.get_active_panel_ref().kind {
        PanelKind::Tree(_) => Context::TreePanel,
        PanelKind::Sessions(_) | PanelKind::Remote(_) => Context::NetboxPanel,
        PanelKind::QuickView(_) => Context::Viewer,
        PanelKind::Files => Context::Panels,
    }
}

fn line_context(line: &crate::models::CommandLine) -> Context {
    if matches!(line.recall, Recall::Searching { .. }) {
        Context::CommandLineSearch
    } else if line.menu.is_some() {
        Context::CommandLineCompletion
    } else {
        Context::CommandLine
    }
}

fn task_context(modal: &Modal) -> Option<Context> {
    match modal {
        Modal::Browse => Some(Context::TaskList),
        Modal::Search(_) => Some(Context::TaskListSearch),
        _ => None,
    }
}

pub(super) fn validate(app: &App, invocation: Invocation) -> io::Result<()> {
    let active = current_context(app);
    let valid = match invocation {
        Invocation::Panels(_) => matches!(app.overlay, Overlay::None),
        Invocation::Viewer(ViewerCommand::Close) if matches!(app.overlay, Overlay::None) => false,
        Invocation::CommandLine(_) => matches!(
            active,
            Some(Context::CommandLine | Context::CommandLineCompletion)
        ),
        _ => active == Some(invocation.context()),
    };
    if valid {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "action is unavailable in the current context",
        ))
    }
}
