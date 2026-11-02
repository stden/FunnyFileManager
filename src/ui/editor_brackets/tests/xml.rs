//! Purpose: Unit tests for XML/HTML paired-tag highlighting and scanning (`def:PairStart`/`def:PairEnd`).

use super::super::xml::{matching_xml_pair, TagSpan};
use super::*;

fn xml_editor(text: &str, line: usize, col: usize) -> Editor {
    let mut ed = Editor::from_content("pom.xml".into(), PathBuf::from("pom.xml"), text);
    ed.cursor_line = line;
    ed.cursor_col = col;
    ed
}

#[test]
fn xml_tag_pair_highlights_both_opening_and_closing_tags_in_bright_red() {
    let line = "  <fb-contrib.version>7.7.4</fb-contrib.version>";
    let ed = xml_editor(line, 0, 44);
    let mut terminal = Terminal::new(TestBackend::new(60, 1)).unwrap();
    terminal
        .draw(|frame| render(frame, &ed, frame.area(), 0, Cells::new(0)))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let red: Vec<u16> = (0..60)
        .filter(|&x| buffer[(x, 0)].fg == super::super::super::palette::CODE_URL)
        .collect();
    let expected: Vec<u16> = (2..22).chain(27..48).collect();
    assert_eq!(red, expected);
}

#[test]
fn multiline_xml_opening_tag_with_attributes_pairs_with_closing_tag() {
    let xml = "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"\n         xsi:schemaLocation=\"x\">\n</project>";
    let spans = matching_xml_pair(&xml_editor(xml, 0, 0)).unwrap();
    assert_eq!(spans[0], TagSpan::new(0, 0, 8));
    assert_eq!(spans[1], TagSpan::new(2, 0, 10));
}

#[test]
fn self_closing_xml_tag_pairs_opening_prefix_with_slash_gt_from_both_ends() {
    let xml = "<plugin\n  attr=\"val\"/>";
    let expected = [TagSpan::new(0, 0, 7), TagSpan::new(1, 12, 14)];
    assert_eq!(matching_xml_pair(&xml_editor(xml, 0, 2)), Some(expected));
    assert_eq!(matching_xml_pair(&xml_editor(xml, 1, 13)), Some(expected));
}

#[test]
fn xml_scanner_ignores_comments_pis_doctypes_quoted_brackets_and_malformed_tags() {
    let xml = concat!(
        "<?xml version=\"1.0\"?>\n",
        "<!DOCTYPE project>\n",
        "<!-- <root> ignored\n",
        "     still in comment -->\n",
        "<root a=\"x>y\" b='z/>w'>\n",
        "  <![CDATA[ a > b </root> ]]>\n",
        "  a < b </> </bad attr> <unclosed>\n",
        "  <item><item>val</item></item>\n",
        "</root >"
    );
    let root_pair = [TagSpan::new(4, 0, 5), TagSpan::new(8, 0, 8)];
    assert_eq!(matching_xml_pair(&xml_editor(xml, 4, 1)), Some(root_pair));
    assert_eq!(matching_xml_pair(&xml_editor(xml, 8, 2)), Some(root_pair));

    let outer_item = [TagSpan::new(7, 2, 8), TagSpan::new(7, 24, 31)];
    assert_eq!(matching_xml_pair(&xml_editor(xml, 7, 3)), Some(outer_item));
}

#[test]
fn cursor_just_after_tag_matches_only_when_followed_by_whitespace_or_eol() {
    let text = "<tag>body</tag> ";
    // Col 5 is right after `<tag>`, where `b` is not whitespace -> does not match.
    assert_eq!(matching_xml_pair(&xml_editor(text, 0, 5)), None);
    // Col 15 is right after `</tag>`, followed by space -> matches!
    assert!(matching_xml_pair(&xml_editor(text, 0, 15)).is_some());
    // Plain text line without `<` or `/>` returns None immediately.
    assert_eq!(matching_xml_pair(&xml_editor("plain text", 0, 0)), None);
}

#[test]
fn xml_tag_paint_clips_vertically_and_horizontally_scrolled_spans() {
    let xml = "<tag>\n  body\n</tag>";
    let ed = xml_editor(xml, 0, 1);
    let mut terminal = Terminal::new(TestBackend::new(4, 2)).unwrap();
    // top = 0, height = 2 -> line 2 (`</tag>`) is below the viewport; skip = 2 clips `<t` and leaves `ag>`.
    terminal
        .draw(|frame| render(frame, &ed, frame.area(), 0, Cells::new(2)))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let red_row0: Vec<u16> = (0..4)
        .filter(|&x| buffer[(x, 0)].fg == super::super::super::palette::CODE_URL)
        .collect();
    assert_eq!(red_row0, vec![0, 1, 2]);
}

#[test]
fn cursor_outside_tag_on_line_with_angle_brackets_skips_full_scan() {
    let line = "    <item>7.7.4</item>  ";
    assert_eq!(matching_xml_pair(&xml_editor(line, 0, 0)), None);
    assert_eq!(matching_xml_pair(&xml_editor(line, 0, 12)), None);
    assert_eq!(matching_xml_pair(&xml_editor(line, 0, 23)), None);
    assert_eq!(
        matching_xml_pair(&xml_editor("if a < b && c > d {}", 0, 5)),
        None
    );
}

#[test]
fn xml_scanner_bounds_search_window_around_cursor_line() {
    let mut lines = vec!["<far>".to_owned()];
    lines.extend((0..1_050).map(|_| "  <item>ok</item>".to_owned()));
    lines.push("</far>".to_owned());
    let text = lines.join("\n");
    // `<far>` at line 0 and `</far>` at line 1051 are more than MAX_SCAN_LINES apart.
    assert_eq!(matching_xml_pair(&xml_editor(&text, 1_051, 1)), None);
    // `<item>` on line 1050 still pairs inside the window.
    assert_eq!(
        matching_xml_pair(&xml_editor(&text, 1_050, 3)),
        Some([TagSpan::new(1_050, 2, 8), TagSpan::new(1_050, 10, 17)])
    );
}
