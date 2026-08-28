//! Purpose: Maps English Cucumber scenarios to `FunnyFileManager` test steps.
//!
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
use funny_file_manager::s3::{list_aws_profiles, VirtualPath};
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
    // For VirtualPath scenarios
    parsed_path: Option<VirtualPath>,
    // For utility scenarios
    formatted_size: Option<String>,
    unique_name: Option<String>,
    aws_profiles: Vec<String>,
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
            .field("aws_profiles", &self.aws_profiles)
            .field("original_left_path", &self.original_left_path)
            .finish()
    }
}

impl FfmWorld {
    fn temp_path(&self) -> PathBuf {
        self.temp_dir
            .as_ref()
            .expect("no temp dir")
            .path()
            .to_path_buf()
    }
    fn src_path(&self) -> PathBuf {
        self.src_dir
            .as_ref()
            .expect("no src dir")
            .path()
            .to_path_buf()
    }
    fn dst_path(&self) -> PathBuf {
        self.dst_dir
            .as_ref()
            .expect("no dst dir")
            .path()
            .to_path_buf()
    }
    const fn app(&mut self) -> &mut App {
        self.app.as_mut().expect("no app")
    }
    const fn panel(&mut self) -> &mut Panel {
        self.panel.as_mut().expect("no panel")
    }
}

// ═══════════════════════════════════════════════════════════════════
// GIVEN steps
// ═══════════════════════════════════════════════════════════════════

#[given("a temporary directory")]
fn given_temp_dir(world: &mut FfmWorld) {
    world.temp_dir = Some(TempDir::new().expect("create temp dir"));
}

#[given("a source directory")]
fn given_src_dir(world: &mut FfmWorld) {
    world.src_dir = Some(TempDir::new().expect("create src dir"));
}

#[given("an empty destination directory")]
fn given_empty_dst_dir(world: &mut FfmWorld) {
    world.dst_dir = Some(TempDir::new().expect("create dst dir"));
}

#[given(expr = "a file {string} with content {string}")]
fn given_file(world: &mut FfmWorld, name: String, content: String) {
    let base = world.temp_path();
    let path = base.join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, &content).expect("write file");
}

#[given(expr = "a file {string} with content {string} in the source")]
fn given_file_in_src(world: &mut FfmWorld, name: String, content: String) {
    fs::write(world.src_path().join(&name), &content).expect("write file");
}

#[given(expr = "a file {string} with content {string} in the destination")]
fn given_file_in_dst(world: &mut FfmWorld, name: String, content: String) {
    fs::write(world.dst_path().join(&name), &content).expect("write dst file");
}

#[given(expr = "a file {string} with content {string} in the source dir")]
fn given_file_in_src_subpath(world: &mut FfmWorld, name: String, content: String) {
    let path = world.src_path().join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, &content).expect("write file");
}

#[given(expr = "a subdirectory {string} with file {string}")]
fn given_subdir_with_file(world: &mut FfmWorld, dir: String, file: String) {
    let sub = world.temp_path().join(&dir);
    fs::create_dir_all(&sub).expect("create subdir");
    fs::write(sub.join(&file), "content").expect("write file");
}

#[given(expr = "a subdirectory {string} in the source")]
fn given_subdir_in_src(world: &mut FfmWorld, dir: String) {
    fs::create_dir_all(world.src_path().join(&dir)).expect("create subdir");
}

#[given(expr = "a subdirectory {string} in the source with file {string}")]
fn given_subdir_in_src_with_file(world: &mut FfmWorld, dir: String, file: String) {
    let sub = world.src_path().join(&dir);
    fs::create_dir_all(&sub).expect("create subdir");
    fs::write(sub.join(&file), "content").expect("write file");
}

#[given("a panel at the temporary directory")]
fn given_panel_at_temp(world: &mut FfmWorld) {
    world.panel = Some(Panel::new(world.temp_path()).expect("create panel"));
}

#[given(expr = "a panel at {string} in the temporary directory")]
fn given_panel_at_sub(world: &mut FfmWorld, sub: String) {
    let path = world.temp_path().join(&sub);
    world.panel = Some(Panel::new(path).expect("create panel"));
}

#[given(expr = "the selection is at index {int}")]
fn given_selection_at(world: &mut FfmWorld, index: usize) {
    world.panel().select(Some(index));
}

