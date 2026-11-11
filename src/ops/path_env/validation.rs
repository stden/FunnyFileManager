use crate::path_env::Validation;
use std::path::Path;

pub fn validate(text: &str) -> Validation {
    if text.is_empty() {
        return Validation::Empty;
    }
    let Some(expanded) = expand(text, |name| std::env::var(name).ok()) else {
        return Validation::Unresolved;
    };
    let path = Path::new(&expanded);
    if !path.is_absolute() {
        return Validation::Relative;
    }
    match path.metadata() {
        Ok(metadata) if metadata.is_dir() => Validation::Directory,
        Ok(_) => Validation::NotDirectory,
        Err(_) => Validation::Missing,
    }
}

pub(super) fn expand(text: &str, resolve: impl Fn(&str) -> Option<String>) -> Option<String> {
    let mut rest = text;
    let mut out = String::new();
    if let Some(tail) = rest.strip_prefix("~/") {
        out.push_str(&resolve("HOME")?);
        out.push('/');
        rest = tail;
    }
    while !rest.is_empty() {
        if let Some((name, tail)) = variable(rest) {
            out.push_str(&resolve(name)?);
            rest = tail;
        } else {
            let mut chars = rest.chars();
            out.push(chars.next()?);
            rest = chars.as_str();
        }
    }
    Some(out)
}

#[cfg(windows)]
fn variable(text: &str) -> Option<(&str, &str)> {
    windows_variable(text)
}

#[cfg(not(windows))]
fn variable(text: &str) -> Option<(&str, &str)> {
    unix_variable(text)
}

#[cfg(any(windows, test))]
pub(super) fn windows_variable(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix('%')?;
    rest.split_once('%')
}

#[cfg(any(not(windows), test))]
pub(super) fn unix_variable(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix('$')?;
    if let Some(rest) = rest.strip_prefix('{') {
        return rest.split_once('}');
    }
    let count = rest
        .bytes()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == b'_')
        .count();
    if count == 0 {
        return None;
    }
    rest.split_at_checked(count)
}

#[cfg(test)]
mod tests;
