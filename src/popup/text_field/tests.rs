//! Purpose: The keys every text field shares: Shift-selection, select all,
//! and copy, cut and paste through the clipboard.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use crate::clipboard::ClipboardBackend;

/// A clipboard in memory, shared with the test that installed it.
#[derive(Clone, Default)]
struct Memory(Rc<RefCell<String>>);

impl ClipboardBackend for Memory {
    fn get_native(&mut self) -> Result<String, String> {
        Ok(self.0.borrow().clone())
    }

    fn set_native(&mut self, text: &str) -> Result<(), String> {
        text.clone_into(&mut self.0.borrow_mut());
        Ok(())
    }

    fn emit_osc52(&mut self, _payload: &str) -> Result<(), String> {
        Err("no terminal in a test".to_owned())
    }
}

fn slot() -> (Memory, ClipboardSlot) {
    let memory = Memory::default();
    let mut slot = ClipboardSlot::default();
    slot.install(Box::new(memory.clone()));
    (memory, slot)
}

fn press(
    line: &mut LineEditor,
    slot: &mut ClipboardSlot,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> bool {
    edit(line, code, modifiers, slot)
}

#[test]
fn shift_arrows_select_and_ctrl_c_copies_the_selection() {
    let (memory, mut slot) = slot();
    let mut line = LineEditor::new("report.txt");
    for _ in 0..3 {
        press(&mut line, &mut slot, KeyCode::Left, KeyModifiers::SHIFT);
    }
    assert!(!press(
        &mut line,
        &mut slot,
        KeyCode::Char('c'),
        KeyModifiers::CONTROL
    ));
    assert_eq!(*memory.0.borrow(), "txt");
    press(&mut line, &mut slot, KeyCode::Insert, KeyModifiers::CONTROL);
    assert_eq!(line.text(), "report.txt", "copying leaves the text alone");
}

#[test]
fn ctrl_shift_arrows_select_by_word_and_shift_home_end_to_the_ends() {
    let (_memory, mut slot) = slot();
    let mut line = LineEditor::new("one two");
    press(
        &mut line,
        &mut slot,
        KeyCode::Left,
        KeyModifiers::SHIFT | KeyModifiers::CONTROL,
    );
    assert_eq!(line.selected_text(), Some("two"));
    press(&mut line, &mut slot, KeyCode::Home, KeyModifiers::SHIFT);
    assert_eq!(line.selected_text(), Some("one two"));
    press(&mut line, &mut slot, KeyCode::End, KeyModifiers::NONE);
    assert_eq!(line.selection(), None);
    press(&mut line, &mut slot, KeyCode::Left, KeyModifiers::CONTROL);
    assert_eq!(line.cursor().byte(), 4);
}

#[test]
fn ctrl_a_selects_all_and_typing_replaces_it() {
    let (_memory, mut slot) = slot();
    let mut line = LineEditor::new("old.txt");
    press(
        &mut line,
        &mut slot,
        KeyCode::Char('a'),
        KeyModifiers::CONTROL,
    );
    assert!(press(
        &mut line,
        &mut slot,
        KeyCode::Char('N'),
        KeyModifiers::SHIFT
    ));
    assert_eq!(line.text(), "N");
}

#[test]
fn ctrl_x_and_shift_del_cut_to_the_clipboard() {
    let (memory, mut slot) = slot();
    let mut line = LineEditor::new("keep cut");
    press(
        &mut line,
        &mut slot,
        KeyCode::Left,
        KeyModifiers::SHIFT | KeyModifiers::CONTROL,
    );
    assert!(press(
        &mut line,
        &mut slot,
        KeyCode::Char('x'),
        KeyModifiers::CONTROL
    ));
    assert_eq!((line.text(), memory.0.borrow().as_str()), ("keep ", "cut"));

    let mut line = LineEditor::new("ab");
    press(&mut line, &mut slot, KeyCode::Left, KeyModifiers::SHIFT);
    press(&mut line, &mut slot, KeyCode::Delete, KeyModifiers::SHIFT);
    assert_eq!((line.text(), memory.0.borrow().as_str()), ("a", "b"));
}

#[test]
fn ctrl_v_and_shift_insert_paste_one_line_over_the_selection() {
    let (memory, mut slot) = slot();
    "new name\r\n".clone_into(&mut memory.0.borrow_mut());
    let mut line = LineEditor::new("old");
    press(
        &mut line,
        &mut slot,
        KeyCode::Char('a'),
        KeyModifiers::CONTROL,
    );
    assert!(press(
        &mut line,
        &mut slot,
        KeyCode::Char('v'),
        KeyModifiers::CONTROL
    ));
    assert_eq!(line.text(), "new name");
    press(&mut line, &mut slot, KeyCode::Insert, KeyModifiers::SHIFT);
    assert_eq!(line.text(), "new namenew name");
}

#[test]
fn without_a_clipboard_nothing_breaks() {
    let mut slot = ClipboardSlot::default();
    let mut line = LineEditor::new("abc");
    line.select_all();
    press(
        &mut line,
        &mut slot,
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    );
    assert!(!press(
        &mut line,
        &mut slot,
        KeyCode::Char('v'),
        KeyModifiers::CONTROL
    ));
    assert!(press(
        &mut line,
        &mut slot,
        KeyCode::Char('x'),
        KeyModifiers::CONTROL
    ));
    assert_eq!(line.text(), "");
}

#[test]
fn typing_editing_and_plain_moves_still_work() {
    let (_memory, mut slot) = slot();
    let mut line = LineEditor::new("ac");
    press(&mut line, &mut slot, KeyCode::Left, KeyModifiers::NONE);
    press(&mut line, &mut slot, KeyCode::Char('b'), KeyModifiers::NONE);
    assert_eq!(line.text(), "abc");
    press(&mut line, &mut slot, KeyCode::Home, KeyModifiers::NONE);
    press(&mut line, &mut slot, KeyCode::Delete, KeyModifiers::NONE);
    press(&mut line, &mut slot, KeyCode::End, KeyModifiers::NONE);
    press(&mut line, &mut slot, KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(line.text(), "b");
    assert!(!press(
        &mut line,
        &mut slot,
        KeyCode::Char('q'),
        KeyModifiers::ALT
    ));
    assert!(!press(
        &mut line,
        &mut slot,
        KeyCode::Tab,
        KeyModifiers::NONE
    ));
}

mod paste_line;
