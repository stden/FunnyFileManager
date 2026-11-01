//! Negative: code that hides what it does.

/// A name shown to the user.
pub struct Name;

impl std::fmt::Display for Name {
    // The `Formatter` borrows; the signature hides it.
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("name")
    }
}

/// `String` is in the prelude; the path says nothing.
#[must_use]
pub fn buffer() -> String {
    std::string::String::with_capacity(8)
}

fn copied(text: &str) -> String {
    text.to_owned()
}

/// The copy is freed right here, which `let _ =` hides.
pub fn discard(text: &str) {
    let _ = copied(text);
}
