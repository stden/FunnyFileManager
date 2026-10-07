//! Purpose: Copy, move and delete from F5, F6 and F8, with overwrite policies and destinations.

use super::*;

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

#[when("I press F8")]
fn when_f8(world: &mut FfmWorld) -> StepResult {
    handle_delete_confirm(world.app()?).context("F8")?;
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

#[when(expr = "I copy the selection through F5 using {string}")]
fn when_copy_through_f5_with_policy(world: &mut FfmWorld, policy: String) -> StepResult {
    let mut events = Vec::new();
    push_transfer_with_policy(&mut events, 5, &policy)?;
    push_quit(&mut events);
    world.run_key_workflow(events, "F5 transfer-dialog workflow")
}

#[when("I copy the selection through F5 asking, and keep both")]
fn when_copy_through_f5_asking_keep_both(world: &mut FfmWorld) -> StepResult {
    let mut events = Vec::new();
    push_transfer_with_policy(&mut events, 5, "Ask")?;
    // "File already exists": K for Keep both.
    events.push(key_event(KeyCode::Char('k')));
    push_quit(&mut events);
    world.run_key_workflow(events, "F5 transfer-dialog workflow, keeping both")
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

#[when(expr = "I copy directory {string} to {string}")]
fn when_copy_dir(world: &mut FfmWorld, src: String, dst: String) -> StepResult {
    let base = world.temp_path()?;
    copy_dir_all(&base.join(&src), &base.join(&dst)).context("copy dir")?;
    Ok(())
}