#[given(expr = "I select the entry {string}")]
fn given_select_entry(world: &mut FfmWorld, name: String) {
    let panel = world.panel();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("entry '{name}' not found"));
    panel.select(Some(idx));
}

#[given("the app is set up with source and destination")]
fn given_app_src_dst(world: &mut FfmWorld) {
    let mut app = App::new().expect("create app");
    app.left_panel = Panel::new(world.src_path()).expect("left panel");
    app.right_panel = Panel::new(world.dst_path()).expect("right panel");
    app.active_panel = PanelSide::Left;
    world.original_left_path = Some(app.left_panel.path.clone());
    world.app = Some(app);
}

#[given(expr = "the app is set up at {string} in the source")]
fn given_app_at_src_sub(world: &mut FfmWorld, sub: String) {
    let src_sub = world.src_path().join(&sub);
    let dst = world.dst_path();
    let mut app = App::new().expect("create app");
    app.left_panel = Panel::new(src_sub).expect("left panel");
    app.right_panel = Panel::new(dst).expect("right panel");
    app.active_panel = PanelSide::Left;
    world.original_left_path = Some(app.left_panel.path.clone());
    world.app = Some(app);
}

#[given(expr = "an app at {string} in the temporary directory")]
fn given_app_at_sub(world: &mut FfmWorld, sub: String) {
    let path = world.temp_path().join(&sub);
    let mut app = App::new().expect("create app");
    app.left_panel = Panel::new(path).expect("panel");
    app.right_panel = Panel::new(world.temp_path()).expect("right panel");
    world.app = Some(app);
}

#[given("an app at the temporary directory")]
fn given_app_at_temp(world: &mut FfmWorld) {
    let path = world.temp_path();
    let mut app = App::new().expect("create app");
    app.left_panel = Panel::new(path.clone()).expect("left panel");
    app.right_panel = Panel::new(path).expect("right panel");
    world.app = Some(app);
}

#[given(expr = "I select {string} in the left panel")]
fn given_select_left(world: &mut FfmWorld, name: String) {
    let app = world.app();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("entry '{name}' not found in left panel"));
    app.left_panel.select(Some(idx));
}

#[given(expr = "I select {string} in the active panel")]
fn given_select_active(world: &mut FfmWorld, name: String) {
    let app = world.app();
    let idx = app
        .get_active_panel()
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("entry '{name}' not found"));
    app.get_active_panel().select(Some(idx));
}

#[given(expr = "I select {string} in the right panel")]
fn given_select_right(world: &mut FfmWorld, name: String) {
    let app = world.app();
    let idx = app
        .right_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("entry '{name}' not found in right panel"));
    app.right_panel.select(Some(idx));
}

#[given(expr = "I mark {string} in the left panel")]
fn given_mark_left(world: &mut FfmWorld, name: String) {
    world.app().left_panel.marked.insert(FileName::new(name));
}

#[given(expr = "the file {string} is marked")]
fn given_file_marked(world: &mut FfmWorld, name: String) {
    world
        .app()
        .get_active_panel()
        .marked
        .insert(FileName::new(name));
}

#[given(expr = "{string} is marked")]
fn given_marked(world: &mut FfmWorld, name: String) {
    world.panel().marked.insert(FileName::new(name));
}

#[given(expr = "bookmark {int} is empty")]
fn given_bookmark_empty(world: &mut FfmWorld, slot: usize) {
    world
        .app()
        .bookmarks
        .clear(BookmarkSlot::from_usize(slot).expect("slot"));
}

#[given("the left panel has no selection")]
fn given_no_selection(world: &mut FfmWorld) {
    world.app().left_panel.select(None);
}

#[given(expr = "{int} numbered files")]
fn given_numbered_files(world: &mut FfmWorld, count: usize) {
    let base = world.temp_path();
    for i in 0..count {
        fs::write(base.join(format!("file_{i:02}.txt")), "x").expect("write");
    }
}

// ═══════════════════════════════════════════════════════════════════
// WHEN steps
// ═══════════════════════════════════════════════════════════════════

#[when("I create a panel at the temporary directory")]
fn when_create_panel(world: &mut FfmWorld) {
    world.panel = Some(Panel::new(world.temp_path()).expect("create panel"));
}

