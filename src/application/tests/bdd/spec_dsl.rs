//! High-level domain executable specification DSL steps (English) and shared intent runners.

use cucumber::{given, when};

use super::{BehaviorWorld, Case};
use crate::{
    application::automation::{current_context, invoke},
    automation::catalog,
    keymap::Context,
    models::PanelSide,
};

pub(super) fn parse_side(side: &str) -> PanelSide {
    if matches!(side, "right" | "правой" | "правая" | "правую") {
        PanelSide::Right
    } else {
        PanelSide::Left
    }
}

pub(super) fn panel_entry_path(case: &Case, side: PanelSide, name: &str) -> std::path::PathBuf {
    case.app.panel(side).path.join(name)
}

pub(super) fn run_intent(case: &mut Case, context: &str, action: &str) {
    let invocation = catalog::resolve(context, action).expect("canonical domain command");
    invoke(&mut case.terminal, &mut case.app, invocation).expect("domain command succeeds");
    case.paint();
}

#[given("an isolated FFM specification workspace")]
pub(super) fn given_workspace(world: &mut BehaviorWorld) {
    world.case = Some(Case::new());
    world.case().paint();
}

#[given(expr = "in the {word} panel there is a file {string} with content {string}")]
pub(super) fn given_panel_file(
    world: &mut BehaviorWorld,
    side: String,
    name: String,
    content: String,
) {
    let panel_side = parse_side(&side);
    let case = world.case();
    let path = panel_entry_path(case, panel_side, &name);
    std::fs::write(path, content).unwrap();
    case.app.panel_mut(panel_side).load_entries().unwrap();
    case.paint();
}

#[given(expr = "the cursor in the {word} panel rests on {string}")]
pub(super) fn given_cursor_on(world: &mut BehaviorWorld, side: String, name: String) {
    let panel_side = parse_side(&side);
    let case = world.case();
    assert!(
        case.app.panel_mut(panel_side).select_named(&name),
        "entry {name:?} must exist in {side} panel"
    );
    case.paint();
}

#[when("the user switches focus to the opposite panel")]
pub(super) fn when_switch_focus(world: &mut BehaviorWorld) {
    run_intent(world.case(), "panels", "switch_panel");
}

#[when("the user swaps the panels")]
pub(super) fn when_swap_panels(world: &mut BehaviorWorld) {
    run_intent(world.case(), "panels", "swap_panels");
}

#[when(expr = "the user creates folder {string} in the active panel")]
pub(super) fn when_create_folder(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    run_intent(case, "panels", "mkdir");
    case.type_text(&name);
    case.press("Enter");
}

#[when(expr = "the user enters folder {string} in the active panel")]
pub(super) fn when_enter_folder(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    assert!(case.app.get_active_panel().select_named(&name));
    run_intent(case, "panels", "open");
}

#[when("the user navigates to the parent folder")]
pub(super) fn when_navigate_parent(world: &mut BehaviorWorld) {
    let case = world.case();
    assert!(case.app.get_active_panel().select_named(".."));
    run_intent(case, "panels", "open");
}

#[when("the user copies the current entry to the opposite panel")]
pub(super) fn when_copy_current(world: &mut BehaviorWorld) {
    let case = world.case();
    run_intent(case, "panels", "copy");
    case.press("Enter");
}

#[when("the user moves the current entry to the opposite panel")]
pub(super) fn when_move_current(world: &mut BehaviorWorld) {
    let case = world.case();
    run_intent(case, "panels", "move");
    case.press("Enter");
}

#[when(expr = "the user renames the current entry to {string}")]
pub(super) fn when_rename_current(world: &mut BehaviorWorld, new_name: String) {
    let case = world.case();
    run_intent(case, "panels", "rename");
    case.press("Ctrl+U");
    case.type_text(&new_name);
    case.press("Enter");
}

#[when("the user deletes the current entry with confirmation")]
pub(super) fn when_delete_confirmed(world: &mut BehaviorWorld) {
    let case = world.case();
    run_intent(case, "panels", "delete");
    case.press("y");
}

#[when("the user starts deleting the current entry and cancels")]
pub(super) fn when_delete_cancelled(world: &mut BehaviorWorld) {
    let case = world.case();
    run_intent(case, "panels", "delete");
    case.press("Esc");
}

#[when("the user marks the current entry")]
pub(super) fn when_mark_current(world: &mut BehaviorWorld) {
    run_intent(world.case(), "panels", "toggle_mark");
}

#[when("the user inverts the selection in the active panel")]
pub(super) fn when_invert_selection(world: &mut BehaviorWorld) {
    run_intent(world.case(), "panels", "invert_selection");
}

#[when(expr = "the user sorts the active panel by {word}")]
pub(super) fn when_sort_active_panel(world: &mut BehaviorWorld, criterion: String) {
    let action = if matches!(criterion.as_str(), "extension" | "расширению") {
        "sort_by_extension"
    } else {
        "sort_by_name"
    };
    run_intent(world.case(), "panels", action);
}

#[when("the user opens the current entry for viewing")]
pub(super) fn when_open_viewer(world: &mut BehaviorWorld) {
    run_intent(world.case(), "panels", "view");
}

#[when("the user opens the current entry for editing")]
pub(super) fn when_open_editor(world: &mut BehaviorWorld) {
    run_intent(world.case(), "panels", "edit");
}

#[when(expr = "the user appends line {string} in the editor and saves")]
pub(super) fn when_editor_append_and_save(world: &mut BehaviorWorld, line: String) {
    let case = world.case();
    run_intent(case, "editor", "document_end");
    run_intent(case, "editor", "newline");
    case.type_text(&line);
    run_intent(case, "editor", "save");
}

#[when("the user closes the active modal tool")]
pub(super) fn when_close_modal_tool(world: &mut BehaviorWorld) {
    let case = world.case();
    let ctx = current_context(&case.app)
        .map(Context::name)
        .expect("active context");
    run_intent(case, ctx, "close");
}
