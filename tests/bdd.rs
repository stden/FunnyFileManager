//! Purpose: Maps English Cucumber scenarios to `FunnyFileManager` test steps.
//!
#![expect(
    clippy::future_not_send,
    reason = "Cucumber generates a single-threaded world constructor owning the UI clipboard adapter"
)]
#![expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates every step's signature: captured args arrive owned and the world as `&mut`"
)]
use bdd_runner::{Context as _, StepResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use cucumber::{given, then, when, World as _};
use funny_file_manager::description;
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
use funny_file_manager::models::{
    App, NamePrompt, Overlay, Panel, PanelSide, PanelsShown, SortMode,
};
use funny_file_manager::ops::panels;
use funny_file_manager::s3::{list_aws_profiles, VirtualPath};
use funny_file_manager::status::Status;
use funny_file_manager::ui::ui;
use funny_file_manager::utils::{copy_dir_all, unique_destination};
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
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
    // For F9 Commands > Install MCP server scenarios
    mcp: Option<mcp_install::McpScenario>,
}

impl std::fmt::Debug for FfmWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfmWorld")
            .field("temp_dir", &self.temp_dir.as_ref().map(TempDir::path))
            .field("src_dir", &self.src_dir.as_ref().map(TempDir::path))
            .field("dst_dir", &self.dst_dir.as_ref().map(TempDir::path))
            .field("has_app", &self.app.is_some())
            .field("has_panel", &self.panel.is_some())
            .field("parsed_path", &self.parsed_path)
            .field("formatted_size", &self.formatted_size)
            .field("unique_name", &self.unique_name)
            .field("aws_profiles", &self.aws_profiles)
            .field("original_left_path", &self.original_left_path)
            .field("mcp", &self.mcp)
            .finish()
    }
}

impl FfmWorld {
    fn temp_path(&self) -> StepResult<PathBuf> {
        Ok(self
            .temp_dir
            .as_ref()
            .context("no temp dir")?
            .path()
            .to_path_buf())
    }
    fn src_path(&self) -> StepResult<PathBuf> {
        Ok(self
            .src_dir
            .as_ref()
            .context("no src dir")?
            .path()
            .to_path_buf())
    }
    fn dst_path(&self) -> StepResult<PathBuf> {
        Ok(self
            .dst_dir
            .as_ref()
            .context("no dst dir")?
            .path()
            .to_path_buf())
    }
    fn app(&mut self) -> StepResult<&mut App> {
        self.app.as_mut().context("no app")
    }

    /// The last path segment of the source or destination directory: what a
    /// panel title shows, and unique because the directories are temporary.
    fn directory_name(&self, which: &str) -> StepResult<String> {
        let path = match which {
            "source" => self.src_path()?,
            _ => self.dst_path()?,
        };
        Ok(path
            .file_name()
            .context("temp dir name")?
            .to_string_lossy()
            .into_owned())
    }

    /// One 80x24 frame of the program as the user would see it.
    fn screen_text(&mut self) -> StepResult<String> {
        let mut terminal =
            Terminal::new(WritableTestBackend::new(80, 24)).context("test terminal")?;
        let app = self.app()?;
        terminal.draw(|frame| ui(frame, app)).context("draw")?;
        let buffer = terminal.backend().buffer();
        let rows = buffer.area.rows().map(|row| {
            row.columns()
                .map(|cell| buffer[(cell.x, cell.y)].symbol())
                .collect::<String>()
        });
        Ok(rows.collect::<Vec<_>>().join("\n"))
    }
    fn panel(&mut self) -> StepResult<&mut Panel> {
        self.panel.as_mut().context("no panel")
    }

    fn send_popup_key(&mut self, code: KeyCode) -> StepResult {
        let consumed = process_popup_input(self.app()?, KeyEvent::new(code, KeyModifiers::empty()));
        assert!(consumed, "the open dialog should consume {code:?}");
        Ok(())
    }

    fn type_in_popup(&mut self, text: &str) -> StepResult {
        for ch in text.chars() {
            self.send_popup_key(KeyCode::Char(ch))?;
        }
        Ok(())
    }

    fn run_key_workflow(&self, events: Vec<Event>, description: &str) -> StepResult {
        let app = self.app.as_ref().context("no app")?.clone();
        let mut terminal =
            Terminal::new(WritableTestBackend::new(80, 24)).context("test terminal")?;
        let mut events = events.into_iter();

        run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "BDD event stream exhausted")
            })
        })
        .map_err(|error| format!("{description}: {error}"))
    }
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

fn push_text(events: &mut Vec<Event>, text: &str) {
    events.extend(text.chars().map(|ch| key_event(KeyCode::Char(ch))));
}

fn push_make_folder(events: &mut Vec<Event>, name: &str, multiple_names: bool) {
    events.push(key_event(KeyCode::F(7)));
    push_text(events, name);
    if multiple_names {
        events.push(key_event(KeyCode::Down));
        events.push(key_event(KeyCode::Down));
        events.push(key_event(KeyCode::Char(' ')));
    }
    events.push(key_event(KeyCode::Enter));
}

fn push_quit(events: &mut Vec<Event>) {
    events.push(Event::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    )));
}

fn push_transfer_with_policy(
    events: &mut Vec<Event>,
    function_key: u8,
    policy: &str,
) -> StepResult {
    let choice_steps = match policy {
        "Ask" => 0,
        "Overwrite" => 1,
        "Skip" => 2,
        "Only newer files" => 3,
        other => return Err(format!("unsupported existing-file policy: {other}")),
    };

    events.push(key_event(KeyCode::F(function_key)));
    if choice_steps > 0 {
        // Destination -> access rights -> existing-file policy.
        events.push(key_event(KeyCode::Tab));
        events.push(key_event(KeyCode::Tab));
        events.extend((0..choice_steps).map(|_| key_event(KeyCode::Right)));
    }
    events.push(key_event(KeyCode::Enter));
    Ok(())
}

fn set_file_age(path: &Path, age: SystemTime) -> StepResult {
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|error| format!("open {} to set its timestamp: {error}", path.display()))?
        .set_modified(age)
        .map_err(|error| format!("set timestamp on {}: {error}", path.display()))
}

fn arrange_source_age(world: &FfmWorld, name: &str, source_is_newer: bool) -> StepResult {
    let source = world.src_path()?.join(name);
    let destination = world.dst_path()?.join(name);
    assert!(
        source.is_file(),
        "missing source fixture: {}",
        source.display()
    );
    assert!(
        destination.is_file(),
        "missing destination fixture: {}",
        destination.display()
    );

    let earlier = SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    let later = earlier + Duration::from_hours(24);
    let (source_age, destination_age) = if source_is_newer {
        (later, earlier)
    } else {
        (earlier, later)
    };
    set_file_age(&source, source_age)?;
    set_file_age(&destination, destination_age)?;

    assert_timestamp_order(&source, &destination, source_is_newer)
}

// ═══════════════════════════════════════════════════════════════════
// GIVEN steps
// ═══════════════════════════════════════════════════════════════════