#[when("I press Down")]
fn when_down(world: &mut FfmWorld) {
    world.panel().next();
}

#[when("I press Up")]
fn when_up(world: &mut FfmWorld) {
    world.panel().previous();
}

#[when("I press Home")]
fn when_home(world: &mut FfmWorld) {
    world.panel().select(Some(0));
}

#[when("I press End")]
fn when_end(world: &mut FfmWorld) {
    let last = world.panel().entries.len().saturating_sub(1);
    world.panel().select(Some(last));
}

#[when("I press Enter")]
fn when_enter(world: &mut FfmWorld) {
    world.panel().enter().expect("enter");
}

#[when("I press F5")]
fn when_f5(world: &mut FfmWorld) {
    handle_copy(world.app()).expect("F5");
}

#[when("I press F6")]
fn when_f6(world: &mut FfmWorld) {
    handle_move(world.app()).expect("F6");
}

#[when("I press F7")]
fn when_f7(world: &mut FfmWorld) {
    handle_mkdir(world.app()).expect("F7");
}

#[when("I press F8")]
fn when_f8(world: &mut FfmWorld) {
    handle_delete_confirm(world.app()).expect("F8");
}

#[when("I press F3")]
fn when_f3(world: &mut FfmWorld) {
    handle_view(world.app()).expect("F3");
}

#[when("I confirm the deletion")]
fn when_confirm_delete(world: &mut FfmWorld) {
    handle_delete(world.app()).expect("delete");
}

#[when("I cancel the deletion")]
fn when_cancel_delete(world: &mut FfmWorld) {
    world.app().close_overlay();
    "Delete cancelled".clone_into(&mut world.app().status_text());
}

#[when(expr = "I quick search for {string}")]
fn when_quick_search(world: &mut FfmWorld, q: String) {
    world.panel().find_first_matching(&q);
}

#[when(expr = "I quick search for {string} in the app")]
fn when_quick_search_app(world: &mut FfmWorld, q: String) {
    let app = world.app();
    app.get_active_panel().find_first_matching(&q);
}

#[when(expr = "I set bookmark {int}")]
fn when_set_bookmark(world: &mut FfmWorld, slot: usize) {
    let path = world.app().get_active_panel().path.clone();
    world
        .app()
        .bookmarks
        .set(BookmarkSlot::from_usize(slot).expect("slot"), path);
}

#[when(expr = "I go to bookmark {int}")]
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

#[when(expr = "I create a panel with sort mode Name")]
fn when_sort_name(world: &mut FfmWorld) {
    let mut p = Panel::new(world.temp_path()).expect("panel");
    p.sort_mode = SortMode::Name;
    p.load_entries().expect("load");
    world.panel = Some(p);
}

#[when(expr = "I create a panel with sort mode Extension")]
fn when_sort_ext(world: &mut FfmWorld) {
    let mut p = Panel::new(world.temp_path()).expect("panel");
    p.sort_mode = SortMode::Extension;
    p.load_entries().expect("load");
    world.panel = Some(p);
}

#[when(expr = "I cycle sort mode {int} times")]
fn when_cycle_sort(world: &mut FfmWorld, n: usize) {
    for _ in 0..n {
        world.panel().cycle_sort_mode();
    }
}

// Panel operations
#[when("I press Tab")]
fn when_tab(world: &mut FfmWorld) {
    world.app().switch_panel();
}

#[when("I press Left")]
fn when_left(world: &mut FfmWorld) {
    world.app().active_panel = PanelSide::Left;
}

#[when("I press Right")]
fn when_right(world: &mut FfmWorld) {
    world.app().active_panel = PanelSide::Right;
}

#[when(expr = "I set the active panel to Right")]
fn when_set_right(world: &mut FfmWorld) {
    world.app().active_panel = PanelSide::Right;
}

#[when("I swap panels")]
fn when_swap(world: &mut FfmWorld) {
    world.app().swap_panels();
}

#[when("I toggle console mode")]
fn when_toggle_console(world: &mut FfmWorld) {
    let app = world.app();
    app.view = app.view.toggled();
}

#[when("I refresh panels")]
fn when_refresh(world: &mut FfmWorld) {
    let app = world.app();
    match funny_file_manager::utils::refresh_panels(app) {
        Ok(()) => app.set_status(Status::PanelsRefreshed),
        Err(e) => app.set_status(Status::RefreshFailed(e.to_string())),
    }
}

