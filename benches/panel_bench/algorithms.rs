//! Purpose: Algorithmic microbenchmarks for broad-prefix quick search, tree
//! structure navigation, Unicode editor search, and `ThingsCatalog` updates.

use std::hint::black_box;
use std::path::PathBuf;

use criterion::Criterion;
use funny_file_manager::models::{things::ThingsCatalog, FolderTree, RowMotion};
use funny_file_manager::search::{Caret, Case, Direction, Needle, Search, WordMatch};
use funny_file_manager::things::{Listing, Machine};

use super::panel;

pub fn bench_algorithms(c: &mut Criterion) {
    bench_broad_prefix(c);
    bench_tree_ops(c);
    bench_unicode_search(c);
    bench_things_catalog(c);
}

fn bench_broad_prefix(c: &mut Criterion) {
    let mut group = c.benchmark_group("algo/panel_broad_prefix_10k");
    let mut p = panel(10_000);
    assert!(p.find_first_matching("d"));
    group.bench_function("first", |b| {
        b.iter(|| black_box(p.find_first_matching(black_box("d"))));
    });
    group.bench_function("next_from_mid", |b| {
        b.iter(|| {
            p.select(Some(5_000));
            black_box(p.find_next_matching(black_box("d")))
        });
    });
    group.finish();
}

fn build_deep_tree() -> FolderTree {
    let mut tree = FolderTree::rooted_at(PathBuf::from("/workspace"));
    tree.expand(&mut |dir| (0..50).map(|g| dir.join(format!("group-{g:02}"))).collect());
    for g in (1..=50).rev() {
        tree.cursor = g;
        tree.expand(&mut |dir| (0..100).map(|i| dir.join(format!("leaf-{i:04}"))).collect());
    }
    tree
}

fn bench_tree_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("algo/tree_ops_5k");
    let mut tree = build_deep_tree();
    assert!(tree.find_first("leaf-0099"));
    group.bench_function("find_next", |b| {
        b.iter(|| black_box(tree.find_next(black_box("leaf-0099"))));
    });
    group.bench_function("nav_parent_siblings", |b| {
        b.iter(|| {
            tree.cursor = 2_550;
            let up = tree.move_to_parent();
            let next = tree.next_sibling();
            let prev = tree.previous_sibling();
            black_box((up, next, prev))
        });
    });
    group.finish();
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: search needles are non-empty"
)]
fn bench_unicode_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("algo/editor_search_unicode_50k");
    let ascii: Vec<String> = (0..50_000)
        .map(|i| format!("00{i:05} INFO routine_work background activity in module handler"))
        .collect();
    let unicode: Vec<String> = (0..50_000)
        .map(|i| format!("00{i:05} — Ελληνικά Русский 日本語 Straße routine_work line"))
        .collect();
    let cyrillic = Search::new(Needle::new("привітсвіт").expect("needle"));
    let exact_word = Search {
        needle: Needle::new("routine").expect("needle"),
        case: Case::Match,
        words: WordMatch::WholeWords,
        direction: Direction::Forward,
    };
    group.bench_function("non_ascii_needle_on_ascii", |b| {
        b.iter(|| black_box(cyrillic.find(black_box(&ascii), Caret::new(0, 0))));
    });
    group.bench_function("non_ascii_needle_on_unicode", |b| {
        b.iter(|| black_box(cyrillic.find(black_box(&unicode), Caret::new(0, 0))));
    });
    group.bench_function("exact_whole_word_on_unicode", |b| {
        b.iter(|| black_box(exact_word.find(black_box(&unicode), Caret::new(0, 0))));
    });
    group.finish();
}

fn bench_things_catalog(c: &mut Criterion) {
    let machines: Vec<Machine> = (0..2_000)
        .map(|i| Machine {
            id: format!("id-{i}"),
            name: format!("Web-{i:04}"),
            hostname: format!("web-{i:04}.prod.example.com"),
            ip_address: Some(format!("10.0.{}.{}", i / 256, i % 256)),
            environment: Some("PROD".to_owned()),
            location: Some("Frankfurt".to_owned()),
            tags: vec!["Database".to_owned(), "Ingress".to_owned()],
            ..Machine::default()
        })
        .collect();
    let mut catalog = ThingsCatalog::new(Listing {
        machines,
        total: Some(2_000),
    });
    for ch in "web".chars() {
        catalog.type_filter(ch);
    }
    c.bench_function("algo/things_cursor_and_shown_2k", |b| {
        b.iter(|| {
            catalog.move_cursor(RowMotion::Next);
            black_box(catalog.shown().len())
        });
    });
}
