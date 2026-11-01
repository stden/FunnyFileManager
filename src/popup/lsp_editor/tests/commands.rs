//! Purpose: Typed server commands preserve list/view effects and nested modes.

use super::*;
use crate::action::lsp_editor::LspListCommand;
use crate::models::lsp_editor::Step;
use crate::popup::lsp_editor::apply;

#[test]
fn typed_list_and_view_commands_clear_the_previous_notice() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    editor_mut(&mut app).set_notice(Notice::Info("previous".to_owned()));
    apply(&mut app, LspListCommand::Move(Step::Down));
    assert_eq!(selected(&app), "css");
    assert_eq!(editor(&app).notice(), None);
    apply(&mut app, LspListCommand::View);
    editor_mut(&mut app).record_drawn(Drawn::View {
        lines: 40,
        page: 10,
    });
    apply(&mut app, LspListCommand::Move(Step::Down));
    assert_eq!(mode(&app), &Mode::View(ScrollTop::TOP.down(1, 30)));
    apply(&mut app, LspListCommand::Close);
    assert_eq!(mode(&app), &Mode::Browse);
}

#[test]
fn typed_list_commands_leave_forms_and_exit_questions_untouched() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    apply(&mut app, LspListCommand::Edit);
    editor_mut(&mut app).set_notice(Notice::Info("form".to_owned()));
    apply(&mut app, LspListCommand::Remove);
    assert!(matches!(mode(&app), Mode::Form(_)));
    assert_eq!(
        editor(&app).notice(),
        Some(&Notice::Info("form".to_owned()))
    );
    press(&mut app, KeyCode::Esc);
    apply(&mut app, LspListCommand::ToggleEnabled);
    apply(&mut app, LspListCommand::Close);
    assert!(matches!(mode(&app), Mode::ConfirmExit(_)));
    let draft = editor(&app).draft().clone();
    apply(&mut app, LspListCommand::Remove);
    assert!(matches!(mode(&app), Mode::ConfirmExit(_)));
    assert_eq!(editor(&app).draft(), &draft);
    app.close_overlay();
    apply(&mut app, LspListCommand::Close);
    assert!(matches!(app.overlay, Overlay::None));
}
