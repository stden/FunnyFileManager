//! Purpose: The rows view: one page of a table's rows with the range it covers,
//! while it loads, when it is the last or an empty page, when the request
//! failed, and scrolled.

use super::{answer, on_items, screen, shop_tables, texts};
use crate::models::database::{DatabaseExplorer, Page, PageEnd, Reply, Requests, View};
use crate::ui::database::rows;

fn page(rows: &[[&str; 2]], end: PageEnd) -> Reply {
    Reply::Rows(Page {
        columns: vec!["id".to_owned(), "name".to_owned()],
        rows: rows
            .iter()
            .map(|cells| cells.iter().map(|cell| (*cell).to_owned()).collect())
            .collect(),
        end,
    })
}

/// The rows view of `items`, its first page not answered yet.
fn rows_open(queue: &mut Requests) -> DatabaseExplorer {
    let mut explorer = shop_tables(queue);
    on_items(&mut explorer);
    explorer.open(queue);
    explorer
}

fn rows_lines(explorer: &DatabaseExplorer) -> Vec<String> {
    let View::Rows(rows) = explorer.view() else {
        panic!("the rows view is open")
    };
    texts(&rows::lines(rows))
}

#[test]
fn the_rows_view_titles_the_range_and_shows_a_cell_per_column() {
    let mut queue = Requests::default();
    let mut explorer = rows_open(&mut queue);
    assert_eq!(
        rows_lines(&explorer),
        ["main.items — from row 1", "Loading…"]
    );
    assert!(screen(&explorer).contains("main.items — from row 1"));

    let apple_and_pear = [["1", "apple"], ["2", "pear"]];
    answer(
        &mut explorer,
        &mut queue,
        page(&apple_and_pear, PageEnd::More),
    );
    assert_eq!(
        rows_lines(&explorer),
        [
            "main.items — rows 1–2, more follow",
            "id │ name",
            "1 │ apple",
            "2 │ pear"
        ]
    );
    assert!(screen(&explorer).contains("main.items — rows 1–2, more follow"));
}

#[test]
fn the_last_page_says_no_more_follow_and_an_empty_one_says_no_rows() {
    let mut queue = Requests::default();
    let mut explorer = rows_open(&mut queue);
    answer(
        &mut explorer,
        &mut queue,
        page(&[["7", "fig"]], PageEnd::Last),
    );
    assert_eq!(rows_lines(&explorer)[0], "main.items — rows 1–1");

    let mut queue = Requests::default();
    let mut explorer = rows_open(&mut queue);
    answer(&mut explorer, &mut queue, page(&[], PageEnd::Last));
    assert_eq!(rows_lines(&explorer), ["main.items — no rows", "id │ name"]);
}

#[test]
fn a_later_page_counts_its_rows_from_its_offset() {
    let mut queue = Requests::default();
    let mut explorer = rows_open(&mut queue);
    answer(
        &mut explorer,
        &mut queue,
        page(&[["1", "a"]], PageEnd::More),
    );
    explorer.page_next(&mut queue);
    assert_eq!(
        rows_lines(&explorer),
        ["main.items — from row 201", "Loading…"]
    );
    answer(
        &mut explorer,
        &mut queue,
        page(&[["201", "z"]], PageEnd::Last),
    );
    assert_eq!(rows_lines(&explorer)[0], "main.items — rows 201–201");
}

#[test]
fn a_failed_page_shows_the_reason_in_place_of_rows() {
    let mut queue = Requests::default();
    let mut explorer = rows_open(&mut queue);
    let request = queue.take().expect("the first page");
    assert!(queue.finish(request.id));
    explorer.receive(request.id, Err("statement timeout".to_owned()));
    assert_eq!(
        rows_lines(&explorer),
        ["main.items — from row 1", "statement timeout"]
    );
}

#[test]
fn scrolling_the_rows_moves_the_body_up_under_the_title() {
    let mut queue = Requests::default();
    let mut explorer = rows_open(&mut queue);
    let two = [["1", "apple"], ["2", "pear"]];
    answer(&mut explorer, &mut queue, page(&two, PageEnd::Last));
    explorer.move_down();
    let scrolled = ["main.items — rows 1–2", "1 │ apple", "2 │ pear"];
    assert_eq!(rows_lines(&explorer), scrolled, "the header scrolls off");
    explorer.move_down();
    assert_eq!(rows_lines(&explorer), scrolled, "two rows scroll one line");
    explorer.move_up();
    assert_eq!(rows_lines(&explorer)[1], "id │ name");
}