#[given("a temporary directory")]
fn given_temp_dir(world: &mut FfmWorld) -> StepResult {
    world.temp_dir = Some(TempDir::new().context("create temp dir")?);
    Ok(())
}

#[given("a source directory")]
fn given_src_dir(world: &mut FfmWorld) -> StepResult {
    world.src_dir = Some(TempDir::new().context("create src dir")?);
    Ok(())
}

#[given("an empty destination directory")]
fn given_empty_dst_dir(world: &mut FfmWorld) -> StepResult {
    world.dst_dir = Some(TempDir::new().context("create dst dir")?);
    Ok(())
}

#[given(expr = "a file {string} with content {string}")]
fn given_file(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    let base = world.temp_path()?;
    let path = base.join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create parent dir")?;
    }
    fs::write(path, &content).context("write file")?;
    Ok(())
}

#[given(expr = "a file {string} with content {string} in the source")]
fn given_file_in_src(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    fs::write(world.src_path()?.join(&name), &content).context("write file")?;
    Ok(())
}

#[given(expr = "a file {string} with content {string} in the destination")]
fn given_file_in_dst(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    fs::write(world.dst_path()?.join(&name), &content).context("write dst file")?;
    Ok(())
}

#[given(expr = "a file {string} with content {string} in the source dir")]
fn given_file_in_src_subpath(world: &mut FfmWorld, name: String, content: String) -> StepResult {
    let path = world.src_path()?.join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create parent dir")?;
    }
    fs::write(path, &content).context("write file")?;
    Ok(())
}

#[given(expr = "a subdirectory {string} with file {string}")]
fn given_subdir_with_file(world: &mut FfmWorld, dir: String, file: String) -> StepResult {
    let sub = world.temp_path()?.join(&dir);
    fs::create_dir_all(&sub).context("create subdir")?;
    fs::write(sub.join(&file), "content").context("write file")?;
    Ok(())
}

#[given(expr = "a subdirectory {string} in the source")]
fn given_subdir_in_src(world: &mut FfmWorld, dir: String) -> StepResult {
    fs::create_dir_all(world.src_path()?.join(&dir)).context("create subdir")?;
    Ok(())
}

#[given(expr = "a subdirectory {string} in the source with file {string}")]
fn given_subdir_in_src_with_file(world: &mut FfmWorld, dir: String, file: String) -> StepResult {
    let sub = world.src_path()?.join(&dir);
    fs::create_dir_all(&sub).context("create subdir")?;
    fs::write(sub.join(&file), "content").context("write file")?;
    Ok(())
}

#[given(expr = "the source copy of {string} is older than the destination copy")]
fn given_source_copy_is_older(world: &mut FfmWorld, name: String) -> StepResult {
    arrange_source_age(world, &name, false)
}

#[given(expr = "the source copy of {string} is newer than the destination copy")]
fn given_source_copy_is_newer(world: &mut FfmWorld, name: String) -> StepResult {
    arrange_source_age(world, &name, true)
}

#[given("a panel at the temporary directory")]
fn given_panel_at_temp(world: &mut FfmWorld) -> StepResult {
    world.panel = Some(Panel::new(world.temp_path()?).context("create panel")?);
    Ok(())
}

#[given(expr = "a panel at {string} in the temporary directory")]
fn given_panel_at_sub(world: &mut FfmWorld, sub: String) -> StepResult {
    let path = world.temp_path()?.join(&sub);
    world.panel = Some(Panel::new(path).context("create panel")?);
    Ok(())
}

#[given(expr = "the selection is at index {int}")]
fn given_selection_at(world: &mut FfmWorld, index: usize) -> StepResult {
    world.panel()?.select(Some(index));
    Ok(())
}

#[given(expr = "I select the entry {string}")]
fn given_select_entry(world: &mut FfmWorld, name: String) -> StepResult {
    let panel = world.panel()?;
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found"))?;
    panel.select(Some(idx));
    Ok(())
}

#[given("the app is set up with source and destination")]
fn given_app_src_dst(world: &mut FfmWorld) -> StepResult {
    let mut app = App::new().context("create app")?;
    app.left_panel = Panel::new(world.src_path()?).context("left panel")?;
    app.right_panel = Panel::new(world.dst_path()?).context("right panel")?;
    app.active_panel = PanelSide::Left;
    world.original_left_path = Some(app.left_panel.path.clone());
    world.app = Some(app);
    Ok(())
}

#[given(expr = "the app is set up at {string} in the source")]
fn given_app_at_src_sub(world: &mut FfmWorld, sub: String) -> StepResult {
    let src_sub = world.src_path()?.join(&sub);
    let dst = world.dst_path()?;
    let mut app = App::new().context("create app")?;
    app.left_panel = Panel::new(src_sub).context("left panel")?;
    app.right_panel = Panel::new(dst).context("right panel")?;
    app.active_panel = PanelSide::Left;
    world.original_left_path = Some(app.left_panel.path.clone());
    world.app = Some(app);
    Ok(())
}

#[given(expr = "an app at {string} in the temporary directory")]
fn given_app_at_sub(world: &mut FfmWorld, sub: String) -> StepResult {
    let path = world.temp_path()?.join(&sub);
    let mut app = App::new().context("create app")?;
    app.left_panel = Panel::new(path).context("panel")?;
    app.right_panel = Panel::new(world.temp_path()?).context("right panel")?;
    world.app = Some(app);
    Ok(())
}

#[given("an app at the temporary directory")]
fn given_app_at_temp(world: &mut FfmWorld) -> StepResult {
    let path = world.temp_path()?;
    let mut app = App::new().context("create app")?;
    app.left_panel = Panel::new(path.clone()).context("left panel")?;
    app.right_panel = Panel::new(path).context("right panel")?;
    world.app = Some(app);
    Ok(())
}

#[given(expr = "I select {string} in the left panel")]
fn given_select_left(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found in left panel"))?;
    app.left_panel.select(Some(idx));
    Ok(())
}

#[given(expr = "I select {string} in the active panel")]
fn given_select_active(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .get_active_panel()
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found"))?;
    app.get_active_panel().select(Some(idx));
    Ok(())
}

#[given(expr = "I select {string} in the right panel")]
fn given_select_right(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .right_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found in right panel"))?;
    app.right_panel.select(Some(idx));
    Ok(())
}

#[given(expr = "I mark {string} in the left panel")]
#[when(expr = "I mark {string} in the left panel")]
fn given_mark_left(world: &mut FfmWorld, name: String) -> StepResult {
    world.app()?.left_panel.marked.insert(FileName::new(name));
    Ok(())
}

#[given(expr = "the file {string} is marked")]
fn given_file_marked(world: &mut FfmWorld, name: String) -> StepResult {
    world
        .app()?
        .get_active_panel()
        .marked
        .insert(FileName::new(name));
    Ok(())
}

#[given(expr = "{string} is marked")]
fn given_marked(world: &mut FfmWorld, name: String) -> StepResult {
    world.panel()?.marked.insert(FileName::new(name));
    Ok(())
}

