//! Purpose: Maps Russian Cucumber scenarios to `FunnyFileManager` test steps.
//!
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use cucumber::{given, then, when, World as _};
use funny_file_manager::editor::{handle_edit_test_mode, TestEditorBehavior};
use funny_file_manager::handlers::{
    handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move, handle_view,
    process_popup_input,
};
use funny_file_manager::models::BookmarkSlot;
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Lifecycle;
use funny_file_manager::models::ViewMode;
use funny_file_manager::models::{App, Overlay, Panel, PanelSide, SortMode};
use funny_file_manager::s3::VirtualPath;
use funny_file_manager::status::Status;
use funny_file_manager::utils::{copy_dir_all, unique_destination};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::TempDir;

// ─── World ──────────────────────────────────────────────────────

#[derive(cucumber::World, Default)]
struct FfmWorld {
    temp_dir: Option<TempDir>,
    src_dir: Option<TempDir>,
    dst_dir: Option<TempDir>,
    app: Option<App>,
    panel: Option<Panel>,
    parsed_path: Option<VirtualPath>,
    formatted_size: Option<String>,
    unique_name: Option<String>,
    original_left_path: Option<PathBuf>,
}

impl std::fmt::Debug for FfmWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfmWorld")
            .field(
                "temp_dir",
                &self.temp_dir.as_ref().map(tempfile::TempDir::path),
            )
            .field(
                "src_dir",
                &self.src_dir.as_ref().map(tempfile::TempDir::path),
            )
            .field(
                "dst_dir",
                &self.dst_dir.as_ref().map(tempfile::TempDir::path),
            )
            .field("has_app", &self.app.is_some())
            .field("has_panel", &self.panel.is_some())
            .field("parsed_path", &self.parsed_path)
            .field("formatted_size", &self.formatted_size)
            .field("unique_name", &self.unique_name)
            .field("original_left_path", &self.original_left_path)
            .finish()
    }
}

impl FfmWorld {
    fn temp_path(&self) -> PathBuf {
        self.temp_dir
            .as_ref()
            .expect("нет временного каталога")
            .path()
            .to_path_buf()
    }
    fn src_path(&self) -> PathBuf {
        self.src_dir
            .as_ref()
            .expect("нет исходного каталога")
            .path()
            .to_path_buf()
    }
    fn dst_path(&self) -> PathBuf {
        self.dst_dir
            .as_ref()
            .expect("нет каталога назначения")
            .path()
            .to_path_buf()
    }
    const fn app(&mut self) -> &mut App {
        self.app.as_mut().expect("нет приложения")
    }
    const fn panel(&mut self) -> &mut Panel {
        self.panel.as_mut().expect("нет панели")
    }
}

// ═══════════════════════════════════════════════════════════════════
// ДОПУСТИМ (Given) — шаги подготовки
// ═══════════════════════════════════════════════════════════════════

#[given("существует временный каталог")]
fn given_temp_dir(world: &mut FfmWorld) {
    world.temp_dir = Some(TempDir::new().expect("создать временный каталог"));
}

#[given("существует исходный каталог")]
fn given_src_dir(world: &mut FfmWorld) {
    world.src_dir = Some(TempDir::new().expect("создать исходный каталог"));
}

#[given("существует пустой каталог назначения")]
fn given_empty_dst_dir(world: &mut FfmWorld) {
    world.dst_dir = Some(TempDir::new().expect("создать каталог назначения"));
}

#[given(expr = "файл {string} с содержимым {string}")]
fn given_file(world: &mut FfmWorld, name: String, content: String) {
    let base = world.temp_path();
    let path = base.join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, &content).expect("записать файл");
}

#[given(expr = "файл {string} с содержимым {string} в источнике")]
fn given_file_in_src(world: &mut FfmWorld, name: String, content: String) {
    fs::write(world.src_path().join(&name), &content).expect("записать файл в источник");
}

#[given(expr = "файл {string} с содержимым {string} в назначении")]
fn given_file_in_dst(world: &mut FfmWorld, name: String, content: String) {
    fs::write(world.dst_path().join(&name), &content).expect("записать файл в назначение");
}

#[given(expr = "файл {string} с содержимым {string} в подпути источника")]
fn given_file_in_src_subpath(world: &mut FfmWorld, name: String, content: String) {
    let path = world.src_path().join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, &content).expect("записать файл");
}

#[given(expr = "подкаталог {string} с файлом {string}")]
fn given_subdir_with_file(world: &mut FfmWorld, dir: String, file: String) {
    let sub = world.temp_path().join(&dir);
    fs::create_dir_all(&sub).expect("создать подкаталог");
    fs::write(sub.join(&file), "content").expect("записать файл");
}

