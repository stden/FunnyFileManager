//! Purpose: Typing replaces an offered mask and then appends ordinary
//! characters, without changing which group key opened the dialog.

use super::*;

#[test]
fn typed_characters_replace_the_offer_and_keep_the_selection_purpose() {
    let mut prompt = MaskPrompt::new(MaskPurpose::Select, "*.rs");
    prompt.push('界');
    prompt.push('*');
    assert_eq!(prompt.mask(), "界*");
    assert!(!prompt.is_offered());
    assert_eq!(prompt.purpose(), MaskPurpose::Select);
}
