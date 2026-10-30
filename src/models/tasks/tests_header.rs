//! Purpose: Tests for the header shape: the spec's table row for row, the
//! 14-row threshold, and every height from 0 to 60 for every core count.

use std::num::NonZeroUsize;

use super::{HeaderShape, BARS_PER_ROW};
use crate::models::Viewport;

fn shape(rows: u16, cores: usize) -> HeaderShape {
    HeaderShape::for_screen(cores, Viewport::new(rows))
}

#[test]
fn the_header_matches_the_design_table_row_for_row() {
    // rows, cores, cores per bar, core rows, chrome, page
    let table: [(u16, usize, Option<usize>, u16, u16, usize); 8] = [
        (24, 8, Some(1), 2, 11, 13),
        (24, 32, Some(2), 4, 13, 11),
        (50, 32, Some(1), 8, 17, 33),
        (24, 1, None, 0, 9, 15),
        (18, 8, Some(1), 2, 11, 7),
        (14, 8, None, 0, 9, 5),
        (13, 8, None, 0, 6, 7),
        (4, 8, None, 0, 6, 1),
    ];
    for (rows, cores, per_bar, cpu_rows, chrome, page) in table {
        let header = shape(rows, cores);
        let expected = (per_bar.and_then(NonZeroUsize::new), cpu_rows, chrome, page);
        let actual = (
            header.cores_per_bar(),
            header.cpu_rows(),
            header.chrome(),
            header.page(),
        );
        assert_eq!(actual, expected, "{rows} rows × {cores} CPUs");
    }
}

#[test]
fn the_summary_rows_appear_at_fourteen_rows_and_not_at_thirteen() {
    assert_eq!(shape(13, 8).summary_rows(), 0);
    assert_eq!(shape(14, 8).summary_rows(), 2);
}

#[test]
fn a_session_without_a_sample_draws_no_core_bars() {
    let header = shape(50, 0);
    assert_eq!((header.cpu_rows(), header.cores_per_bar()), (0, None));
    assert_eq!(header.summary_rows(), 2);
}

#[test]
fn every_height_leaves_a_page_and_every_core_a_bar() {
    for rows in 0..=60 {
        for cores in [0, 1, 4, 8, 32, 128] {
            let header = shape(rows, cores);
            let label = format!("{rows} rows × {cores} CPUs");
            assert!(header.page() >= 1, "{label}");
            if rows >= 7 {
                let total = usize::from(header.chrome()) + header.page();
                assert_eq!(total, usize::from(rows), "{label}");
            }
            if let Some(per_bar) = header.cores_per_bar() {
                let slots = usize::from(header.cpu_rows()) * BARS_PER_ROW;
                assert!(
                    slots * per_bar.get() >= cores,
                    "{label}: every core has a bar"
                );
                assert_eq!(
                    header.summary_rows(),
                    2,
                    "{label}: bars come with the summary"
                );
            }
        }
    }
}
