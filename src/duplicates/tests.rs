//! Purpose: the block search on in-memory texts: copies, whitespace, blanks, self-repeats, ranking.

use std::time::{Duration, Instant};

use super::{find, Group, Occurrence};

/// `count` lines that no other block shares, so a test sees only its own copies.
fn lines(tag: &str, count: usize) -> Vec<String> {
    (1..=count).map(|n| format!("{tag} line {n}")).collect()
}

fn text(lines: &[String]) -> String {
    lines.join("\n")
}

fn find_in(texts: &[String], min_block: usize, top: usize) -> Vec<Group> {
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    find(
        &refs,
        min_block,
        top,
        Instant::now() + Duration::from_secs(60),
    )
    .groups
}

fn at(source: usize, first_line: usize, last_line: usize) -> Occurrence {
    Occurrence {
        source,
        first_line,
        last_line,
    }
}

fn group(block_lines: usize, occurrences: Vec<Occurrence>) -> Group {
    Group {
        block_lines,
        occurrences,
    }
}

#[test]
fn identical_blocks_in_two_texts_form_one_group() {
    let block = text(&lines("alpha", 6));
    let groups = find_in(&[block.clone(), block], 6, 10);
    assert_eq!(
        groups,
        [group(6, vec![at(0, 1, 6), at(1, 1, 6)])],
        "one block, both copies"
    );
}

#[test]
fn whitespace_does_not_hide_a_copy() {
    let plain = lines("beta", 6);
    let respaced: Vec<String> = plain
        .iter()
        .map(|line| format!("    {}", line.replace(' ', "\t  ")))
        .collect();
    let groups = find_in(&[text(&plain), text(&respaced)], 6, 10);
    assert_eq!(groups, [group(6, vec![at(0, 1, 6), at(1, 1, 6)])]);
}

#[test]
fn blank_lines_do_not_break_a_match() {
    let plain = lines("gamma", 6);
    let with_blanks: Vec<String> = plain
        .iter()
        .flat_map(|line| [line.clone(), String::new()])
        .collect();
    let groups = find_in(&[text(&plain), text(&with_blanks)], 6, 10);
    assert_eq!(
        groups,
        [group(6, vec![at(0, 1, 6), at(1, 1, 11)])],
        "the copy ends on its last non-blank line, 11"
    );
}

#[test]
fn a_block_is_reported_at_its_full_length() {
    let block = text(&lines("delta", 12));
    let groups = find_in(&[block.clone(), block], 6, 10);
    assert_eq!(groups, [group(12, vec![at(0, 1, 12), at(1, 1, 12)])]);
}

#[test]
fn a_block_repeating_itself_by_one_line_is_not_reported_twice() {
    let run = ["same"; 7].join("\n");
    assert_eq!(
        find_in(std::slice::from_ref(&run), 6, 10),
        [],
        "one place per copy"
    );
    let groups = find_in(&[run.clone(), run], 6, 10);
    assert!(
        groups.contains(&group(7, vec![at(0, 1, 7), at(1, 1, 7)])),
        "{groups:?}"
    );
}

#[test]
fn a_shared_block_is_one_group_with_every_copy() {
    let block = text(&lines("eps", 6));
    let groups = find_in(&[block.clone(), block.clone(), block], 6, 10);
    assert_eq!(
        groups,
        [group(6, vec![at(0, 1, 6), at(1, 1, 6), at(2, 1, 6)])]
    );
}

#[test]
fn a_second_copy_inside_one_text_counts_when_it_does_not_overlap() {
    let block = lines("zeta", 6);
    let mut twice = block.clone();
    twice.push("separator".to_owned());
    twice.extend(block.clone());
    let groups = find_in(&[text(&twice), text(&block)], 6, 10);
    assert_eq!(
        groups,
        [group(6, vec![at(0, 1, 6), at(0, 8, 13), at(1, 1, 6)])]
    );
}

#[test]
fn groups_rank_by_excess_lines_then_block_length() {
    let mut texts = Vec::new();
    let alpha = text(&lines("alpha", 10));
    texts.extend([alpha.clone(), alpha]);
    let gamma = text(&lines("gamma", 3));
    texts.extend([gamma.clone(), gamma.clone(), gamma]);
    let beta = text(&lines("beta", 4));
    texts.extend([beta.clone(), beta]);
    let ranked = find_in(&texts, 3, 10);
    let sizes: Vec<(usize, usize)> = ranked
        .iter()
        .map(|found| (found.block_lines, found.occurrences.len()))
        .collect();
    assert_eq!(
        sizes,
        [(10, 2), (3, 3), (4, 2)],
        "excess 10, then 6, then 4"
    );
}

#[test]
fn equal_excess_ranks_the_longer_block_first() {
    let six = text(&lines("six", 6));
    let three = text(&lines("three", 3));
    let texts = [six.clone(), six, three.clone(), three.clone(), three];
    let ranked = find_in(&texts, 3, 10);
    let lengths: Vec<usize> = ranked.iter().map(|found| found.block_lines).collect();
    assert_eq!(lengths, [6, 3], "both excess 6; the 6-line block leads");
}

#[test]
fn only_the_top_groups_are_kept() {
    let mut texts = Vec::new();
    for tag in ["one", "two", "three"] {
        let block = text(&lines(tag, 4));
        texts.extend([block.clone(), block]);
    }
    let ranked = find_in(&texts, 4, 2);
    assert_eq!(ranked.len(), 2);
    assert_eq!(
        ranked.first().map(|found| found.occurrences[0].source),
        Some(0)
    );
}

#[test]
fn nothing_is_reported_without_a_repeat_or_a_usable_size() {
    let unique = text(&lines("unique", 8));
    assert_eq!(find_in(std::slice::from_ref(&unique), 6, 10), []);
    assert_eq!(find_in(&[], 6, 10), []);
    assert_eq!(find_in(&[unique.clone(), unique.clone()], 0, 10), []);
    assert_eq!(find_in(&[unique.clone(), unique], 6, 0), []);
}