#[given(expr = "bookmark {int} is empty")]
fn given_bookmark_empty(world: &mut FfmWorld, slot: usize) -> StepResult {
    world
        .app()?
        .bookmarks
        .clear(BookmarkSlot::from_usize(slot).context("slot")?);
    Ok(())
}

#[given("the left panel has no selection")]
fn given_no_selection(world: &mut FfmWorld) -> StepResult {
    world.app()?.left_panel.select(None);
    Ok(())
}

#[given(expr = "{int} numbered files")]
fn given_numbered_files(world: &mut FfmWorld, count: usize) -> StepResult {
    let base = world.temp_path()?;
    for i in 0..count {
        fs::write(base.join(format!("file_{i:02}.txt")), "x").context("write")?;
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// WHEN steps
// ═══════════════════════════════════════════════════════════════════

#[when("I create a panel at the temporary directory")]
fn when_create_panel(world: &mut FfmWorld) -> StepResult {
    world.panel = Some(Panel::new(world.temp_path()?).context("create panel")?);
    Ok(())
}

#[when("I press Down")]
fn when_down(world: &mut FfmWorld) -> StepResult {
    world.panel()?.next();
    Ok(())
}

#[when("I press Up")]
fn when_up(world: &mut FfmWorld) -> StepResult {
    world.panel()?.previous();
    Ok(())
}

#[when("I press Home")]
fn when_home(world: &mut FfmWorld) -> StepResult {
    world.panel()?.select(Some(0));
    Ok(())
}

#[when("I press End")]
fn when_end(world: &mut FfmWorld) -> StepResult {
    let last = world.panel()?.entries.len().saturating_sub(1);
    world.panel()?.select(Some(last));
    Ok(())
}

#[when("I press Enter")]
fn when_enter(world: &mut FfmWorld) -> StepResult {
    world.panel()?.enter().context("enter")?;
    Ok(())
}

#[when("I press F5")]
fn when_f5(world: &mut FfmWorld) -> StepResult {
    handle_copy(world.app()?).context("F5")?;
    Ok(())
}

#[when("I press F6")]
fn when_f6(world: &mut FfmWorld) -> StepResult {
    handle_move(world.app()?).context("F6")?;
    Ok(())
}

#[when("I press F7")]
fn when_f7(world: &mut FfmWorld) -> StepResult {
    handle_mkdir(world.app()?).context("F7")?;
    Ok(())
}

#[when("I press F8")]
fn when_f8(world: &mut FfmWorld) -> StepResult {
    handle_delete_confirm(world.app()?).context("F8")?;
    Ok(())
}

#[when("I press F3")]
fn when_f3(world: &mut FfmWorld) -> StepResult {
    handle_view(world.app()?).context("F3")?;
    Ok(())
}

#[when("I confirm the deletion")]
fn when_confirm_delete(world: &mut FfmWorld) -> StepResult {
    handle_delete(world.app()?).context("delete")?;
    Ok(())
}

#[when("I cancel the deletion")]
fn when_cancel_delete(world: &mut FfmWorld) -> StepResult {
    world.send_popup_key(KeyCode::Esc)
}

#[when(expr = "I quick search for {string}")]
fn when_quick_search(world: &mut FfmWorld, q: String) -> StepResult {
    world.panel()?.find_first_matching(&q);
    Ok(())
}

#[when(expr = "I quick search for {string} in the app")]
fn when_quick_search_app(world: &mut FfmWorld, q: String) -> StepResult {
    let app = world.app()?;
    app.get_active_panel().find_first_matching(&q);
    Ok(())
}

#[when(expr = "I set bookmark {int}")]
fn when_set_bookmark(world: &mut FfmWorld, slot: usize) -> StepResult {
    let path = world.app()?.get_active_panel().path.clone();
    world
        .app()?
        .bookmarks
        .set(BookmarkSlot::from_usize(slot).context("slot")?, path);
    Ok(())
}

#[when(expr = "I go to bookmark {int}")]
fn when_go_bookmark(world: &mut FfmWorld, slot: usize) -> StepResult {
    let app = world.app()?;
    if let Some(path) = app
        .bookmarks
        .get(BookmarkSlot::from_usize(slot).context("slot")?)
        .map(Path::to_path_buf)
    {
        let panel = app.get_active_panel();
        panel.path = path;
        panel.load_entries().context("reload the panel")?;
        panel.ensure_selection();
    } else {
        app.set_status(Status::CommandOutput(format!("Bookmark {slot} is empty")));
    }
    Ok(())
}

#[when(expr = "I create a panel with sort mode Name")]
fn when_sort_name(world: &mut FfmWorld) -> StepResult {
    let mut p = Panel::new(world.temp_path()?).context("panel")?;
    p.sort_mode = SortMode::Name;
    p.load_entries().context("load")?;
    world.panel = Some(p);
    Ok(())
}

#[when(expr = "I create a panel with sort mode Extension")]
fn when_sort_ext(world: &mut FfmWorld) -> StepResult {
    let mut p = Panel::new(world.temp_path()?).context("panel")?;
    p.sort_mode = SortMode::Extension;
    p.load_entries().context("load")?;
    world.panel = Some(p);
    Ok(())
}

#[when(expr = "I cycle sort mode {int} times")]
fn when_cycle_sort(world: &mut FfmWorld, n: usize) -> StepResult {
    for _ in 0..n {
        world.panel()?.cycle_sort_mode();
    }
    Ok(())
}

// Panel operations. These call what the keys call, so the guard that keeps
// the cursor off a hidden panel (Ctrl+F1 / Ctrl+F2) is the one under test.
#[when("I press Tab")]
fn when_tab(world: &mut FfmWorld) -> StepResult {
    panels::switch_panel(world.app()?);
    Ok(())
}

#[when("I press Left")]
fn when_left(world: &mut FfmWorld) -> StepResult {
    panels::select(world.app()?, PanelSide::Left);
    Ok(())
}

#[when("I press Right")]
fn when_right(world: &mut FfmWorld) -> StepResult {
    panels::select(world.app()?, PanelSide::Right);
    Ok(())
}

#[when("I press Ctrl+F1")]
fn when_ctrl_f1(world: &mut FfmWorld) -> StepResult {
    panels::toggle(world.app()?, PanelSide::Left);
    Ok(())
}

#[when("I press Ctrl+F2")]
fn when_ctrl_f2(world: &mut FfmWorld) -> StepResult {
    panels::toggle(world.app()?, PanelSide::Right);
    Ok(())
}

#[when(expr = "I set the active panel to Right")]
fn when_set_right(world: &mut FfmWorld) -> StepResult {
    world.app()?.active_panel = PanelSide::Right;
    Ok(())
}

#[when("I swap panels")]
fn when_swap(world: &mut FfmWorld) -> StepResult {
    panels::swap_panels(world.app()?);
    Ok(())
}

#[when("I toggle console mode")]
fn when_toggle_console(world: &mut FfmWorld) -> StepResult {
    // Call what `Ctrl+O` calls. Re-implementing the toggle here meant the
    // scenarios exercised this function rather than the program, and the
    // off-branch closed an overlay instead of restoring the panels — so
    // "toggle it off" never turned anything off.
    let app = world.app()?;
    app.view = app.view.toggled();
    Ok(())
}

#[when("I refresh panels")]
fn when_refresh(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    match funny_file_manager::utils::refresh_panels(app) {
        Ok(()) => app.set_status(Status::PanelsRefreshed),
        Err(e) => app.set_status(Status::RefreshFailed(e.to_string())),
    }
    Ok(())
}

#[when("I toggle hide left panel")]
fn when_hide_left(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if matches!(app.overlay, Overlay::Help) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::Help); // reuse Help overlay as hide toggle for tests
    }
    Ok(())
}

