//! Purpose: Verify prompt windowing drops leading segments when no path segment is present.

use super::{window_prompt, Cells};
use crate::prompt::{Part, Segment};

fn segment(text: &str, part: Part) -> Segment {
    Segment {
        text: text.to_owned(),
        part,
    }
}

#[test]
fn window_prompt_without_a_path_drops_leading_segments_to_fit() {
    let prompt = vec![
        segment("user", Part::User),
        segment("host", Part::Host),
        segment("$ ", Part::Sigil),
    ];
    let fitted = window_prompt(prompt, Cells::new(6));
    assert_eq!(
        fitted,
        vec![segment("host", Part::Host), segment("$ ", Part::Sigil)]
    );
}
