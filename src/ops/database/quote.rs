//! Purpose: Quote one identifier for both drivers: double quotes around it, with
//! each double quote inside it doubled. This is the only place a database name
//! is put into SQL text.

#[cfg(test)]
mod tests;

pub(super) fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