#[given(expr = "подкаталог {string} в источнике")]
fn given_subdir_in_src(world: &mut FfmWorld, dir: String) {
    fs::create_dir_all(world.src_path().join(&dir)).expect("создать подкаталог в источнике");
}

#[given(expr = "подкаталог {string} в источнике с файлом {string}")]
fn given_subdir_in_src_with_file(world: &mut FfmWorld, dir: String, file: String) {
    let sub = world.src_path().join(&dir);
    fs::create_dir_all(&sub).expect("создать подкаталог");
    fs::write(sub.join(&file), "content").expect("записать файл");
}

#[given("панель во временном каталоге")]
fn given_panel_at_temp(world: &mut FfmWorld) {
    world.panel = Some(Panel::new(world.temp_path()).expect("создать панель"));
}

#[given(expr = "выделение на позиции {int}")]
fn given_selection_at(world: &mut FfmWorld, index: usize) {
    world.panel().select(Some(index));
}

#[given(expr = "я выбираю запись {string}")]
fn given_select_entry(world: &mut FfmWorld, name: String) {
    let panel = world.panel();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("запись '{name}' не найдена"));
    panel.select(Some(idx));
}

#[given("приложение настроено с исходным каталогом и каталогом назначения")]
fn given_app_src_dst(world: &mut FfmWorld) {
    let mut app = App::new().expect("создать приложение");
    app.left_panel = Panel::new(world.src_path()).expect("левая панель");
    app.right_panel = Panel::new(world.dst_path()).expect("правая панель");
    app.active_panel = PanelSide::Left;
    world.original_left_path = Some(app.left_panel.path.clone());
    world.app = Some(app);
}

#[given(expr = "приложение в подкаталоге {string} во временном каталоге")]
fn given_app_at_sub(world: &mut FfmWorld, sub: String) {
    let path = world.temp_path().join(&sub);
    let mut app = App::new().expect("создать приложение");
    app.left_panel = Panel::new(path).expect("панель");
    app.right_panel = Panel::new(world.temp_path()).expect("правая панель");
    world.app = Some(app);
}

#[given(expr = "я выбираю {string} в левой панели")]
fn given_select_left(world: &mut FfmWorld, name: String) {
    let app = world.app();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("запись '{name}' не найдена в левой панели"));
    app.left_panel.select(Some(idx));
}

#[given(expr = "я выбираю {string} в правой панели")]
fn given_select_right(world: &mut FfmWorld, name: String) {
    let app = world.app();
    let idx = app
        .right_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("запись '{name}' не найдена в правой панели"));
    app.right_panel.select(Some(idx));
}

#[given(expr = "{string} помечен")]
fn given_marked(world: &mut FfmWorld, name: String) {
    world.panel().marked.insert(FileName::new(name));
}

#[given(expr = "закладка {int} пуста")]
fn given_bookmark_empty(world: &mut FfmWorld, slot: usize) {
    world
        .app()
        .bookmarks
        .clear(BookmarkSlot::from_usize(slot).expect("slot"));
}

#[given("я переключаюсь на правую панель")]
fn given_switch_right(world: &mut FfmWorld) {
    world.app().active_panel = PanelSide::Right;
}

#[given("левая панель без выделения")]
fn given_no_selection(world: &mut FfmWorld) {
    world.app().left_panel.select(None);
}

#[given(expr = "{int} нумерованных файлов")]
fn given_numbered_files(world: &mut FfmWorld, count: usize) {
    let base = world.temp_path();
    for i in 0..count {
        fs::write(base.join(format!("file_{i:02}.txt")), "x").expect("записать");
    }
}

// ═══════════════════════════════════════════════════════════════════
// КОГДА (When) — шаги действий
// ═══════════════════════════════════════════════════════════════════

#[when("я нажимаю Down")]
fn when_down(world: &mut FfmWorld) {
    world.panel().next();
}

#[when("я нажимаю Up")]
fn when_up(world: &mut FfmWorld) {
    world.panel().previous();
}

#[when("я нажимаю Home")]
fn when_home(world: &mut FfmWorld) {
    world.panel().select(Some(0));
}

#[when("я нажимаю End")]
fn when_end(world: &mut FfmWorld) {
    let last = world.panel().entries.len().saturating_sub(1);
    world.panel().select(Some(last));
}

#[when("я нажимаю Enter")]
fn when_enter(world: &mut FfmWorld) {
    world.panel().enter().expect("enter");
}

#[when("я нажимаю PageUp")]
fn when_page_up(world: &mut FfmWorld) {
    let panel = world.panel();
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        panel.select(Some(cur.saturating_sub(10)));
    }
}