#[when("I toggle hide right panel")]
fn when_hide_right(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if matches!(app.overlay, Overlay::DriveSelection { .. }) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::DriveSelection {
            side: PanelSide::Right,
            selected_index: 0,
            rows: funny_file_manager::models::drive_rows(),
        });
    }
    Ok(())
}

// Command line — use process_popup_input to execute commands
#[when(expr = "I execute command {string}")]
fn when_exec_cmd(world: &mut FfmWorld, cmd: String) -> StepResult {
    let app = world.app()?;
    app.set_overlay(Overlay::CommandLine(cmd.into()));
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    process_popup_input(app, key);
    Ok(())
}

#[when(expr = "I navigate to {string}")]
fn when_navigate_to(world: &mut FfmWorld, sub: String) -> StepResult {
    let app = world.app()?;
    let new_path = app.get_active_panel().path.join(&sub);
    let panel = app.get_active_panel();
    panel.path = new_path;
    panel.load_entries().context("reload the panel")?;
    panel.ensure_selection();
    Ok(())
}

// Keyboard shortcuts
#[when("I set should_quit")]
fn when_set_quit(world: &mut FfmWorld) -> StepResult {
    world.app()?.lifecycle = Lifecycle::Quitting;
    Ok(())
}

#[when("I go to root")]
fn when_go_root(world: &mut FfmWorld) -> StepResult {
    when_exec_cmd(world, "cd /".to_owned())
}

#[when("I go to parent directory")]
fn when_go_parent(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if let Some(parent) = app.get_active_panel().path.parent().map(Path::to_path_buf) {
        app.get_active_panel().path = parent;
        app.get_active_panel()
            .load_entries()
            .context("reload the panel")?;
    }
    Ok(())
}

#[when("I press PageUp")]
fn when_page_up(world: &mut FfmWorld) -> StepResult {
    let panel = world.panel()?;
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        panel.select(Some(cur.saturating_sub(10)));
    }
    Ok(())
}

#[when("I press PageDown")]
fn when_page_down(world: &mut FfmWorld) -> StepResult {
    let panel = world.panel()?;
    if !panel.entries.is_empty() {
        let cur = panel.selected().unwrap_or(0);
        let max = panel.entries.len() - 1;
        panel.select(Some((cur + 10).min(max)));
    }
    Ok(())
}

#[when("I insert filename into command line")]
fn when_insert_filename(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if let Some(name) = app
        .get_active_panel()
        .get_selected_entry()
        .map(|e| e.name.clone())
    {
        if app.get_command_line().is_none() {
            app.set_overlay(Overlay::CommandLine(String::new().into()));
        }
        let cmd = app.get_command_line_mut().context("command line")?;
        if !cmd.is_empty() {
            cmd.insert_char(' ');
        }
        cmd.insert_str(&name);
    }
    Ok(())
}

#[when("I insert full path into command line")]
fn when_insert_path(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if let Some(path) = app.get_active_panel().selected_path() {
        let s = path.display().to_string();
        if app.get_command_line().is_none() {
            app.set_overlay(Overlay::CommandLine(String::new().into()));
        }
        let cmd = app.get_command_line_mut().context("command line")?;
        if !cmd.is_empty() {
            cmd.insert_char(' ');
        }
        cmd.insert_str(&s);
    }
    Ok(())
}

#[when("I sort by name")]
fn when_sort_by_name(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let panel = app.get_active_panel();
    panel.sort_mode = SortMode::Name;
    panel.load_entries().context("reload the panel")?;
    app.set_status(Status::Sort("Name".to_owned()));
    Ok(())
}

#[when("I sort by extension")]
fn when_sort_by_ext(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let panel = app.get_active_panel();
    panel.sort_mode = SortMode::Extension;
    panel.load_entries().context("reload the panel")?;
    app.set_status(Status::Sort("Extension".to_owned()));
    Ok(())
}

#[when("I sort by size")]
fn when_sort_by_size(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let panel = app.get_active_panel();
    panel.sort_mode = SortMode::Size;
    panel.load_entries().context("reload the panel")?;
    app.set_status(Status::Sort("Size".to_owned()));
    Ok(())
}

// Marking
#[when("I toggle mark")]
fn when_toggle_mark(world: &mut FfmWorld) -> StepResult {
    world.panel()?.toggle_mark_selected();
    Ok(())
}

#[when("I clear all marks")]
fn when_clear_marks(world: &mut FfmWorld) -> StepResult {
    world.panel()?.clear_marks();
    Ok(())
}

#[when("I select all files")]
fn when_select_all(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
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
    Ok(())
}

// Overlays
#[when("I toggle help")]
fn when_toggle_help(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    if matches!(app.overlay, Overlay::Help) {
        app.close_overlay();
    } else {
        app.set_overlay(Overlay::Help);
    }
    Ok(())
}

#[when("I open new file dialog")]
fn when_open_new_file(world: &mut FfmWorld) -> StepResult {
    world
        .app()?
        .set_overlay(Overlay::NewFileInput(String::new()));
    Ok(())
}

#[when("I open the make folder dialog")]
fn when_open_make_folder(world: &mut FfmWorld) -> StepResult {
    world
        .app()?
        .set_overlay(Overlay::NamePrompt(NamePrompt::make_folder()));
    Ok(())
}

#[when(expr = "I type folder name {string}")]
fn when_type_folder_name(world: &mut FfmWorld, name: String) -> StepResult {
    world.type_in_popup(&name)
}

#[when("I enable processing multiple folder names")]
fn when_enable_multiple_folder_names(world: &mut FfmWorld) -> StepResult {
    // Name -> link type -> multiple names. The disabled target field is
    // intentionally skipped, matching keyboard navigation in the dialog.
    world.send_popup_key(KeyCode::Down)?;
    world.send_popup_key(KeyCode::Down)?;
    world.send_popup_key(KeyCode::Char(' '))
}

#[when("I confirm the make folder dialog")]
fn when_confirm_make_folder(world: &mut FfmWorld) -> StepResult {
    world.send_popup_key(KeyCode::Enter)
}

#[when("I cancel the make folder dialog")]
fn when_cancel_make_folder(world: &mut FfmWorld) -> StepResult {
    world.send_popup_key(KeyCode::Esc)
}