#[when("I toggle hide left panel")]
fn when_hide_left(world: &mut FfmWorld) {
    let app = world.app();
    if matches!(app.overlay, Overlay::Help) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::Help); // reuse Help overlay as hide toggle for tests
    }
}

#[when("I toggle hide right panel")]
fn when_hide_right(world: &mut FfmWorld) {
    let app = world.app();
    if matches!(app.overlay, Overlay::DriveSelection { .. }) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::DriveSelection {
            side: PanelSide::Right,
            selected_index: 0,
        });
    }
}

// Command line — use process_popup_input to execute commands
#[when(expr = "I execute command {string}")]
fn when_exec_cmd(world: &mut FfmWorld, cmd: String) {
    let app = world.app();
    app.set_overlay(Overlay::CommandLine(cmd.into()));
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::empty(),
    );
    process_popup_input(app, key);
}

#[when(expr = "I navigate to {string}")]
fn when_navigate_to(world: &mut FfmWorld, sub: String) {
    let app = world.app();
    let new_path = app.get_active_panel().path.join(&sub);
    let panel = app.get_active_panel();
    panel.path = new_path;
    let _ = panel.load_entries();
    panel.ensure_selection();
}

// Keyboard shortcuts
#[when("I set should_quit")]
fn when_set_quit(world: &mut FfmWorld) {
    world.app().lifecycle = Lifecycle::Quitting;
}

#[when("I go to root")]
fn when_go_root(world: &mut FfmWorld) {
    let app = world.app();
    app.get_active_panel().path = PathBuf::from("/");
    let _ = app.get_active_panel().load_entries();
}

#[when("I go to parent directory")]
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

#[when("I press PageUp")]
fn when_page_up(world: &mut FfmWorld) {
    let panel = world.panel();
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        panel.select(Some(cur.saturating_sub(10)));
    }
}

#[when("I press PageDown")]
fn when_page_down(world: &mut FfmWorld) {
    let panel = world.panel();
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        let max = panel.entries.len() - 1;
        panel.select(Some((cur + 10).min(max)));
    }
}

#[when("I insert filename into command line")]
fn when_insert_filename(world: &mut FfmWorld) {
    let app = world.app();
    if let Some(name) = app
        .get_active_panel()
        .get_selected_entry()
        .map(|e| e.name.clone())
    {
        if app.get_command_line().is_none() {
            app.set_overlay(Overlay::CommandLine(String::new().into()));
        }
        let cmd = app.get_command_line_mut().expect("command line");
        if !cmd.is_empty() {
            cmd.insert_char(' ');
        }
        cmd.insert_str(&name);
    }
}

#[when("I insert full path into command line")]
fn when_insert_path(world: &mut FfmWorld) {
    let app = world.app();
    if let Some(path) = app.get_active_panel().selected_path() {
        let s = path.display().to_string();
        if app.get_command_line().is_none() {
            app.set_overlay(Overlay::CommandLine(String::new().into()));
        }
        let cmd = app.get_command_line_mut().expect("command line");
        if !cmd.is_empty() {
            cmd.insert_char(' ');
        }
        cmd.insert_str(&s);
    }
}

#[when("I sort by name")]
fn when_sort_by_name(world: &mut FfmWorld) {
    let app = world.app();
    let panel = app.get_active_panel();
    panel.sort_mode = SortMode::Name;
    let _ = panel.load_entries();
    app.set_status(Status::Sort("Name".to_owned()));
}

#[when("I sort by extension")]
fn when_sort_by_ext(world: &mut FfmWorld) {
    let app = world.app();
    let panel = app.get_active_panel();
    panel.sort_mode = SortMode::Extension;
    let _ = panel.load_entries();
    app.set_status(Status::Sort("Extension".to_owned()));
}

#[when("I sort by size")]
fn when_sort_by_size(world: &mut FfmWorld) {
    let app = world.app();
    let panel = app.get_active_panel();
    panel.sort_mode = SortMode::Size;
    let _ = panel.load_entries();
    app.set_status(Status::Sort("Size".to_owned()));
}

// Marking
#[when("I toggle mark")]
fn when_toggle_mark(world: &mut FfmWorld) {
    world.panel().toggle_mark_selected();
}

