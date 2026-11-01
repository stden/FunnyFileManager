//! Purpose: Given and Then steps that write a file in one of the editor's
//! encodings, and check a file's bytes written out as hex, so a scenario can
//! hold bytes that are not UTF-8 without trusting the encoder under test.

use funny_file_manager::encoding::TextEncoding;

use super::*;

/// The encoding the editor shows under `label`, as its dialog lists it.
#[expect(
    clippy::expect_used,
    reason = "a scenario naming an encoding the editor does not offer is a test bug"
)]
fn encoding(label: &str) -> TextEncoding {
    TextEncoding::ALL
        .into_iter()
        .find(|encoding| encoding.label() == label)
        .expect("an encoding the editor offers")
}

/// The file's bytes: `text` as `label` writes it.
#[expect(
    clippy::expect_used,
    reason = "a step whose text the encoding cannot hold is a test bug"
)]
fn bytes(text: &str, label: &str) -> Vec<u8> {
    encoding(label)
        .encode(text)
        .expect("text the encoding can hold")
}

/// Bytes written as hex pairs separated by spaces, as a hex dump shows them.
#[expect(
    clippy::expect_used,
    reason = "a scenario with a byte that is not two hex digits is a test bug"
)]
fn hex_bytes(hex: &str) -> Vec<u8> {
    hex.split_whitespace()
        .map(|pair| u8::from_str_radix(pair, 16).expect("a hex byte"))
        .collect()
}

/// A fixture written in `label`; the scenario's own expectations are literal
/// bytes, so the encoder only has to make the fixture the editor can read.
#[given(expr = "the {word} file {string} is {string} in {string}")]
#[expect(
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "a Cucumber step fails by panicking; Cucumber dictates the world as `&mut`"
)]
fn encoded_fixture(world: &mut TuiWorld, side: String, file: String, text: String, label: String) {
    fs::write(world.path(&side, &file), bytes(&text, &label)).expect("fixture file");
}

#[then(expr = "the {word} file {string} holds the bytes {string}")]
#[expect(
    clippy::expect_used,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "a Cucumber step fails by panicking; Cucumber dictates the world as `&mut`"
)]
fn file_holds_bytes(world: &mut TuiWorld, side: String, file: String, hex: String) {
    assert_eq!(
        fs::read(world.path(&side, &file)).expect("file"),
        hex_bytes(&hex)
    );
}
