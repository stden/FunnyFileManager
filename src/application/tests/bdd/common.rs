//! Shared fixture, key and rendered-screen steps.

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use super::{BehaviorWorld, Case};
use crate::{
    action::Action,
    menu::{Entry, MenuCommand},
    models::Overlay,
};
use cucumber::{given, then, when};

#[given("an isolated acceptance workspace")]
fn workspace(world: &mut BehaviorWorld) {
    world.case = Some(Case::new());
    world.case().paint();
}

#[when(expr = "I dispatch the keys {string}")]
fn dispatch_keys(world: &mut BehaviorWorld, keys: String) {
    world.case().press(&keys);
}

#[when(expr = "I type characters {string}")]
fn type_chars(world: &mut BehaviorWorld, text: String) {
    world.case().type_chars(&text);
}

#[then(expr = "the rendered screen contains {string}")]
fn contains(world: &mut BehaviorWorld, expected: String) {
    let screen = world.case().screen.text();
    assert!(
        screen.contains(&expected),
        "missing {expected:?}\n{}",
        visible_rows(&screen)
    );
}

#[then(expr = "the rendered screen excludes {string}")]
fn excludes(world: &mut BehaviorWorld, text: String) {
    let screen = world.case().screen.text();
    assert!(
        !screen.contains(&text),
        "unexpected {text:?}\n{}",
        visible_rows(&screen)
    );
}

fn visible_rows(screen: &str) -> String {
    let chars: Vec<char> = screen.chars().collect();
    chars
        .chunks(120)
        .map(|row| row.iter().collect::<String>().trim_end().to_owned())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[then(expr = "the {word} panel cursor selects {string}")]
fn selected(world: &mut BehaviorWorld, side: String, name: String) {
    let panel = world.case().app.panel(Case::side(&side));
    assert_eq!(panel.get_selected_entry().unwrap().name, name);
}

#[then("the terminal presents complete synchronized frames")]
fn synchronized(world: &mut BehaviorWorld) {
    let writes = world.case().screen.writes();
    let (frames, remainder) = writes.as_chunks::<16>();
    assert_ne!(frames.len(), 0);
    assert_eq!(remainder, b"");
    assert!(frames
        .iter()
        .all(|frame| frame == b"\x1b[?2026h\x1b[?2026l"));
}

#[then("the overlay is closed")]
fn overlay_closed(world: &mut BehaviorWorld) {
    let overlay = &world.case().app.overlay;
    assert!(matches!(overlay, Overlay::None), "expected no overlay");
}

/// Rows the Commands drop-down may need pressing past before it reaches one.
const MAX_MENU_PRESSES: usize = 100;

/// Opens F9's Commands drop-down as a user does (Alt+C, then Home), walks it to
/// `command`'s row, and runs that row with Enter.
pub(super) fn open_command_row(case: &mut Case, command: MenuCommand) {
    case.press("F9 Alt+C Home");
    for _ in 0..MAX_MENU_PRESSES {
        let Overlay::Menu(menu) = &case.app.overlay else {
            panic!("the F9 Commands menu must be open");
        };
        if menu.entry().and_then(Entry::action) == Some(Action::Menu(command)) {
            case.press("Enter");
            return;
        }
        case.press("Down");
    }
    panic!("the Commands menu must offer {command:?}");
}

/// Yields up to 5 seconds until `done()` returns true.
pub(super) fn wait_until(label: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "{label} did not finish");
        std::thread::yield_now();
    }
}

/// One-shot gate holding a background worker reply until released.
#[derive(Default)]
pub(super) struct DelayGate {
    release: Option<Sender<()>>,
    wait: Option<Receiver<()>>,
}

impl DelayGate {
    pub(super) fn delayed() -> Self {
        let (release, wait) = mpsc::channel();
        Self {
            release: Some(release),
            wait: Some(wait),
        }
    }

    pub(super) fn blocker(&mut self) -> Option<Receiver<()>> {
        self.wait.take()
    }

    pub(super) fn release(&mut self) {
        self.release.take();
    }
}

pub(super) fn await_gate(blocker: Option<Receiver<()>>) {
    if let Some(rx) = blocker {
        if rx.recv().is_err() {}
    }
}
