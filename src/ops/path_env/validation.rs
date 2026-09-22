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
            let ch = rest.chars().next()?;
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    Some(out)
}

fn variable(text: &str) -> Option<(&str, &str)> {
    if cfg!(windows) {
        let rest = text.strip_prefix('%')?;
        let (name, tail) = rest.split_once('%')?;
        return Some((name, tail));
    }
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
    Some((&rest[..count], &rest[count..]))
}

#[cfg(test)]
mod tests;