#[when("я нажимаю PageDown")]
fn when_page_down(world: &mut FfmWorld) {
    let panel = world.panel();
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        let max = panel.entries.len() - 1;
        panel.select(Some((cur + 10).min(max)));
    }
}

#[when("я нажимаю F3")]
fn when_f3(world: &mut FfmWorld) {
    handle_view(world.app()).expect("F3");
}

#[when("я нажимаю F5")]
fn when_f5(world: &mut FfmWorld) {
    handle_copy(world.app()).expect("F5");
}

#[when("я нажимаю F6")]
fn when_f6(world: &mut FfmWorld) {
    handle_move(world.app()).expect("F6");
}

#[when("я нажимаю F7")]
fn when_f7(world: &mut FfmWorld) {
    handle_mkdir(world.app()).expect("F7");
}

#[when("я нажимаю F8")]
fn when_f8(world: &mut FfmWorld) {
    handle_delete_confirm(world.app()).expect("F8");
}

#[when("я подтверждаю удаление")]
fn when_confirm_delete(world: &mut FfmWorld) {
    handle_delete(world.app()).expect("удаление");
}

#[when("я отменяю удаление")]
fn when_cancel_delete(world: &mut FfmWorld) {
    world.app().close_overlay();
    "Delete cancelled".clone_into(&mut world.app().status_text());
}

#[when("я нажимаю Tab")]
fn when_tab(world: &mut FfmWorld) {
    world.app().switch_panel();
}

#[when("я нажимаю Left")]
fn when_left(world: &mut FfmWorld) {
    world.app().active_panel = PanelSide::Left;
}

#[when("я нажимаю Right")]
fn when_right(world: &mut FfmWorld) {
    world.app().active_panel = PanelSide::Right;
}

#[when("я меняю панели местами")]
fn when_swap(world: &mut FfmWorld) {
    world.app().swap_panels();
}

#[when("я переключаю режим консоли")]
fn when_toggle_console(world: &mut FfmWorld) {
    let app = world.app();
    app.view = app.view.toggled();
}

#[when("я перехожу в корневой каталог")]
fn when_go_root(world: &mut FfmWorld) {
    let app = world.app();
    app.get_active_panel().path = PathBuf::from("/");
    let _ = app.get_active_panel().load_entries();
}

#[when("я перехожу в родительский каталог")]
fn when_go_parent(world: &mut FfmWorld) {
    let app = world.app();
    if let Some(parent) = app
        .get_active_panel()
        .path
        .parent()
        .map(std::path::Path::to_path_buf)
    {
        app.get_active_panel().path = parent;
        let _ = app.get_active_panel().load_entries();
    }
}

#[when("я переключаюсь на правую панель")]
fn when_switch_right(world: &mut FfmWorld) {
    world.app().active_panel = PanelSide::Right;
}

#[when(expr = "я выбираю {string} в правой панели")]
fn when_select_right(world: &mut FfmWorld, name: String) {
    let app = world.app();
    let idx = app
        .right_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("запись '{name}' не найдена в правой панели"));
    app.right_panel.select(Some(idx));
}

// Быстрый поиск
#[when(expr = "я ищу {string}")]
fn when_quick_search(world: &mut FfmWorld, q: String) {
    world.panel().find_first_matching(&q);
}

// Закладки
#[when(expr = "я устанавливаю закладку {int}")]
fn when_set_bookmark(world: &mut FfmWorld, slot: usize) {
    let path = world.app().get_active_panel().path.clone();
    world
        .app()
        .bookmarks
        .set(BookmarkSlot::from_usize(slot).expect("slot"), path);
}

#[when(expr = "я перехожу к закладке {int}")]
fn when_go_bookmark(world: &mut FfmWorld, slot: usize) {
    let app = world.app();
    if let Some(path) = app
        .bookmarks
        .get(BookmarkSlot::from_usize(slot).expect("slot"))
        .map(Path::to_path_buf)
    {
        let panel = app.get_active_panel();
        panel.path = path;
        let _ = panel.load_entries();
        panel.ensure_selection();
    } else {
        app.set_status(Status::CommandOutput(format!("Bookmark {slot} is empty")));
    }
}

// Сортировка
#[when("я создаю панель с сортировкой по расширению")]
fn when_sort_ext(world: &mut FfmWorld) {
    let mut p = Panel::new(world.temp_path()).expect("панель");
    p.sort_mode = SortMode::Extension;
    p.load_entries().expect("загрузить");
    world.panel = Some(p);
}