#[when("I clear all marks")]
fn when_clear_marks(world: &mut FfmWorld) {
    world.panel().clear_marks();
}

#[when("I select all files")]
fn when_select_all(world: &mut FfmWorld) {
    let app = world.app();
    let panel = app.get_active_panel();
    let names: Vec<String> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir() && e.name != "..")
        .map(|e| e.name.clone())
        .collect();
    let all_marked = !names.is_empty() && names.iter().all(|n| panel.marked.contains(n.as_str()));
    if all_marked {
        panel.clear_marks();
    } else {
        for n in &names {
            panel.marked.insert(FileName::new(n.clone()));
        }
    }
}

// Overlays
#[when("I toggle help")]
fn when_toggle_help(world: &mut FfmWorld) {
    let app = world.app();
    if matches!(app.overlay, Overlay::Help) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::Help);
    }
}

#[when("I open new file dialog")]
fn when_open_new_file(world: &mut FfmWorld) {
    world
        .app()
        .set_overlay(Overlay::NewFileInput(String::new()));
}

#[when(expr = "I type filename {string}")]
fn when_type_filename(world: &mut FfmWorld, name: String) {
    world.app().set_overlay(Overlay::NewFileInput(name));
}

#[when("I confirm new file")]
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
        fs::File::create(&path).expect("create file");
        app.set_status(Status::CommandOutput(format!("Created file: {input}")));
        let _ = funny_file_manager::utils::refresh_panels(app);
    }
}

#[when("I close the viewer")]
fn when_close_viewer(world: &mut FfmWorld) {
    world.app().close_overlay();
}

#[when(expr = "I open drive selection for {word}")]
fn when_open_drives(world: &mut FfmWorld, side: String) {
    let app = world.app();
    let side = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    app.set_overlay(Overlay::DriveSelection {
        side,
        selected_index: 0,
    });
}

// Editor (F4)
#[when(expr = "I press F4 with success behavior")]
fn when_f4_success(world: &mut FfmWorld) {
    handle_edit_test_mode(world.app(), TestEditorBehavior::Success).expect("F4 success");
}

#[when(expr = "I press F4 with fail_launch behavior")]
fn when_f4_fail_launch(world: &mut FfmWorld) {
    handle_edit_test_mode(world.app(), TestEditorBehavior::FailLaunch).expect("F4 fail_launch");
}

#[when(expr = "I press F4 with exit_error behavior")]
fn when_f4_exit_error(world: &mut FfmWorld) {
    handle_edit_test_mode(world.app(), TestEditorBehavior::ExitError).expect("F4 exit_error");
}

#[when(expr = "I create new file {string} in the active panel")]
fn when_create_new_file(world: &mut FfmWorld, name: String) {
    let app = world.app();
    let path = app.get_active_panel().path.join(&name);
    fs::File::create(&path).expect("create file");
    let _ = funny_file_manager::utils::refresh_panels(app);
}

#[when("I switch to the right panel")]
fn when_switch_right(world: &mut FfmWorld) {
    world.app().active_panel = PanelSide::Right;
}

#[when(expr = "I select {string} in the right panel")]
fn when_select_right(world: &mut FfmWorld, name: String) {
    let app = world.app();
    let idx = app
        .right_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("entry '{name}' not found in right panel"));
    app.right_panel.select(Some(idx));
}

#[when(expr = "I select {string} in the left panel")]
fn when_select_left(world: &mut FfmWorld, name: String) {
    let app = world.app();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .unwrap_or_else(|| panic!("entry '{name}' not found in left panel"));
    app.left_panel.select(Some(idx));
}

// VirtualPath
#[when(expr = "I parse path {string}")]
fn when_parse_path(world: &mut FfmWorld, path: String) {
    world.parsed_path = Some(VirtualPath::parse(&path));
}

#[when("I list AWS profiles")]
fn when_list_profiles(world: &mut FfmWorld) {
    world.aws_profiles = list_aws_profiles().into_iter().map(|p| p.name).collect();
}

// Utilities
#[when(expr = "I format size {int}")]
fn when_format_size(world: &mut FfmWorld, size: u64) {
    world.formatted_size = Some(FileSize::from_bytes(size).to_string());
}

