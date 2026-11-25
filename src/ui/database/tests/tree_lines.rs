//! Purpose: The tree's lines: each row indented by depth and marked open or
//! closed, a column's type and key, the row waiting for an answer, and why a
//! connection failed.

use ratatui::style::Modifier;

use super::{answer, item_columns, on_items, shop, shop_tables, texts};
use crate::models::database::{DatabaseExplorer, Requests};
use crate::ui::database::tree;

#[test]
fn the_tree_indents_marks_and_describes_each_row() {
    let mut queue = Requests::default();
    let mut explorer = shop_tables(&mut queue);
    on_items(&mut explorer);
    explorer.expand(&mut queue);
    answer(&mut explorer, &mut queue, item_columns());

    let lines = tree::lines(&explorer);
    assert_eq!(
        texts(&lines),
        [
            "▾ shop (sqlite)",
            "  ▾ main",
            "    ▾ items",
            "      id INTEGER PK NOT NULL",
            "      note TEXT",
            "    ▸ item_labels [view]",
        ]
    );
    let reversed: Vec<bool> = lines
        .iter()
        .map(|line| line.style.add_modifier.contains(Modifier::REVERSED))
        .collect();
    assert_eq!(reversed, [false, false, true, false, false, false]);
}

#[test]
fn a_row_still_waiting_for_its_answer_shows_loading() {
    let mut queue = Requests::default();
    let mut explorer = shop_tables(&mut queue);
    on_items(&mut explorer);
    explorer.expand(&mut queue);
    assert_eq!(texts(&tree::lines(&explorer))[2], "    ▾ items Loading…");
}

#[test]
fn a_connection_that_failed_shows_why() {
    let mut queue = Requests::default();
    let mut explorer = DatabaseExplorer::new(vec![shop()], None);
    explorer.expand(&mut queue);
    let request = queue.take().expect("the schemas request");
    assert!(queue.finish(request.id));
    explorer.receive(request.id, Err("unable to open".to_owned()));
    assert_eq!(
        texts(&tree::lines(&explorer)),
        ["▾ shop (sqlite) — unable to open"]
    );
}
