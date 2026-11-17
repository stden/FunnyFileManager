//! Unit tests for `WindowsReader::sample_with` system and process-list error mapping.

use super::*;

#[test]
fn sample_with_maps_system_and_process_list_failures() {
    let mut reader = WindowsReader::new();
    assert!(matches!(
        reader.sample_with(
            || Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            list_processes,
        ),
        Err(SampleError::Unreadable {
            part: SystemPart::CpuTimes,
            kind: io::ErrorKind::PermissionDenied,
        })
    ));
    assert!(matches!(
        reader.sample_with(read_system_sample, |_| {
            Err(io::Error::from(io::ErrorKind::NotFound))
        }),
        Err(SampleError::Unreadable {
            part: SystemPart::ProcessList,
            kind: io::ErrorKind::NotFound,
        })
    ));
}