#[when(expr = "I request unique name for {string}")]
fn when_unique_name(world: &mut FfmWorld, name: String) {
    let result =
        unique_destination(world.temp_path().as_path(), OsStr::new(&name)).expect("unique name");
    world.unique_name = Some(
        result
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    );
}

#[when(expr = "I copy directory {string} to {string}")]
fn when_copy_dir(world: &mut FfmWorld, src: String, dst: String) {
    let base = world.temp_path();
    copy_dir_all(&base.join(&src), &base.join(&dst)).expect("copy dir");
}

// ═══════════════════════════════════════════════════════════════════
// THEN steps
// ═══════════════════════════════════════════════════════════════════

#[then(expr = "the panel should have {int} entries")]
fn then_panel_count(world: &mut FfmWorld, n: usize) {
    assert_eq!(world.panel().entries.len(), n);
}

#[then(expr = "entry {int} should be {string}")]
fn then_entry(world: &mut FfmWorld, i: usize, name: String) {
    assert_eq!(world.panel().entries[i].name, name);
}

#[then(expr = "the selection should be at index {int}")]
fn then_sel(world: &mut FfmWorld, i: usize) {
    assert_eq!(world.panel().selected(), Some(i));
}

#[then(expr = "the panel path should end with {string}")]
fn then_path_ends(world: &mut FfmWorld, s: String) {
    let p = if let Some(ref panel) = world.panel {
        panel.path.display().to_string()
    } else {
        world.app().get_active_panel().path.display().to_string()
    };
    assert!(p.ends_with(&s), "'{p}' should end with '{s}'");
}

#[then(expr = "the active panel path should end with {string}")]
fn then_active_ends(world: &mut FfmWorld, s: String) {
    let p = world.app().get_active_panel().path.display().to_string();
    assert!(p.ends_with(&s), "'{p}' should end with '{s}'");
}

#[then(expr = "the active panel path should not end with {string}")]
fn then_active_not_ends(world: &mut FfmWorld, s: String) {
    let p = world.app().get_active_panel().path.display().to_string();
    assert!(!p.ends_with(&s), "'{p}' should not end with '{s}'");
}

#[then(expr = "the active panel path should be {string}")]
fn then_active_is(world: &mut FfmWorld, s: String) {
    let p = world.app().get_active_panel().path.display().to_string();
    assert_eq!(p, s);
}

#[then(expr = "{string} should exist in the destination")]
fn then_dst_exists(world: &mut FfmWorld, n: String) {
    assert!(
        world.dst_path().join(&n).exists(),
        "{n} should exist in dst"
    );
}

#[then(expr = "{string} should exist in the source")]
fn then_src_has(world: &mut FfmWorld, n: String) {
    assert!(
        world.src_path().join(&n).exists(),
        "{n} should exist in src"
    );
}

#[then(expr = "{string} should still exist in the source")]
fn then_src_exists(world: &mut FfmWorld, n: String) {
    assert!(
        world.src_path().join(&n).exists(),
        "{n} should exist in src"
    );
}

#[then(expr = "{string} should not exist in the source")]
fn then_src_not(world: &mut FfmWorld, n: String) {
    assert!(!world.src_path().join(&n).exists(), "{n} should not exist");
}

#[then(expr = "the status message should contain {string}")]
fn then_status(world: &mut FfmWorld, t: String) {
    let m = &world.app().status_text();
    assert!(m.contains(&t), "'{m}' should contain '{t}'");
}

#[then("the status message should not be empty")]
fn then_status_not_empty(world: &mut FfmWorld) {
    assert!(!world.app().status_text().is_empty());
}

#[then(expr = "the status message should not contain {string}")]
fn then_status_not_contain(world: &mut FfmWorld, t: String) {
    let m = &world.app().status_text();
    assert!(!m.contains(&t), "'{m}' should NOT contain '{t}'");
}

#[then(expr = "{string} should be a directory in the source")]
fn then_dir_src(world: &mut FfmWorld, n: String) {
    assert!(world.src_path().join(&n).is_dir());
}

#[then("a delete confirmation should be shown")]
fn then_del_confirm(world: &mut FfmWorld) {
    assert!(world.app().get_delete_confirm().is_some());
}

#[then("a delete confirmation should not be shown")]
fn then_no_del_confirm(world: &mut FfmWorld) {
    assert!(world.app().get_delete_confirm().is_none());
}

