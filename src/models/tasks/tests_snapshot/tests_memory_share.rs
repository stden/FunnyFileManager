//! Purpose: unit tests for memory shares: resident over total, at most full, and
//! unknown without a nonzero total.

use super::*;

fn resident(raw: u32, kib_resident: u64) -> ProcessSample {
    ProcessSample {
        memory: Some(MemoryUsage {
            virtual_size: kib(kib_resident * 2),
            resident: kib(kib_resident),
            shared: None,
        }),
        ..process(raw, "cache")
    }
}

fn with_memory_total(total_kib: u64, processes: Vec<ProcessSample>) -> Sample {
    let mut sample = first(processes);
    sample.system.memory = Some(MemorySample {
        total: kib(total_kib),
        free: kib(0),
        available: kib(0),
        buffers: kib(0),
        cached: kib(0),
        reclaimable: kib(0),
        shared: kib(0),
    });
    sample
}

#[test]
fn memory_share_is_resident_over_total_and_at_most_full() {
    let processes = vec![
        resident(1, 250),
        resident(2, 4_000),
        process(3, "unmeasured"),
    ];
    let (snapshot, _) =
        Snapshot::derive(&Baselines::default(), with_memory_total(1_000, processes));
    let shares: Vec<Option<Percent>> = snapshot
        .rows()
        .iter()
        .map(ProcessRow::memory_share)
        .collect();
    let expected = [
        Some(Percent::from_hundredths(2_500)),
        Some(Percent::FULL),
        None,
    ];
    assert_eq!(shares, expected);
}

#[test]
fn memory_share_is_unknown_without_a_nonzero_total() {
    let without = first(vec![resident(1, 250)]);
    let empty = with_memory_total(0, vec![resident(1, 250)]);
    for sample in [without, empty] {
        let (snapshot, _) = Snapshot::derive(&Baselines::default(), sample);
        assert_eq!(snapshot.rows()[0].memory_share(), None);
    }
}