#[when(expr = "я переключаю режим сортировки {int} раза")]
fn when_cycle_sort_3(world: &mut FfmWorld, n: usize) {
    for _ in 0..n {
        world.panel().cycle_sort_mode();
    }
}

#[when(expr = "я переключаю режим сортировки {int} раз")]
fn when_cycle_sort_n(world: &mut FfmWorld, n: usize) {
    for _ in 0..n {
        world.panel().cycle_sort_mode();
    }
}

// Пометка
#[when("я переключаю пометку")]
fn when_toggle_mark(world: &mut FfmWorld) {
    world.panel().toggle_mark_selected();
}

#[when("я снимаю все пометки")]
fn when_clear_marks(world: &mut FfmWorld) {
    world.panel().clear_marks();
}

#[when("я выделяю все файлы")]
fn when_select_all(world: &mut FfmWorld) {
    let app = world.app();
    let panel = app.get_active_panel();
    let names: Vec<String> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir() && e.name != "..")
        .map(|e| e.name.clone())
        .collect();
    for n in &names {
        panel.marked.insert(FileName::new(n.clone()));
    }
}

// Командная строка
#[when(expr = "я выполняю команду {string}")]
fn when_exec_cmd(world: &mut FfmWorld, cmd: String) {
    let app = world.app();
    app.set_overlay(Overlay::CommandLine(cmd.into()));
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::empty(),
    );
    process_popup_input(app, key);
}

// Создание файлов
#[when("я открываю диалог нового файла")]
fn when_open_new_file(world: &mut FfmWorld) {
    world
        .app()
        .set_overlay(Overlay::NewFileInput(String::new()));
}

#[when(expr = "я ввожу имя файла {string}")]
fn when_type_filename(world: &mut FfmWorld, name: String) {
    world.app().set_overlay(Overlay::NewFileInput(name));
}

#[when("я подтверждаю создание файла")]
fn when_confirm_new_file(world: &mut FfmWorld) {
    let app = world.app();
    let input = app.get_new_file_input().unwrap_or_default().to_owned();
    app.close_overlay();
    let input = input.trim().to_owned();
    if input.is_empty() {
        app.set_status(Status::CommandOutput("Filename cannot be empty".to_owned()));
        return;
    }
    let path = app.get_active_panel().path.join(&input);
    if path.exists() {
        app.set_status(Status::CommandOutput(format!(
            "File already exists: {input}"
        )));
    } else {
        fs::File::create(&path).expect("создать файл");
        app.set_status(Status::CommandOutput(format!("Created file: {input}")));
        let _ = funny_file_manager::utils::refresh_panels(app);
    }
}

// Просмотрщик
#[when("я закрываю просмотрщик")]
fn when_close_viewer(world: &mut FfmWorld) {
    world.app().close_overlay();
}

// Выход и диски
#[when("я устанавливаю флаг выхода")]
fn when_set_quit(world: &mut FfmWorld) {
    world.app().lifecycle = Lifecycle::Quitting;
}

#[when(expr = "я открываю выбор дисков для {word}")]
fn when_open_drives(world: &mut FfmWorld, side: String) {
    let app = world.app();
    let panel_side = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    app.set_overlay(Overlay::DriveSelection {
        side: panel_side,
        selected_index: 0,
    });
}

// Редактор (F4)
#[when("я нажимаю F4 с успешным поведением")]
fn when_f4_success(world: &mut FfmWorld) {
    handle_edit_test_mode(world.app(), TestEditorBehavior::Success).expect("F4 успех");
}

#[when("я нажимаю F4 с ошибкой запуска")]
fn when_f4_fail_launch(world: &mut FfmWorld) {
    handle_edit_test_mode(world.app(), TestEditorBehavior::FailLaunch).expect("F4 ошибка запуска");
}

#[when("я нажимаю F4 с ошибкой выхода")]
fn when_f4_exit_error(world: &mut FfmWorld) {
    handle_edit_test_mode(world.app(), TestEditorBehavior::ExitError).expect("F4 ошибка выхода");
}

// VirtualPath
#[when(expr = "я разбираю путь {string}")]
fn when_parse_path(world: &mut FfmWorld, path: String) {
    world.parsed_path = Some(VirtualPath::parse(&path));
}

// Утилиты
#[when(expr = "я форматирую размер {int}")]
fn when_format_size(world: &mut FfmWorld, size: u64) {
    world.formatted_size = Some(FileSize::from_bytes(size).to_string());
}

#[when(expr = "я запрашиваю уникальное имя для {string}")]
fn when_unique_name(world: &mut FfmWorld, name: String) {
    let result =
        unique_destination(world.temp_path().as_path(), OsStr::new(&name)).expect("уникальное имя");
    world.unique_name = Some(
        result
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    );
}