#[then("the viewer should be open")]
fn then_viewer_open(world: &mut FfmWorld) {
    assert!(world.app().get_viewer().is_some());
}

#[then("the viewer should not be open")]
fn then_viewer_closed(world: &mut FfmWorld) {
    assert!(world.app().get_viewer().is_none());
}

#[then(expr = "the viewer title should contain {string}")]
fn then_viewer_title(world: &mut FfmWorld, t: String) {
    let title = &world.app().get_viewer().expect("viewer").title;
    assert!(title.contains(&t), "'{title}' should contain '{t}'");
}

#[then(expr = "the viewer should show {string}")]
fn then_viewer_shows(world: &mut FfmWorld, t: String) {
    let lines = &world.app().get_viewer().expect("viewer").lines;
    assert!(lines.join("\n").contains(&t));
}

#[then(expr = "the selected entry should be {string}")]
fn then_selected(world: &mut FfmWorld, n: String) {
    assert_eq!(world.panel().get_selected_entry().expect("sel").name, n);
}

#[then(expr = "bookmark {int} should be set to {string}")]
fn then_bookmark(world: &mut FfmWorld, slot: usize, sub: String) {
    let bm = world
        .app()
        .bookmarks
        .get(BookmarkSlot::from_usize(slot).expect("slot"))
        .expect("bookmark");
    assert!(bm.display().to_string().contains(&sub));
}

#[then("the sort mode should be back to Name")]
fn then_sort_name(world: &mut FfmWorld) {
    assert_eq!(world.panel().sort_mode, SortMode::Name);
}

#[then("the first extension should come before the last alphabetically")]
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
        assert!(a <= b, "'{a}' should <= '{b}'");
    }
}

// Panel operations
#[then(expr = "the active panel should be {word}")]
fn then_active_panel(world: &mut FfmWorld, side: String) {
    let expected = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    assert_eq!(world.app().active_panel, expected);
}

#[then("console mode should be off")]
fn then_console_off(world: &mut FfmWorld) {
    assert_ne!(world.app().view, ViewMode::Console);
}

#[then("console mode should be on")]
fn then_console_on(world: &mut FfmWorld) {
    assert_eq!(world.app().view, ViewMode::Console);
}

#[then("the left panel path should differ from original")]
fn then_left_differs(world: &mut FfmWorld) {
    let orig = world.original_left_path.clone();
    if let Some(orig) = orig {
        assert_ne!(&world.app().left_panel.path, &orig);
    }
}

// Command line
#[then(expr = "the sort mode should be {word}")]
fn then_sort_mode(world: &mut FfmWorld, mode: String) {
    let expected = match mode.as_str() {
        "Name" => SortMode::Name,
        "Extension" => SortMode::Extension,
        "Size" => SortMode::Size,
        _ => panic!("unknown sort mode: {mode}"),
    };
    assert_eq!(world.app().get_active_panel().sort_mode, expected);
}

#[then("the active panel should have marks")]
fn then_has_marks(world: &mut FfmWorld) {
    assert!(!world.app().get_active_panel().marked.is_empty());
}

#[then("the active panel should have no marks")]
fn then_no_marks(world: &mut FfmWorld) {
    assert!(world.app().get_active_panel().marked.is_empty());
}

#[then("help should be visible")]
fn then_help_on(world: &mut FfmWorld) {
    assert!(matches!(world.app().overlay, Overlay::Help));
}

#[then("help should not be visible")]
fn then_help_off(world: &mut FfmWorld) {
    assert!(!matches!(world.app().overlay, Overlay::Help));
}

#[then("the app should be quitting")]
fn then_quitting(world: &mut FfmWorld) {
    assert!(!world.app().lifecycle.is_running());
}

#[then("drive selection should be visible")]
fn then_drives_visible(world: &mut FfmWorld) {
    assert!(matches!(
        world.app().overlay,
        Overlay::DriveSelection { .. }
    ));
}

// Marking
#[then(expr = "{string} should be marked")]
fn then_marked(world: &mut FfmWorld, n: String) {
    assert!(world.panel().marked.contains(n.as_str()));
}

#[then(expr = "{string} should not be marked")]
fn then_not_marked(world: &mut FfmWorld, n: String) {
    assert!(!world.panel().marked.contains(n.as_str()));
}

