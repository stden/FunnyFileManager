//! Purpose: Guard out-of-bounds bitset slot recording in `TakenSlots`.

use super::{TakenSlots, BITSET_WORDS, BITS_PER_WORD};

#[test]
fn taken_slots_ignores_out_of_range_slot_indices() {
    let mut slots = TakenSlots::new();
    let out_of_range = BITSET_WORDS * BITS_PER_WORD;
    slots.record(out_of_range);
    assert!(!slots.contains(out_of_range));
}
