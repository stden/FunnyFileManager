//! Purpose: A file read for one value — `statm`, `status`, `loadavg` and
//! `uptime` — cut anywhere by the reader's cap or a process exiting mid-read
//! reads as nothing or as the whole, never as a smaller number.

use super::parse::{effective_uid, load_average, statm, uptime};
use super::tests_robustness::page;

/// A file read for one value: its fixture and its parser, the value spelled
/// for comparison.
struct OneValue {
    name: &'static str,
    bytes: &'static [u8],
    value: fn(&str) -> Option<String>,
}

const ONE_VALUE: [OneValue; 4] = [
    OneValue {
        name: "statm",
        bytes: b"812345 156672 23456 12 0 345678 0\n",
        value: |text| statm(text, page()).map(|usage| format!("{usage:?}")),
    },
    OneValue {
        name: "status",
        bytes: b"Name:\tbash\nUid:\t1000\t1001\t1002\t1003\nGid:\t100\t100\t100\t100\n",
        value: |text| effective_uid(text).map(|uid| uid.to_string()),
    },
    OneValue {
        name: "/proc/loadavg",
        bytes: b"10.52 20.58 30.59 3/1204 4242\n",
        value: |text| load_average(text).map(|load| format!("{load:?}")),
    },
    OneValue {
        name: "/proc/uptime",
        bytes: b"21126.06 667890.12\n",
        value: |text| uptime(text).map(|up| format!("{up:?}")),
    },
];

#[test]
fn a_file_cut_short_never_reads_as_a_smaller_number() {
    for file in &ONE_VALUE {
        let whole = (file.value)(&String::from_utf8_lossy(file.bytes));
        assert!(whole.is_some(), "{}", file.name);
        for end in 0..file.bytes.len() {
            let cut = (file.value)(&String::from_utf8_lossy(&file.bytes[..end]));
            assert!(
                cut.is_none() || cut == whole,
                "{} cut after {end} bytes read {cut:?}, not {whole:?}",
                file.name
            );
        }
    }
}