#[when(expr = "I create folder {string} through F7")]
fn when_create_folder_through_f7(world: &mut FfmWorld, name: String) -> StepResult {
    let mut events = Vec::with_capacity(name.chars().count() + 3);
    push_make_folder(&mut events, &name, false);
    push_quit(&mut events);
    world.run_key_workflow(events, "F7 folder workflow")
}

#[when(expr = "I scaffold project {string} with folders {string} through F7")]
fn when_scaffold_project_through_f7(
    world: &mut FfmWorld,
    project: String,
    folders: String,
) -> StepResult {
    let mut events = Vec::new();
    push_make_folder(&mut events, &project, false);
    events.push(key_event(KeyCode::Enter));
    push_make_folder(&mut events, &folders, true);
    push_quit(&mut events);
    world.run_key_workflow(events, "F7 project scaffolding workflow")
}

#[when(expr = "I create nested folder path {string} through F7")]
fn when_create_nested_folder_path_through_f7(world: &mut FfmWorld, path: String) -> StepResult {
    let components = path
        .split('/')
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>();
    assert!(!components.is_empty(), "nested folder path cannot be empty");

    let mut events = Vec::new();
    for (index, component) in components.iter().enumerate() {
        push_make_folder(&mut events, component, false);
        if index + 1 < components.len() {
            events.push(key_event(KeyCode::Enter));
        }
    }
    push_quit(&mut events);
    world.run_key_workflow(events, "F7 nested-folder workflow")
}

#[when(expr = "I create folder {string} on the right panel through F7")]
fn when_create_folder_on_right_panel_through_f7(world: &mut FfmWorld, name: String) -> StepResult {
    let mut events = vec![key_event(KeyCode::Tab)];
    push_make_folder(&mut events, &name, false);
    push_quit(&mut events);
    world.run_key_workflow(events, "right-panel F7 folder workflow")
}

#[when(expr = "I copy the selection through F5 using {string}")]
fn when_copy_through_f5_with_policy(world: &mut FfmWorld, policy: String) -> StepResult {
    let mut events = Vec::new();
    push_transfer_with_policy(&mut events, 5, &policy)?;
    push_quit(&mut events);
    world.run_key_workflow(events, "F5 transfer-dialog workflow")
}

#[when(expr = "I move the selection through F6 using {string}")]
fn when_move_through_f6_with_policy(world: &mut FfmWorld, policy: String) -> StepResult {
    let mut events = Vec::new();
    push_transfer_with_policy(&mut events, 6, &policy)?;
    push_quit(&mut events);
    world.run_key_workflow(events, "F6 transfer-dialog workflow")
}

#[when("I cancel the F5 copy dialog")]
fn when_cancel_f5_copy_dialog(world: &mut FfmWorld) -> StepResult {
    let mut events = vec![key_event(KeyCode::F(5)), key_event(KeyCode::Esc)];
    push_quit(&mut events);
    world.run_key_workflow(events, "cancelled F5 transfer-dialog workflow")
}

#[when(expr = "I copy the selection through F5 to relative destination {string}")]
fn when_copy_through_f5_to_relative_destination(
    world: &mut FfmWorld,
    destination: String,
) -> StepResult {
    let offered_destination = world
        .app
        .as_ref()
        .context("no app")?
        .get_inactive_panel_ref()
        .path
        .display()
        .to_string();
    let mut events = vec![key_event(KeyCode::F(5))];
    events.extend(
        offered_destination
            .chars()
            .map(|_| key_event(KeyCode::Backspace)),
    );
    push_text(&mut events, &destination);
    events.push(key_event(KeyCode::Enter));
    push_quit(&mut events);
    world.run_key_workflow(events, "F5 custom-destination workflow")
}

#[when(expr = "I type filename {string}")]
fn when_type_filename(world: &mut FfmWorld, name: String) -> StepResult {
    world.app()?.set_overlay(Overlay::NewFileInput(name));
    Ok(())
}

#[when("I confirm new file")]
fn when_confirm_new_file(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let input = app.get_new_file_input().unwrap_or_default().to_owned();
    app.close_overlay();
    let input = input.trim().to_owned();
    if input.is_empty() {
        app.set_status(Status::CommandOutput("Filename cannot be empty".to_owned()));
        return Ok(());
    }
    let path = app.get_active_panel().path.join(&input);
    if path.exists() {
        app.set_status(Status::CommandOutput(format!(
            "File already exists: {input}"
        )));
    } else {
        fs::File::create(&path).context("create file")?;
        app.set_status(Status::CommandOutput(format!("Created file: {input}")));
        funny_file_manager::utils::refresh_panels(app).context("refresh panels")?;
    }
    Ok(())
}

#[when("I close the viewer")]
fn when_close_viewer(world: &mut FfmWorld) -> StepResult {
    world.app()?.close_overlay();
    Ok(())
}

#[when(expr = "I open drive selection for {word}")]
fn when_open_drives(world: &mut FfmWorld, side: String) -> StepResult {
    let app = world.app()?;
    let side = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    app.set_overlay(Overlay::DriveSelection {
        side,
        selected_index: 0,
        rows: funny_file_manager::models::drive_rows(),
    });
    Ok(())
}

// Editor (F4)
#[when(expr = "I press F4 with success behavior")]
fn when_f4_success(world: &mut FfmWorld) -> StepResult {
    handle_edit_test_mode(world.app()?, TestEditorBehavior::Success).context("F4 success")?;
    Ok(())
}

#[when(expr = "I press F4 with fail_launch behavior")]
fn when_f4_fail_launch(world: &mut FfmWorld) -> StepResult {
    handle_edit_test_mode(world.app()?, TestEditorBehavior::FailLaunch)
        .context("F4 fail_launch")?;
    Ok(())
}

#[when(expr = "I press F4 with exit_error behavior")]
fn when_f4_exit_error(world: &mut FfmWorld) -> StepResult {
    handle_edit_test_mode(world.app()?, TestEditorBehavior::ExitError).context("F4 exit_error")?;
    Ok(())
}

#[when(expr = "I create new file {string} in the active panel")]
fn when_create_new_file(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let path = app.get_active_panel().path.join(&name);
    fs::File::create(&path).context("create file")?;
    funny_file_manager::utils::refresh_panels(app).context("refresh panels")?;
    Ok(())
}

#[when("I switch to the right panel")]
fn when_switch_right(world: &mut FfmWorld) -> StepResult {
    world.app()?.active_panel = PanelSide::Right;
    Ok(())
}

#[when(expr = "I select {string} in the right panel")]
fn when_select_right(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .right_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found in right panel"))?;
    app.right_panel.select(Some(idx));
    Ok(())
}

#[when(expr = "I select {string} in the left panel")]
fn when_select_left(world: &mut FfmWorld, name: String) -> StepResult {
    let app = world.app()?;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == name)
        .ok_or_else(|| format!("entry '{name}' not found in left panel"))?;
    app.left_panel.select(Some(idx));
    Ok(())
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
fn when_unique_name(world: &mut FfmWorld, name: String) -> StepResult {
    let result = unique_destination(world.temp_path()?.as_path(), OsStr::new(&name))
        .context("unique name")?;
    world.unique_name = Some(
        result
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    );
    Ok(())
}