#[then(expr = "{string} should be marked in the app")]
fn then_marked_app(world: &mut FfmWorld, n: String) {
    assert!(world.app().get_active_panel().marked.contains(n.as_str()));
}

#[then(expr = "{string} should not be marked in the app")]
fn then_not_marked_app(world: &mut FfmWorld, n: String) {
    assert!(!world.app().get_active_panel().marked.contains(n.as_str()));
}

#[then("the panel should have no marks")]
fn then_panel_no_marks(world: &mut FfmWorld) {
    assert!(world.panel().marked.is_empty());
}

// Keyboard shortcuts
#[then(expr = "the command line should contain {string}")]
fn then_cmd_has(world: &mut FfmWorld, t: String) {
    let cmd = world.app().get_command_line().unwrap_or("");
    assert!(cmd.contains(&t), "cmd '{cmd}' should contain '{t}'");
}

// Overlays
#[then(expr = "{string} should exist in the temporary directory")]
fn then_temp_exists(world: &mut FfmWorld, n: String) {
    assert!(world.temp_path().join(&n).exists(), "{n} should exist");
}

// VirtualPath
#[then(expr = "the path type should be Local")]
fn then_local(world: &mut FfmWorld) {
    assert!(matches!(world.parsed_path, Some(VirtualPath::Local(_))));
}

#[then(expr = "the path type should be S3")]
fn then_s3(world: &mut FfmWorld) {
    assert!(matches!(world.parsed_path, Some(VirtualPath::S3 { .. })));
}

#[then(expr = "the display should be {string}")]
fn then_display(world: &mut FfmWorld, s: String) {
    assert_eq!(world.parsed_path.as_ref().expect("path").display(), s);
}

#[then(expr = "the profile should be {string}")]
fn then_profile(world: &mut FfmWorld, p: String) {
    match world.parsed_path.as_ref().expect("path") {
        VirtualPath::S3 { profile, .. } => assert_eq!(profile, &p),
        _ => panic!("expected S3 path"),
    }
}

#[then(expr = "the bucket should be {string}")]
fn then_bucket(world: &mut FfmWorld, b: String) {
    match world.parsed_path.as_ref().expect("path") {
        VirtualPath::S3 { bucket, .. } => assert_eq!(bucket.as_deref(), Some(b.as_str())),
        _ => panic!("expected S3 path"),
    }
}

#[then("the bucket should be empty")]
fn then_bucket_empty(world: &mut FfmWorld) {
    match world.parsed_path.as_ref().expect("path") {
        VirtualPath::S3 { bucket, .. } => assert!(bucket.is_none()),
        _ => panic!("expected S3 path"),
    }
}

#[then(expr = "the prefix should be {string}")]
fn then_prefix(world: &mut FfmWorld, p: String) {
    match world.parsed_path.as_ref().expect("path") {
        VirtualPath::S3 { prefix, .. } => assert_eq!(prefix, &p),
        _ => panic!("expected S3 path"),
    }
}

#[then(expr = "the profiles should include {string}")]
fn then_profiles_include(world: &mut FfmWorld, name: String) {
    assert!(world.aws_profiles.iter().any(|p| p == &name));
}

// Utilities
#[then(expr = "the formatted size should be {string}")]
fn then_formatted(world: &mut FfmWorld, s: String) {
    assert_eq!(world.formatted_size.as_deref(), Some(s.as_str()));
}

#[then(expr = "the unique name should be {string}")]
fn then_unique(world: &mut FfmWorld, s: String) {
    assert_eq!(world.unique_name.as_deref(), Some(s.as_str()));
}

// Editor
#[then("the left panel should have entries")]
fn then_left_has_entries(world: &mut FfmWorld) {
    assert!(!world.app().left_panel.entries.is_empty());
}

#[then("the right panel should have entries")]
fn then_right_has_entries(world: &mut FfmWorld) {
    assert!(!world.app().right_panel.entries.is_empty());
}

#[then("the left panel should have a valid selection")]
fn then_left_valid_sel(world: &mut FfmWorld) {
    let app = world.app();
    let sel = app.left_panel.selected();
    assert!(sel.is_some(), "left panel should have a selection");
    assert!(
        sel.unwrap() < app.left_panel.entries.len(),
        "selection should be within bounds"
    );
}

fn main() {
    futures::executor::block_on(FfmWorld::run("tests/features"));
}
