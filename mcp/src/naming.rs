//! Purpose: Destination paths built from a caller-supplied file name, which may
//! add exactly one component to a directory already confined to the root.

use std::path::{Component, Path, PathBuf};

#[cfg(test)]
mod tests;

/// Join a caller-supplied name onto a directory that is already confined to
/// the server root — the only sanctioned way to build a path from an argument
/// the sandbox never inspected.
pub fn join_child(base: &Path, name: &str) -> Result<PathBuf, String> {
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(single)), None) => Ok(base.join(single)),
        _ => Err(format!(
            "'{name}' is not a plain file name: exactly one path component is allowed"
        )),
    }
}

const MAX_ATTEMPTS: usize = 9999;
const FAST_PROBES: usize = 4;
const BITS_PER_WORD: usize = 64;
const BITSET_WORDS: usize = (MAX_ATTEMPTS / BITS_PER_WORD) + 1;

struct OccupiedCounters([u64; BITSET_WORDS]);

impl OccupiedCounters {
    const fn new() -> Self {
        Self([0; BITSET_WORDS])
    }

    fn mark(&mut self, counter: usize) {
        if let Some(word) = self.0.get_mut(counter / BITS_PER_WORD) {
            *word |= 1_u64 << (counter % BITS_PER_WORD);
        }
    }

    fn is_free(&self, counter: usize) -> bool {
        self.0
            .get(counter / BITS_PER_WORD)
            .is_some_and(|word| word & (1_u64 << (counter % BITS_PER_WORD)) == 0)
    }
}

#[derive(Clone, Copy)]
struct NameTemplate<'a> {
    stem: &'a str,
    extension: Option<&'a str>,
}

impl<'a> NameTemplate<'a> {
    fn split(file_name: &'a str) -> Self {
        let path = Path::new(file_name);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .filter(|ext| !ext.is_empty());
        Self { stem, extension }
    }

    fn numbered_path(self, base: &Path, counter: usize) -> PathBuf {
        let name = match self.extension {
            Some(ext) => format!("{}_{counter}.{ext}", self.stem),
            None => format!("{}_{counter}", self.stem),
        };
        base.join(name)
    }

    fn parse_counter(self, entry_name: &str, limit: usize) -> Option<usize> {
        let tail = entry_name.strip_prefix(self.stem)?.strip_prefix('_')?;
        let number_part = match self.extension {
            Some(ext) => tail.strip_suffix(ext)?.strip_suffix('.')?,
            None => tail,
        };
        let is_plain_digits =
            !number_part.starts_with('0') && number_part.bytes().all(|b| b.is_ascii_digit());
        is_plain_digits
            .then(|| number_part.parse::<usize>().ok())
            .flatten()
            .filter(|&num| (1..=limit).contains(&num))
    }
}

/// Generate a unique file path to avoid overwriting existing files.
/// If `test.txt` already exists, returns `test_1.txt`, `test_2.txt`, etc.
pub fn unique_destination(base: &Path, file_name: &str) -> Result<PathBuf, String> {
    let dest = join_child(base, file_name)?;
    if dest.symlink_metadata().is_err() {
        return Ok(dest);
    }

    let template = NameTemplate::split(file_name);
    for counter in 1..=FAST_PROBES {
        let candidate = template.numbered_path(base, counter);
        if candidate.symlink_metadata().is_err() {
            return Ok(candidate);
        }
    }
    probe_with_snapshot(base, file_name, template, FAST_PROBES + 1, MAX_ATTEMPTS)
}

fn probe_with_snapshot(
    base: &Path,
    file_name: &str,
    template: NameTemplate<'_>,
    start: usize,
    max_attempts: usize,
) -> Result<PathBuf, String> {
    let occupied = scan_occupied_counters(base, template, max_attempts);
    (start..=max_attempts)
        .filter(|&counter| occupied.is_free(counter))
        .map(|counter| template.numbered_path(base, counter))
        .find(|candidate| candidate.symlink_metadata().is_err())
        .ok_or_else(|| {
            format!("could not find unique name for '{file_name}' after {max_attempts} attempts")
        })
}

fn scan_occupied_counters(
    dir: &Path,
    template: NameTemplate<'_>,
    limit: usize,
) -> OccupiedCounters {
    let mut occupied = OccupiedCounters::new();
    crate::fsops::visit_child_names(dir, |raw| {
        if let Some(counter) = raw
            .to_str()
            .and_then(|text| template.parse_counter(text, limit))
        {
            occupied.mark(counter);
        }
    });
    occupied
}

/// The final component of a resolved path, as the batch tools report it and as
/// they name the copy at the destination.
pub fn entry_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