#[when(expr = "I copy directory {string} to {string}")]
fn when_copy_dir(world: &mut FfmWorld, src: String, dst: String) -> StepResult {
    let base = world.temp_path()?;
    copy_dir_all(&base.join(&src), &base.join(&dst)).context("copy dir")?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// THEN steps
// ═══════════════════════════════════════════════════════════════════

#[then(expr = "the panel should have {int} entries")]
fn then_panel_count(world: &mut FfmWorld, n: usize) -> StepResult {
    assert_eq!(world.panel()?.entries.len(), n);
    Ok(())
}

#[then(expr = "entry {int} should be {string}")]
fn then_entry(world: &mut FfmWorld, i: usize, name: String) -> StepResult {
    let entries = &world.panel()?.entries;
    let entry = entries.get(i).ok_or_else(|| {
        format!(
            "index out of bounds: the len is {} but the index is {i}",
            entries.len()
        )
    })?;
    assert_eq!(entry.name, name);
    Ok(())
}

#[then(expr = "the selection should be at index {int}")]
fn then_sel(world: &mut FfmWorld, i: usize) -> StepResult {
    assert_eq!(world.panel()?.selected(), Some(i));
    Ok(())
}

#[then(expr = "the panel path should end with {string}")]
fn then_path_ends(world: &mut FfmWorld, s: String) -> StepResult {
    let p = if let Some(ref panel) = world.panel {
        panel.path.display().to_string()
    } else {
        world.app()?.get_active_panel().path.display().to_string()
    };
    assert!(p.ends_with(&s), "'{p}' should end with '{s}'");
    Ok(())
}

#[then(expr = "the active panel path should end with {string}")]
fn then_active_ends(world: &mut FfmWorld, s: String) -> StepResult {
    let p = world.app()?.get_active_panel().path.display().to_string();
    assert!(p.ends_with(&s), "'{p}' should end with '{s}'");
    Ok(())
}

#[then(expr = "the active panel path should not end with {string}")]
fn then_active_not_ends(world: &mut FfmWorld, s: String) -> StepResult {
    let p = world.app()?.get_active_panel().path.display().to_string();
    assert!(!p.ends_with(&s), "'{p}' should not end with '{s}'");
    Ok(())
}

#[then("the active panel should be at the source filesystem root")]
fn then_active_at_root(world: &mut FfmWorld) -> StepResult {
    let source = world
        .src_path()?
        .canonicalize()
        .context("source directory")?;
    let root = source.ancestors().last().context("filesystem root")?;
    let actual = world
        .app()?
        .get_active_panel()
        .path
        .canonicalize()
        .context("active directory")?;
    assert_eq!(actual, root);
    Ok(())
}

#[then("the active panel should be at the source directory")]
fn then_active_at_source(world: &mut FfmWorld) -> StepResult {
    let source = world
        .src_path()?
        .canonicalize()
        .context("source directory")?;
    let actual = world
        .app()?
        .get_active_panel()
        .path
        .canonicalize()
        .context("active directory")?;
    assert_eq!(actual, source);
    Ok(())
}

#[then(expr = "{string} should exist in the destination")]
fn then_dst_exists(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.dst_path()?.join(&n).exists(),
        "{n} should exist in dst"
    );
    Ok(())
}

#[then(expr = "{string} should exist in the source")]
fn then_src_has(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.src_path()?.join(&n).exists(),
        "{n} should exist in src"
    );
    Ok(())
}

#[then(expr = "{string} should still exist in the source")]
fn then_src_exists(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.src_path()?.join(&n).exists(),
        "{n} should exist in src"
    );
    Ok(())
}

#[then(expr = "{string} should not exist in the source")]
fn then_src_not(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(!world.src_path()?.join(&n).exists(), "{n} should not exist");
    Ok(())
}

#[then(expr = "{string} should not exist in the destination")]
fn then_dst_not(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        !world.dst_path()?.join(&n).exists(),
        "{n} should not exist in dst"
    );
    Ok(())
}

#[then(expr = "{string} in the source should have content {string}")]
fn then_source_file_has_content(
    world: &mut FfmWorld,
    name: String,
    expected: String,
) -> StepResult {
    let path = world.src_path()?.join(&name);
    let actual =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    assert_eq!(actual, expected, "unexpected content in {}", path.display());
    Ok(())
}

#[then(expr = "{string} in the destination should have content {string}")]
fn then_destination_file_has_content(
    world: &mut FfmWorld,
    name: String,
    expected: String,
) -> StepResult {
    let path = world.dst_path()?.join(&name);
    let actual =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    assert_eq!(actual, expected, "unexpected content in {}", path.display());
    Ok(())
}

#[then(expr = "the status message should contain {string}")]
fn then_status(world: &mut FfmWorld, t: String) -> StepResult {
    let m = &world.app()?.status_text();
    assert!(m.contains(&t), "'{m}' should contain '{t}'");
    Ok(())
}

#[then(expr = "the {string} diagnostic should match the current platform")]
fn then_windows_diagnostic(world: &mut FfmWorld, title: String) -> StepResult {
    let app = world.app()?;
    #[cfg(windows)]
    {
        assert_eq!(app.status, Status::OpenedView(title.clone()));
        let viewer = app.get_viewer().context("Windows diagnostic report")?;
        assert_eq!(viewer.title(), title);
        assert!(!viewer.lines.is_empty(), "report must include its result");
    }
    #[cfg(not(windows))]
    {
        let tool = match title.as_str() {
            "Windows System Messages" => "system",
            "Windows Performance" => "performance",
            other => return Err(format!("unknown Windows diagnostic: {other}")),
        };
        assert_eq!(app.status, Status::WindowsOnly { tool });
        assert!(app.get_viewer().is_none());
    }
    Ok(())
}

#[then("the status message should not be empty")]
fn then_status_not_empty(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.status_text().is_empty());
    Ok(())
}

#[then(expr = "the status message should not contain {string}")]
fn then_status_not_contain(world: &mut FfmWorld, t: String) -> StepResult {
    let m = &world.app()?.status_text();
    assert!(!m.contains(&t), "'{m}' should NOT contain '{t}'");
    Ok(())
}

#[then(expr = "{string} should be a directory in the source")]
fn then_dir_src(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.src_path()?.join(&n).is_dir(),
        "{n} should be a directory in src"
    );
    Ok(())
}

#[then(expr = "{string} should be a directory in the destination")]
fn then_dir_dst(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.dst_path()?.join(&n).is_dir(),
        "{n} should be a directory in dst"
    );
    Ok(())
}

#[then(expr = "the active panel selection should be {string}")]
fn then_active_selection(world: &mut FfmWorld, name: String) -> StepResult {
    let selected = world
        .app()?
        .get_active_panel_ref()
        .get_selected_entry()
        .map(|entry| entry.name.as_str());
    assert_eq!(selected, Some(name.as_str()));
    Ok(())
}