#[when(expr = "я копирую каталог {string} в {string}")]
fn when_copy_dir(world: &mut FfmWorld, src: String, dst: String) {
    let base = world.temp_path();
    copy_dir_all(&base.join(&src), &base.join(&dst)).expect("копировать каталог");
}

// ═══════════════════════════════════════════════════════════════════
// ТОГДА (Then) — шаги проверки
// ═══════════════════════════════════════════════════════════════════

#[then(expr = "выделение должно быть на позиции {int}")]
fn then_sel(world: &mut FfmWorld, i: usize) {
    assert_eq!(world.panel().selected(), Some(i));
}

#[then(expr = "путь панели должен заканчиваться на {string}")]
fn then_path_ends(world: &mut FfmWorld, s: String) {
    let p = if let Some(ref panel) = world.panel {
        panel.path.display().to_string()
    } else {
        world.app().get_active_panel().path.display().to_string()
    };
    assert!(p.ends_with(&s), "'{p}' должен заканчиваться на '{s}'");
}

#[then(expr = "путь активной панели должен заканчиваться на {string}")]
fn then_active_ends(world: &mut FfmWorld, s: String) {
    let p = world.app().get_active_panel().path.display().to_string();
    assert!(p.ends_with(&s), "'{p}' должен заканчиваться на '{s}'");
}

#[then(expr = "путь активной панели должен быть {string}")]
fn then_active_is(world: &mut FfmWorld, s: String) {
    let p = world.app().get_active_panel().path.display().to_string();
    assert_eq!(p, s);
}

#[then(expr = "{string} должен существовать в назначении")]
fn then_dst_exists(world: &mut FfmWorld, n: String) {
    assert!(
        world.dst_path().join(&n).exists(),
        "{n} должен существовать в назначении"
    );
}

#[then(expr = "{string} должен существовать в источнике")]
fn then_src_has(world: &mut FfmWorld, n: String) {
    assert!(
        world.src_path().join(&n).exists(),
        "{n} должен существовать в источнике"
    );
}

#[then(expr = "{string} не должен существовать в источнике")]
fn then_src_not(world: &mut FfmWorld, n: String) {
    assert!(
        !world.src_path().join(&n).exists(),
        "{n} не должен существовать"
    );
}

#[then(expr = "сообщение статуса должно содержать {string}")]
fn then_status(world: &mut FfmWorld, t: String) {
    let m = &world.app().status_text();
    assert!(m.contains(&t), "'{m}' должно содержать '{t}'");
}

#[then("сообщение статуса не должно быть пустым")]
fn then_status_not_empty(world: &mut FfmWorld) {
    assert!(!world.app().status_text().is_empty());
}

#[then("должен появиться диалог подтверждения удаления")]
fn then_del_confirm(world: &mut FfmWorld) {
    assert!(world.app().get_delete_confirm().is_some());
}

#[then("просмотрщик должен быть открыт")]
fn then_viewer_open(world: &mut FfmWorld) {
    assert!(world.app().get_viewer().is_some());
}

#[then("просмотрщик не должен быть открыт")]
fn then_viewer_closed(world: &mut FfmWorld) {
    assert!(world.app().get_viewer().is_none());
}

#[then(expr = "заголовок просмотрщика должен содержать {string}")]
fn then_viewer_title(world: &mut FfmWorld, t: String) {
    let title = &world.app().get_viewer().expect("просмотрщик").title;
    assert!(title.contains(&t), "'{title}' должен содержать '{t}'");
}

#[then(expr = "просмотрщик должен показывать {string}")]
fn then_viewer_shows(world: &mut FfmWorld, t: String) {
    let lines = &world.app().get_viewer().expect("просмотрщик").lines;
    assert!(lines.join("\n").contains(&t));
}

#[then(expr = "выбранная запись должна быть {string}")]
fn then_selected(world: &mut FfmWorld, n: String) {
    assert_eq!(
        world.panel().get_selected_entry().expect("выделение").name,
        n
    );
}

#[then(expr = "активная панель должна быть {word}")]
fn then_active_panel(world: &mut FfmWorld, side: String) {
    let expected = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    assert_eq!(world.app().active_panel, expected);
}

#[then("путь левой панели должен отличаться от исходного")]
fn then_left_differs(world: &mut FfmWorld) {
    let orig = world.original_left_path.clone();
    if let Some(orig) = orig {
        assert_ne!(&world.app().left_panel.path, &orig);
    }
}

