//! High-level domain executable specification DSL steps (Russian / `# language: ru`).

use cucumber::{given, then, when};

use super::{
    spec_dsl::{
        given_cursor_on, given_panel_file, given_workspace, when_close_modal_tool,
        when_copy_current, when_create_folder, when_delete_cancelled, when_delete_confirmed,
        when_editor_append_and_save, when_enter_folder, when_invert_selection, when_mark_current,
        when_move_current, when_navigate_parent, when_open_editor, when_open_viewer,
        when_rename_current, when_sort_active_panel, when_swap_panels, when_switch_focus,
    },
    spec_dsl_checks::{
        then_active_domain_context, then_cursor_rests_on, then_focus_is_on,
        then_marked_entries_list, then_panel_entries_ordered, then_panel_entry_absent,
        then_panel_file_content, then_panel_folder_exists, then_rejected_already_exists,
    },
    BehaviorWorld,
};

#[given("изолированное рабочее пространство спецификации FFM")]
fn ru_given_workspace(world: &mut BehaviorWorld) {
    given_workspace(world);
}

#[given(expr = "в {word} панели находится файл {string} с содержимым {string}")]
fn ru_given_panel_file(world: &mut BehaviorWorld, side: String, name: String, content: String) {
    given_panel_file(world, side, name, content);
}

#[given(expr = "курсор в {word} панели установлен на {string}")]
fn ru_given_cursor_on(world: &mut BehaviorWorld, side: String, name: String) {
    given_cursor_on(world, side, name);
}

#[when("пользователь переключает фокус на противоположную панель")]
fn ru_when_switch_focus(world: &mut BehaviorWorld) {
    when_switch_focus(world);
}

#[when("пользователь меняет панели местами")]
fn ru_when_swap_panels(world: &mut BehaviorWorld) {
    when_swap_panels(world);
}

#[when(expr = "пользователь создаёт каталог {string} в активной панели")]
fn ru_when_create_folder(world: &mut BehaviorWorld, name: String) {
    when_create_folder(world, name);
}

#[when(expr = "пользователь входит в каталог {string} в активной панели")]
fn ru_when_enter_folder(world: &mut BehaviorWorld, name: String) {
    when_enter_folder(world, name);
}

#[when("пользователь переходит в родительский каталог")]
fn ru_when_navigate_parent(world: &mut BehaviorWorld) {
    when_navigate_parent(world);
}

#[when("пользователь копирует текущий элемент в противоположную панель")]
fn ru_when_copy_current(world: &mut BehaviorWorld) {
    when_copy_current(world);
}

#[when("пользователь перемещает текущий элемент в противоположную панель")]
fn ru_when_move_current(world: &mut BehaviorWorld) {
    when_move_current(world);
}

#[when(expr = "пользователь переименовывает текущий элемент в {string}")]
fn ru_when_rename_current(world: &mut BehaviorWorld, new_name: String) {
    when_rename_current(world, new_name);
}

#[when("пользователь удаляет текущий элемент с подтверждением")]
fn ru_when_delete_confirmed(world: &mut BehaviorWorld) {
    when_delete_confirmed(world);
}

#[when("пользователь начинает удаление текущего элемента и отменяет его")]
fn ru_when_delete_cancelled(world: &mut BehaviorWorld) {
    when_delete_cancelled(world);
}

#[when("пользователь отмечает текущий элемент")]
fn ru_when_mark_current(world: &mut BehaviorWorld) {
    when_mark_current(world);
}

#[when("пользователь инвертирует выделение в активной панели")]
fn ru_when_invert_selection(world: &mut BehaviorWorld) {
    when_invert_selection(world);
}

#[when(expr = "пользователь сортирует активную панель по {word}")]
fn ru_when_sort_active_panel(world: &mut BehaviorWorld, criterion: String) {
    when_sort_active_panel(world, criterion);
}

#[when("пользователь открывает текущий элемент для просмотра")]
fn ru_when_open_viewer(world: &mut BehaviorWorld) {
    when_open_viewer(world);
}

#[when("пользователь открывает текущий элемент для редактирования")]
fn ru_when_open_editor(world: &mut BehaviorWorld) {
    when_open_editor(world);
}

#[when(expr = "пользователь добавляет строку {string} в редакторе и сохраняет файл")]
fn ru_when_editor_append_and_save(world: &mut BehaviorWorld, line: String) {
    when_editor_append_and_save(world, line);
}

#[when("пользователь закрывает активный модальный инструмент")]
fn ru_when_close_modal_tool(world: &mut BehaviorWorld) {
    when_close_modal_tool(world);
}

#[then(expr = "фокус установлен на {word} панель")]
fn ru_then_focus_is_on(world: &mut BehaviorWorld, side: String) {
    then_focus_is_on(world, side);
}

#[then(expr = "курсор в {word} панели установлен на {string}")]
fn ru_then_cursor_rests_on(world: &mut BehaviorWorld, side: String, expected: String) {
    then_cursor_rests_on(world, side, expected);
}

#[then(expr = "в {word} панели файл {string} содержит {string}")]
fn ru_then_panel_file_content(
    world: &mut BehaviorWorld,
    side: String,
    name: String,
    expected: String,
) {
    then_panel_file_content(world, side, name, expected);
}

#[then(expr = "в {word} панели элемент {string} отсутствует")]
fn ru_then_panel_entry_absent(world: &mut BehaviorWorld, side: String, name: String) {
    then_panel_entry_absent(world, side, name);
}

#[then(expr = "в {word} панели существует каталог {string}")]
fn ru_then_panel_folder_exists(world: &mut BehaviorWorld, side: String, name: String) {
    then_panel_folder_exists(world, side, name);
}

#[then("операция отклонена из-за существования целевого файла")]
fn ru_then_rejected_already_exists(world: &mut BehaviorWorld) {
    then_rejected_already_exists(world);
}

#[then(expr = "активен доменный контекст {string}")]
fn ru_then_active_domain_context(world: &mut BehaviorWorld, expected: String) {
    then_active_domain_context(world, expected);
}

#[then(expr = "в {word} панели отмечены элементы {string}")]
fn ru_then_marked_entries_list(world: &mut BehaviorWorld, side: String, expected: String) {
    then_marked_entries_list(world, side, expected);
}

#[then(expr = "элементы в {word} панели упорядочены как {string}")]
fn ru_then_panel_entries_ordered(world: &mut BehaviorWorld, side: String, expected: String) {
    then_panel_entries_ordered(world, side, expected);
}
