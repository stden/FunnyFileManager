//! Queued cursor movement, command boundaries and synchronized repaint failures.

use super::{BehaviorWorld, Case};
use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use cucumber::{given, then, when};

#[given(expr = "both panels contain {int} numbered documents")]
fn documents(world: &mut BehaviorWorld, count: usize) {
    let case = world.case();
    for side in ["left", "right"] {
        for index in 0..count {
            let file = format!("file{index:03}.txt");
            std::fs::write(
                case.root.path().join(side).join(file),
                format!("document {index}"),
            )
            .unwrap();
        }
        case.app.panel_mut(Case::side(side)).load_entries().unwrap();
    }
    case.paint();
}

#[given("j moves down and Down opens deletion confirmation")]
fn remap(world: &mut BehaviorWorld) {
    let keys = &mut world.case().app.keymap.panels;
    keys.bind(crate::action::Action::NavDown, "j".parse().unwrap());
    keys.bind(
        crate::action::Action::DeleteConfirm,
        "Down".parse().unwrap(),
    );
}

#[when(expr = "I queue {int} presses of {string}")]
fn repeated(world: &mut BehaviorWorld, count: usize, key: String) {
    world
        .case()
        .queued
        .extend(std::iter::repeat_n(Case::key(&key), count));
}

#[when(expr = "I queue the keys {string}")]
fn queue_keys(world: &mut BehaviorWorld, keys: String) {
    world
        .case()
        .queued
        .extend(keys.split_whitespace().map(Case::key));
}

#[when(expr = "I queue {int} downward wheel ticks over the {word} panel")]
fn wheel(world: &mut BehaviorWorld, count: usize, side: String) {
    let column = if side == "left" { 20 } else { 80 };
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column,
        row: 5,
        modifiers: KeyModifiers::NONE,
    });
    world
        .case()
        .queued
        .extend(std::iter::repeat_n(event, count));
}

#[when(expr = "I queue a terminal resize to {int} by {int}")]
fn resize(world: &mut BehaviorWorld, width: u16, height: u16) {
    world.case().queued.push_back(Event::Resize(width, height));
}

#[when("I process the queued input")]
fn process(world: &mut BehaviorWorld) {
    world.case().batch();
}

#[then("queued navigation yields frames within its processing budget")]
fn frames(world: &mut BehaviorWorld) {
    let case = world.case();
    assert!(
        case.batch_frames >= case.batch_events.div_ceil(64),
        "a long burst must yield frames"
    );
    assert!(case.batch_frames <= case.batch_events + 1);
    if case.batch_frames > case.batch_events {
        assert_budget_spent(case);
    }
}

fn assert_budget_spent(case: &Case) {
    let report = super::statistics::saved(case);
    let row = report["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            matches!(
                row["action"].as_str(),
                Some("Navigate(NavDown)" | "Panels/Mouse(ScrollDown)")
            )
        })
        .expect("navigation must be measured");
    let count = u32::try_from(row["count"].as_u64().unwrap()).unwrap();
    let events = u32::try_from(case.batch_events).unwrap();
    let boundaries = events.saturating_sub(count).saturating_add(1);
    let budgeted_keys = count.saturating_sub(boundaries);
    let processing_ms = row["total_ms"].as_f64().unwrap() - row["render_ms"].as_f64().unwrap();
    // Burst tails have no following movement to combine. Every other key
    // must have spent the budget to justify a per-key repaint under load.
    assert!(
        processing_ms + 0.001 >= f64::from(budgeted_keys) * 8.0,
        "fast queued navigation must share frames: {report}"
    );
}

#[then(expr = "the rendered terminal size is {int} by {int}")]
fn size(world: &mut BehaviorWorld, width: u16, height: u16) {
    assert_eq!(
        world.case().terminal.size().unwrap(),
        ratatui::layout::Size::new(width, height)
    );
}

#[when("the backend refuses the next repaint")]
fn failed_repaint(world: &mut BehaviorWorld) {
    let case = world.case();
    case.screen.fail_next_draw();
    case.failed_frame =
        crate::application::frame::draw_frame(&mut case.terminal, &mut case.app).is_err();
}

#[then("the repaint failure is reported")]
fn failed(world: &mut BehaviorWorld) {
    assert!(world.case().failed_frame);
}