#[then("режим консоли должен быть включён")]
fn then_console_on(world: &mut FfmWorld) {
    assert_eq!(world.app().view, ViewMode::Console);
}

#[then("режим консоли должен быть выключен")]
fn then_console_off(world: &mut FfmWorld) {
    assert_ne!(world.app().view, ViewMode::Console);
}

#[then(expr = "режим сортировки должен быть {word}")]
fn then_sort_mode(world: &mut FfmWorld, mode: String) {
    let expected = match mode.as_str() {
        "Name" => SortMode::Name,
        "Extension" => SortMode::Extension,
        "Size" => SortMode::Size,
        _ => panic!("неизвестный режим сортировки: {mode}"),
    };
    assert_eq!(world.app().get_active_panel().sort_mode, expected);
}

#[then("режим сортировки должен вернуться к Name")]
fn then_sort_name(world: &mut FfmWorld) {
    assert_eq!(world.panel().sort_mode, SortMode::Name);
}

#[then("первое расширение должно быть раньше последнего по алфавиту")]
fn then_ext_sorted(world: &mut FfmWorld) {
    let files: Vec<_> = world
        .panel()
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir() && e.name != "..")
        .collect();
    if files.len() >= 2 {
        let a = files[0].name.rsplit('.').next().unwrap_or("");
        let b = files[files.len() - 1].name.rsplit('.').next().unwrap_or("");
        assert!(a <= b, "'{a}' должно быть <= '{b}'");
    }
}

// Пометка
#[then(expr = "{string} должен быть помечен")]
fn then_marked(world: &mut FfmWorld, n: String) {
    assert!(world.panel().marked.contains(n.as_str()));
}

#[then(expr = "{string} не должен быть помечен")]
fn then_not_marked(world: &mut FfmWorld, n: String) {
    assert!(!world.panel().marked.contains(n.as_str()));
}

#[then("панель не должна иметь пометок")]
fn then_panel_no_marks(world: &mut FfmWorld) {
    assert!(world.panel().marked.is_empty());
}

#[then("активная панель должна иметь пометки")]
fn then_has_marks(world: &mut FfmWorld) {
    assert!(!world.app().get_active_panel().marked.is_empty());
}

#[then("активная панель не должна иметь пометок")]
fn then_no_marks(world: &mut FfmWorld) {
    assert!(world.app().get_active_panel().marked.is_empty());
}

// Закладки
#[then(expr = "закладка {int} должна указывать на {string}")]
fn then_bookmark(world: &mut FfmWorld, slot: usize, sub: String) {
    let bm = world
        .app()
        .bookmarks
        .get(BookmarkSlot::from_usize(slot).expect("slot"))
        .expect("закладка");
    assert!(bm.display().to_string().contains(&sub));
}

// Справка и выход
#[then("справка должна быть видна")]
fn then_help_on(world: &mut FfmWorld) {
    assert!(matches!(world.app().overlay, Overlay::Help));
}

#[then("приложение должно завершаться")]
fn then_quitting(world: &mut FfmWorld) {
    assert!(!world.app().lifecycle.is_running());
}

#[then("диалог выбора дисков должен быть видимым")]
fn then_drives_visible(world: &mut FfmWorld) {
    assert!(matches!(
        world.app().overlay,
        Overlay::DriveSelection { .. }
    ));
}

// VirtualPath
#[then("тип пути должен быть Local")]
fn then_local(world: &mut FfmWorld) {
    assert!(matches!(world.parsed_path, Some(VirtualPath::Local(_))));
}

#[then("тип пути должен быть S3")]
fn then_s3(world: &mut FfmWorld) {
    assert!(matches!(world.parsed_path, Some(VirtualPath::S3 { .. })));
}

#[then(expr = "профиль должен быть {string}")]
fn then_profile(world: &mut FfmWorld, p: String) {
    match world.parsed_path.as_ref().expect("путь") {
        VirtualPath::S3 { profile, .. } => assert_eq!(profile, &p),
        _ => panic!("ожидался S3 путь"),
    }
}

#[then(expr = "бакет должен быть {string}")]
fn then_bucket(world: &mut FfmWorld, b: String) {
    match world.parsed_path.as_ref().expect("путь") {
        VirtualPath::S3 { bucket, .. } => assert_eq!(bucket.as_deref(), Some(b.as_str())),
        _ => panic!("ожидался S3 путь"),
    }
}

#[then("бакет должен быть пустым")]
fn then_bucket_empty(world: &mut FfmWorld) {
    match world.parsed_path.as_ref().expect("путь") {
        VirtualPath::S3 { bucket, .. } => assert!(bucket.is_none()),
        _ => panic!("ожидался S3 путь"),
    }
}

