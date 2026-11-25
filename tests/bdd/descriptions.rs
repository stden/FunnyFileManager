//! Purpose: File descriptions: describing the selection or the marked files, and the file it
//! writes.

use super::*;

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
    let content = fs::read_to_string(&path).context("read")?;
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

#[then(expr = "a missing line in {string} names that file")]
fn then_missing_line_names_file(world: &mut FfmWorld, file: String) {
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drop(assert_source_line(world, &file, "this line is absent"));
    }));
    assert!(
        caught.is_err(),
        "a missing source line should fail the check"
    );
}
