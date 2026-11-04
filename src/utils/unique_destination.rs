//! Purpose: Find the lowest-numbered free sibling file name using a stack bitset.

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use super::name_is_taken;

const MAX_UNIQUE_ATTEMPTS: usize = 9999;
const FAST_PROBE_LIMIT: usize = 4;
const BITS_PER_WORD: usize = 64;
const BITSET_WORDS: usize = (MAX_UNIQUE_ATTEMPTS / BITS_PER_WORD) + 1;

struct TakenSlots([u64; BITSET_WORDS]);

impl TakenSlots {
    const fn new() -> Self {
        Self([0; BITSET_WORDS])
    }

    const fn bit_pos(slot: usize) -> (usize, u64) {
        (slot / BITS_PER_WORD, 1_u64 << (slot % BITS_PER_WORD))
    }

    fn record(&mut self, slot: usize) {
        let (word_idx, mask) = Self::bit_pos(slot);
        if let Some(word) = self.0.get_mut(word_idx) {
            *word |= mask;
        }
    }

    fn contains(&self, slot: usize) -> bool {
        let (word_idx, mask) = Self::bit_pos(slot);
        self.0.get(word_idx).is_some_and(|word| (word & mask) != 0)
    }
}

#[derive(Clone, Copy)]
struct DestinationTemplate<'a> {
    stem: &'a str,
    ext: Option<&'a str>,
}

impl<'a> DestinationTemplate<'a> {
    fn from_os_name(file_name: &'a OsStr) -> Self {
        let path = Path::new(file_name);
        let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or("");
        let ext = path
            .extension()
            .and_then(OsStr::to_str)
            .filter(|e| !e.is_empty());
        Self { stem, ext }
    }

    fn candidate_in(self, base: &Path, slot: usize) -> PathBuf {
        let file_name = match self.ext {
            Some(ext) => format!("{}_{slot}.{ext}", self.stem),
            None => format!("{}_{slot}", self.stem),
        };
        base.join(file_name)
    }
}

/// Generate a unique file path to avoid overwriting existing files.
/// If "test.txt" already exists, returns "`test_1.txt`", "`test_2.txt`", etc.
pub fn unique_destination(base: &Path, file_name: &OsStr) -> io::Result<PathBuf> {
    probe_unique_destination(base, file_name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "could not find unique name for {} after {MAX_UNIQUE_ATTEMPTS} attempts",
                file_name.display()
            ),
        )
    })
}

fn probe_unique_destination(base: &Path, file_name: &OsStr) -> Option<PathBuf> {
    let dest = base.join(file_name);
    if !name_is_taken(&dest) {
        return Some(dest);
    }
    let template = DestinationTemplate::from_os_name(file_name);
    if let Some(path) = (1..=FAST_PROBE_LIMIT)
        .map(|slot| template.candidate_in(base, slot))
        .find(|path| !name_is_taken(path))
    {
        return Some(path);
    }
    let taken = scan_taken_slots(base, template);
    ((FAST_PROBE_LIMIT + 1)..=MAX_UNIQUE_ATTEMPTS)
        .filter(|&slot| !taken.contains(slot))
        .map(|slot| template.candidate_in(base, slot))
        .find(|path| !name_is_taken(path))
}

fn scan_taken_slots(base: &Path, template: DestinationTemplate<'_>) -> TakenSlots {
    let mut slots = TakenSlots::new();
    for entry in fs::read_dir(base).into_iter().flatten().flatten() {
        if let Some(slot) = entry
            .file_name()
            .to_str()
            .and_then(|name| parse_slot(name, template))
        {
            slots.record(slot);
        }
    }
    slots
}

fn parse_slot(name: &str, template: DestinationTemplate<'_>) -> Option<usize> {
    let after_stem = name.strip_prefix(template.stem)?.strip_prefix('_')?;
    let digits = match template.ext {
        Some(ext) => after_stem.strip_suffix(ext)?.strip_suffix('.')?,
        None => after_stem,
    };
    let canonical = !digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit());
    canonical
        .then(|| digits.parse::<usize>().ok())
        .flatten()
        .filter(|&n| (1..=MAX_UNIQUE_ATTEMPTS).contains(&n))
}