#[then("the make folder dialog should be closed")]
fn then_make_folder_dialog_closed(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.is_overlay_active());
    Ok(())
}

#[then("a delete confirmation should be shown")]
fn then_del_confirm(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_delete_confirm().is_some());
    Ok(())
}

#[then("a delete confirmation should not be shown")]
fn then_no_del_confirm(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_delete_confirm().is_none());
    Ok(())
}

#[then("the viewer should be open")]
fn then_viewer_open(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_viewer().is_some());
    Ok(())
}

#[then("the viewer should not be open")]
fn then_viewer_closed(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_viewer().is_none());
    Ok(())
}

#[then(expr = "the viewer title should contain {string}")]
fn then_viewer_title(world: &mut FfmWorld, t: String) -> StepResult {
    let title = world.app()?.get_viewer().context("viewer")?.title();
    assert!(title.contains(&t), "'{title}' should contain '{t}'");
    Ok(())
}

#[then(expr = "the viewer should show {string}")]
fn then_viewer_shows(world: &mut FfmWorld, t: String) -> StepResult {
    let lines = &world.app()?.get_viewer().context("viewer")?.lines;
    assert!(lines.join("\n").contains(&t));
    Ok(())
}

#[then(expr = "the selected entry should be {string}")]
fn then_selected(world: &mut FfmWorld, n: String) -> StepResult {
    assert_eq!(world.panel()?.get_selected_entry().context("sel")?.name, n);
    Ok(())
}

#[then(expr = "bookmark {int} should be set to {string}")]
fn then_bookmark(world: &mut FfmWorld, slot: usize, sub: String) -> StepResult {
    let bm = world
        .app()?
        .bookmarks
        .get(BookmarkSlot::from_usize(slot).context("slot")?)
        .context("bookmark")?;
    assert!(bm.display().to_string().contains(&sub));
    Ok(())
}

#[then("the sort mode should be back to Name")]
fn then_sort_name(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.panel()?.sort_mode, SortMode::Name);
    Ok(())
}

#[then("the first extension should come before the last alphabetically")]
fn then_ext_sorted(world: &mut FfmWorld) -> StepResult {
    let files: Vec<_> = world
        .panel()?
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir() && e.name != "..")
        .collect();
    if let [first, .., last] = files.as_slice() {
        let a = first.name.rsplit('.').next().unwrap_or("");
        let b = last.name.rsplit('.').next().unwrap_or("");
        assert!(a <= b, "'{a}' should <= '{b}'");
    }
    Ok(())
}

// Panel operations
#[then(expr = "the active panel should be {word}")]
fn then_active_panel(world: &mut FfmWorld, side: String) -> StepResult {
    let expected = match side.as_str() {
        "Left" => PanelSide::Left,
        _ => PanelSide::Right,
    };
    assert_eq!(world.app()?.active_panel, expected);
    Ok(())
}

#[then(expr = "only the {word} panel should be on screen")]
fn then_only_panel_on_screen(world: &mut FfmWorld, side: String) -> StepResult {
    let expected = match side.as_str() {
        "Left" => PanelsShown::LeftOnly,
        _ => PanelsShown::RightOnly,
    };
    assert_eq!(world.app()?.panels_shown, expected);
    Ok(())
}

#[then("both panels should be on screen")]
fn then_both_panels_on_screen(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.app()?.panels_shown, PanelsShown::Both);
    Ok(())
}

#[then(expr = "the {word} directory should be drawn")]
fn then_directory_drawn(world: &mut FfmWorld, which: String) -> StepResult {
    let name = world.directory_name(&which)?;
    let screen = world.screen_text()?;
    assert!(screen.contains(&name), "{name} is on screen:\n{screen}");
    Ok(())
}

#[then(expr = "the {word} directory should not be drawn")]
fn then_directory_not_drawn(world: &mut FfmWorld, which: String) -> StepResult {
    let name = world.directory_name(&which)?;
    let screen = world.screen_text()?;
    assert!(!screen.contains(&name), "{name} is off screen:\n{screen}");
    Ok(())
}

#[then("console mode should be off")]
fn then_console_off(world: &mut FfmWorld) -> StepResult {
    assert_ne!(world.app()?.view, ViewMode::Console);
    Ok(())
}

#[then("console mode should be on")]
fn then_console_on(world: &mut FfmWorld) -> StepResult {
    assert_eq!(world.app()?.view, ViewMode::Console);
    Ok(())
}

#[then("the left panel path should differ from original")]
fn then_left_differs(world: &mut FfmWorld) -> StepResult {
    let orig = world.original_left_path.clone();
    if let Some(orig) = orig {
        assert_ne!(&world.app()?.left_panel.path, &orig);
    }
    Ok(())
}

// Command line
#[then(expr = "the sort mode should be {word}")]
fn then_sort_mode(world: &mut FfmWorld, mode: String) -> StepResult {
    let expected = match mode.as_str() {
        "Name" => SortMode::Name,
        "Extension" => SortMode::Extension,
        "Size" => SortMode::Size,
        _ => return Err(format!("unknown sort mode: {mode}")),
    };
    assert_eq!(world.app()?.get_active_panel().sort_mode, expected);
    Ok(())
}

#[then("the active panel should have marks")]
fn then_has_marks(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.get_active_panel().marked.is_empty());
    Ok(())
}

#[then("the active panel should have no marks")]
fn then_no_marks(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_active_panel().marked.is_empty());
    Ok(())
}

#[then("help should be visible")]
fn then_help_on(world: &mut FfmWorld) -> StepResult {
    assert!(matches!(world.app()?.overlay, Overlay::Help));
    Ok(())
}

#[then("help should not be visible")]
fn then_help_off(world: &mut FfmWorld) -> StepResult {
    assert!(!matches!(world.app()?.overlay, Overlay::Help));
    Ok(())
}

#[then("the app should be quitting")]
fn then_quitting(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.lifecycle.is_running());
    Ok(())
}

#[then("drive selection should be visible")]
fn then_drives_visible(world: &mut FfmWorld) -> StepResult {
    assert!(matches!(
        world.app()?.overlay,
        Overlay::DriveSelection { .. }
    ));
    Ok(())
}

// Marking
#[then(expr = "{string} should be marked")]
fn then_marked(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(world.panel()?.marked.contains(n.as_str()));
    Ok(())
}

#[then(expr = "{string} should not be marked")]
fn then_not_marked(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(!world.panel()?.marked.contains(n.as_str()));
    Ok(())
}

#[then(expr = "{string} should be marked in the app")]
fn then_marked_app(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(world.app()?.get_active_panel().marked.contains(n.as_str()));
    Ok(())
}

#[then(expr = "{string} should not be marked in the app")]
fn then_not_marked_app(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(!world.app()?.get_active_panel().marked.contains(n.as_str()));
    Ok(())
}