#[then(expr = "префикс должен быть {string}")]
fn then_prefix(world: &mut FfmWorld, p: String) {
    match world.parsed_path.as_ref().expect("путь") {
        VirtualPath::S3 { prefix, .. } => assert_eq!(prefix, &p),
        _ => panic!("ожидался S3 путь"),
    }
}

// Утилиты
#[then(expr = "отформатированный размер должен быть {string}")]
fn then_formatted(world: &mut FfmWorld, s: String) {
    assert_eq!(world.formatted_size.as_deref(), Some(s.as_str()));
}

#[then(expr = "уникальное имя должно быть {string}")]
fn then_unique(world: &mut FfmWorld, s: String) {
    assert_eq!(world.unique_name.as_deref(), Some(s.as_str()));
}

#[then(expr = "{string} должен существовать во временном каталоге")]
fn then_temp_exists(world: &mut FfmWorld, n: String) {
    assert!(
        world.temp_path().join(&n).exists(),
        "{n} должен существовать"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Новые шаги для покрытия оверлеев и расширенных операций
// ═══════════════════════════════════════════════════════════════════

// --- GIVEN: новые шаги подготовки ---

#[given(expr = "файл {string} с многострочным содержимым в источнике")]
fn given_multiline_file_in_src(world: &mut FfmWorld, name: String) {
    let content = (0..50)
        .map(|i| format!("строка {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(world.src_path().join(&name), &content).expect("записать файл");
}

#[given(expr = "командная строка активна с текстом {string}")]
fn given_cmdline_active(world: &mut FfmWorld, text: String) {
    world.app().set_overlay(Overlay::CommandLine(text.into()));
}

#[given("диалог нового файла открыт")]
fn given_new_file_dialog(world: &mut FfmWorld) {
    world
        .app()
        .set_overlay(Overlay::NewFileInput(String::new()));
}

#[given(expr = "диалог нового файла открыт с текстом {string}")]
fn given_new_file_dialog_with_text(world: &mut FfmWorld, text: String) {
    world.app().set_overlay(Overlay::NewFileInput(text));
}

#[given(expr = "я помечаю {string} в левой панели")]
fn given_mark_left(world: &mut FfmWorld, name: String) {
    world.app().left_panel.marked.insert(FileName::new(name));
}

#[given(expr = "панель во временном каталоге с сортировкой по размеру")]
fn given_panel_sorted_by_size(world: &mut FfmWorld) {
    let mut p = Panel::new(world.temp_path()).expect("панель");
    p.sort_mode = SortMode::Size;
    p.load_entries().expect("загрузить");
    world.panel = Some(p);
}

// --- WHEN: новые шаги действий ---

#[when("я прокручиваю просмотрщик вниз")]
fn when_scroll_viewer_down(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я прокручиваю просмотрщик вверх")]
fn when_scroll_viewer_up(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я отправляю клавишу Esc в оверлей")]
fn when_overlay_esc(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я отправляю клавишу Enter в оверлей")]
fn when_overlay_enter(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я отправляю клавишу Y в оверлей")]
fn when_overlay_y(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я отправляю клавишу N в оверлей")]
fn when_overlay_n(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Char('N'), KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я нажимаю Backspace в командной строке")]
fn when_cmdline_backspace(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when(expr = "я ввожу символ {string} в командную строку")]
fn when_cmdline_char(world: &mut FfmWorld, ch: String) {
    let c = ch.chars().next().unwrap();
    let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when(expr = "я ввожу символ {string} в диалог нового файла")]
fn when_new_file_char(world: &mut FfmWorld, ch: String) {
    let c = ch.chars().next().unwrap();
    let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я отправляю клавишу Enter в диалог нового файла")]
fn when_new_file_enter(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я нажимаю Backspace в диалоге нового файла")]
fn when_new_file_backspace(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я отправляю клавишу Esc в диалог нового файла")]
fn when_new_file_esc(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я нажимаю Down в выборе дисков")]
fn when_drives_down(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я нажимаю Up в выборе дисков")]
fn when_drives_up(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я нажимаю End в выборе дисков")]
fn when_drives_end(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::End, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я нажимаю Home в выборе дисков")]
fn when_drives_home(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Home, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when("я отправляю клавишу Enter в выбор дисков")]
fn when_drives_enter(world: &mut FfmWorld) {
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    process_popup_input(world.app(), key);
}

#[when(expr = "я выбираю запись {string}")]
fn when_select_entry(world: &mut FfmWorld, name: String) {
    let panel = world.panel();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("запись '{name}' не найдена"));
    panel.select(Some(idx));
}

#[when(expr = "я ищу следующее совпадение {string}")]
fn when_find_next(world: &mut FfmWorld, q: String) {
    world.panel().find_next_matching(&q);
}

#[when("я перезагружаю панель")]
fn when_reload_panel(world: &mut FfmWorld) {
    world.panel().load_entries().expect("загрузить");
}

#[when("я вызываю ensure_selection")]
fn when_ensure_selection(world: &mut FfmWorld) {
    world.panel().ensure_selection();
}

// --- THEN: новые шаги проверки ---

#[then(expr = "позиция прокрутки просмотрщика должна быть больше {int}")]
fn then_scroll_gt(world: &mut FfmWorld, n: usize) {
    let scroll = world
        .app()
        .get_viewer()
        .expect("просмотрщик")
        .scroll
        .lines();
    assert!(scroll > n, "прокрутка {scroll} должна быть > {n}");
}

#[then(expr = "позиция прокрутки просмотрщика должна быть {int}")]
fn then_scroll_eq(world: &mut FfmWorld, n: usize) {
    let scroll = world
        .app()
        .get_viewer()
        .expect("просмотрщик")
        .scroll
        .lines();
    assert_eq!(scroll, n);
}

#[then("справка не должна быть видна")]
fn then_help_off(world: &mut FfmWorld) {
    assert!(!matches!(world.app().overlay, Overlay::Help));
}

#[then("командная строка должна быть закрыта")]
fn then_cmdline_closed(world: &mut FfmWorld) {
    assert!(world.app().get_command_line().is_none());
}

#[then(expr = "командная строка должна содержать {string}")]
fn then_cmdline_has(world: &mut FfmWorld, t: String) {
    let cmd = world.app().get_command_line().expect("командная строка");
    assert_eq!(cmd, t.as_str());
}

#[then("диалог выбора дисков не должен быть видимым")]
fn then_drives_hidden(world: &mut FfmWorld) {
    assert!(!matches!(
        world.app().overlay,
        Overlay::DriveSelection { .. }
    ));
}

#[then(expr = "индекс выбора дисков должен быть {int}")]
fn then_drive_index(world: &mut FfmWorld, idx: usize) {
    assert!(matches!(
        world.app().overlay,
        Overlay::DriveSelection {
            selected_index,
            ..
        } if selected_index == idx
    ));
}

#[then("диалог нового файла должен быть закрыт")]
fn then_new_file_closed(world: &mut FfmWorld) {
    assert!(world.app().get_new_file_input().is_none());
}

#[then(expr = "буфер диалога нового файла должен содержать {string}")]
fn then_new_file_buf(world: &mut FfmWorld, t: String) {
    let buf = world.app().get_new_file_input().expect("диалог");
    assert_eq!(buf, t.as_str());
}

#[then(expr = "{string} должен быть каталогом в источнике")]
fn then_dir_in_src(world: &mut FfmWorld, n: String) {
    assert!(world.src_path().join(&n).is_dir());
}

#[then(expr = "панель должна содержать запись {string}")]
fn then_panel_has_entry(world: &mut FfmWorld, name: String) {
    let panel = world.panel();
    assert!(
        panel.entries.iter().any(|e| e.name == name),
        "панель не содержит запись '{name}'"
    );
}

#[then(expr = "первая файловая запись должна быть {string}")]
fn then_first_file_entry(world: &mut FfmWorld, name: String) {
    let panel = world.panel();
    let first_file = panel
        .entries
        .iter()
        .find(|e| !e.kind.is_dir() && e.name != "..")
        .expect("нет файлов");
    assert_eq!(first_file.name, name);
}

#[then(expr = "action_entry_names должен содержать {int} элемента")]
fn then_action_names_count_a(world: &mut FfmWorld, n: usize) {
    assert_eq!(world.panel().action_entry_names().len(), n);
}

#[then(expr = "action_entry_names должен содержать {int} элемент")]
fn then_action_names_count_b(world: &mut FfmWorld, n: usize) {
    assert_eq!(world.panel().action_entry_names().len(), n);
}

#[then("панель должна иметь допустимое выделение")]
fn then_valid_selection(world: &mut FfmWorld) {
    let panel = world.panel();
    if !panel.entries.is_empty() {
        let sel = panel.selected();
        assert!(sel.is_some());
        assert!(sel.unwrap() < panel.entries.len());
    }
}

#[then("неактивная панель не должна быть пустой")]
fn then_inactive_not_empty(world: &mut FfmWorld) {
    assert!(!world.app().get_inactive_panel().entries.is_empty());
}

fn main() {
    futures::executor::block_on(FfmWorld::run("tests/features_ru"));
}
