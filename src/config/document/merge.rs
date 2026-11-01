//! Purpose: The three-way merge a settings save makes, so that it writes what
//! the session changed and keeps what someone else changed in the file.
//!
//! All three sides are the saving writer's own settings as its type writes
//! them, so a file that spells a value differently (`8` for `'8'`, a page
//! left out for its default) compares equal to it. Only the file itself is
//! written to, and only where the session changed something.

use serde_norway::{Mapping, Value};

/// One writer's settings at one level of the file.
pub struct Sides<'a> {
    /// As the file holds them now.
    pub on_disk: &'a Mapping,
    /// As the writer loaded them: the file then, or the writer's defaults.
    pub loaded: &'a Mapping,
    /// As the writer holds them now.
    pub current: &'a Mapping,
}

/// Bring the entries of `file` the session changed to their current value.
///
/// An entry the session did not change keeps the file's value, and so does
/// one the file already holds. One changed only in the session is replaced
/// (or removed). One changed both in the session and in the file is merged
/// key by key when all three sides are mappings; otherwise the error is the
/// dotted path to it, and `file` must not be written.
pub fn merge(file: &mut Mapping, sides: &Sides<'_>) -> Result<(), String> {
    let removed = sides
        .loaded
        .keys()
        .filter(|key| !sides.current.contains_key(*key));
    for key in sides.current.keys().chain(removed) {
        let current = sides.current.get(key);
        let loaded = sides.loaded.get(key);
        let on_disk = sides.on_disk.get(key);
        if current == loaded || current == on_disk {
            continue;
        }
        if on_disk == loaded {
            replace(file, key, current);
            continue;
        }
        merge_entry(file, key, [current, loaded, on_disk]).map_err(|below| {
            let name = key.as_str().unwrap_or("?");
            if below.is_empty() {
                name.to_owned()
            } else {
                format!("{name}.{below}")
            }
        })?;
    }
    Ok(())
}

/// Set `key` of `file` to `value`, or remove it for `None`; the other keys
/// keep their order.
fn replace(file: &mut Mapping, key: &Value, value: Option<&Value>) {
    match value {
        Some(value) => {
            file.insert(key.clone(), value.clone());
        }
        None => {
            file.shift_remove(key);
        }
    }
}

/// Merge the entry `key`, changed both in the session and in the file, key by
/// key. The error is the path below `key`: empty when the entry itself
/// conflicts.
fn merge_entry(file: &mut Mapping, key: &Value, sides: [Option<&Value>; 3]) -> Result<(), String> {
    let [Some(Value::Mapping(current)), Some(Value::Mapping(loaded)), Some(Value::Mapping(on_disk))] =
        sides
    else {
        return Err(String::new());
    };
    let Value::Mapping(entries) = file
        .entry(key.clone())
        .or_insert_with(|| Value::Mapping(Mapping::new()))
    else {
        return Err(String::new());
    };
    merge(
        entries,
        &Sides {
            on_disk,
            loaded,
            current,
        },
    )
}