#[then("the panel should have no marks")]
fn then_panel_no_marks(world: &mut FfmWorld) -> StepResult {
    assert!(world.panel()?.marked.is_empty());
    Ok(())
}

// Keyboard shortcuts
#[then(expr = "the command line should contain {string}")]
fn then_cmd_has(world: &mut FfmWorld, t: String) -> StepResult {
    let cmd = world.app()?.get_command_line().unwrap_or("");
    assert!(cmd.contains(&t), "cmd '{cmd}' should contain '{t}'");
    Ok(())
}

// Overlays
#[then(expr = "{string} should exist in the temporary directory")]
fn then_temp_exists(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(world.temp_path()?.join(&n).exists(), "{n} should exist");
    Ok(())
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
fn then_display(world: &mut FfmWorld, s: String) -> StepResult {
    assert_eq!(world.parsed_path.as_ref().context("path")?.display(), s);
    Ok(())
}

#[then(expr = "the profile should be {string}")]
fn then_profile(world: &mut FfmWorld, p: String) -> StepResult {
    match world.parsed_path.as_ref().context("path")? {
        VirtualPath::S3 { profile, .. } => assert_eq!(profile, &p),
        VirtualPath::Local(_) => return Err("expected S3 path".to_owned()),
    }
    Ok(())
}

#[then(expr = "the bucket should be {string}")]
fn then_bucket(world: &mut FfmWorld, b: String) -> StepResult {
    match world.parsed_path.as_ref().context("path")? {
        VirtualPath::S3 { bucket, .. } => assert_eq!(bucket.as_deref(), Some(b.as_str())),
        VirtualPath::Local(_) => return Err("expected S3 path".to_owned()),
    }
    Ok(())
}

#[then("the bucket should be empty")]
fn then_bucket_empty(world: &mut FfmWorld) -> StepResult {
    match world.parsed_path.as_ref().context("path")? {
        VirtualPath::S3 { bucket, .. } => assert!(bucket.is_none()),
        VirtualPath::Local(_) => return Err("expected S3 path".to_owned()),
    }
    Ok(())
}

#[then(expr = "the prefix should be {string}")]
fn then_prefix(world: &mut FfmWorld, p: String) -> StepResult {
    match world.parsed_path.as_ref().context("path")? {
        VirtualPath::S3 { prefix, .. } => assert_eq!(prefix, &p),
        VirtualPath::Local(_) => return Err("expected S3 path".to_owned()),
    }
    Ok(())
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
fn then_left_has_entries(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.left_panel.entries.is_empty());
    Ok(())
}

#[then("the right panel should have entries")]
fn then_right_has_entries(world: &mut FfmWorld) -> StepResult {
    assert!(!world.app()?.right_panel.entries.is_empty());
    Ok(())
}

#[then("the left panel should have a valid selection")]
#[expect(
    clippy::unwrap_used,
    reason = "the assert on the previous line proves sel is Some"
)]
fn then_left_valid_sel(world: &mut FfmWorld) -> StepResult {
    let app = world.app()?;
    let sel = app.left_panel.selected();
    assert!(sel.is_some(), "left panel should have a selection");
    assert!(
        sel.unwrap() < app.left_panel.entries.len(),
        "selection should be within bounds"
    );
    Ok(())
}

// ─── Describe files (Ctrl+Z) ────────────────────────────────────

/// Ctrl+Z as the event loop sees it, so the scenario runs the real path:
/// key -> action -> dispatch -> the dialog -> the file on disk.
fn push_describe_key(events: &mut Vec<Event>) {
    events.push(Event::Key(KeyEvent::new(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL,
    )));
}

#[when(expr = "I describe the selection as {string} through Ctrl+Z")]
fn when_describe_selection(world: &mut FfmWorld, text: String) -> StepResult {
    let mut events = Vec::new();
    push_describe_key(&mut events);
    push_text(&mut events, &text);
    events.push(key_event(KeyCode::Enter));
    push_quit(&mut events);
    world.run_key_workflow(events, "Ctrl+Z describe workflow")
}

#[when(expr = "I describe the marked files as {string} and {string} through Ctrl+Z")]
fn when_describe_marked(world: &mut FfmWorld, first: String, second: String) -> StepResult {
    let mut events = Vec::new();
    push_describe_key(&mut events);
    push_text(&mut events, &first);
    events.push(key_event(KeyCode::Enter));
    push_text(&mut events, &second);
    events.push(key_event(KeyCode::Enter));
    push_quit(&mut events);
    world.run_key_workflow(events, "Ctrl+Z describe-marked workflow")
}

#[when(expr = "I cancel the Describe files dialog after typing {string}")]
fn when_cancel_describe(world: &mut FfmWorld, text: String) -> StepResult {
    let mut events = Vec::new();
    push_describe_key(&mut events);
    push_text(&mut events, &text);
    events.push(key_event(KeyCode::Esc));
    push_quit(&mut events);
    world.run_key_workflow(events, "cancelled Ctrl+Z describe workflow")
}

fn assert_source_line(world: &FfmWorld, file: &str, line: &str) -> StepResult {
    let path = world.src_path()?.join(file);
    let content =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    assert!(
        content.lines().any(|found| found == line),
        "{} should hold the line {line:?}, holds: {content:?}",
        path.display()
    );
    Ok(())
}

#[then(expr = "{string} in the source should have the line {string}")]
fn then_source_file_has_line(world: &mut FfmWorld, file: String, line: String) -> StepResult {
    assert_source_line(world, &file, &line)
}

/// Far quotes a name that holds a space; a `{string}` step cannot carry the
/// quotes itself, so the line is assembled here.
#[then(expr = "{string} in the source should quote {string} before {string}")]
fn then_source_file_quotes_name(
    world: &mut FfmWorld,
    file: String,
    name: String,
    text: String,
) -> StepResult {
    assert_source_line(world, &file, &format!("\"{name}\" {text}"))
}

#[then(expr = "the description of {string} in the source should be {string}")]
fn then_source_description(world: &mut FfmWorld, name: String, expected: String) -> StepResult {
    let descriptions = description::read(&world.src_path()?, &description::default_names());
    assert_eq!(descriptions.get(&name), Some(&expected), "{descriptions:?}");
    Ok(())
}

#[path = "support/bdd_runner.rs"]
mod bdd_runner;
#[path = "bdd/mcp_install.rs"]
mod mcp_install;

fn main() -> io::Result<()> {
    bdd_runner::run(|| futures::executor::block_on(Box::pin(FfmWorld::run("tests/features"))))
}

fn assert_timestamp_order(source: &Path, destination: &Path, source_is_newer: bool) -> StepResult {
    let actual_source = fs::metadata(source)
        .and_then(|metadata| metadata.modified())
        .context("read source timestamp")?;
    let actual_destination = fs::metadata(destination)
        .and_then(|metadata| metadata.modified())
        .context("read destination timestamp")?;
    assert_eq!(
        actual_source > actual_destination,
        source_is_newer,
        "the filesystem did not retain the requested fixture timestamps"
    );
    Ok(())
}
