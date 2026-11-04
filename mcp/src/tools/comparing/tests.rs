use super::*;

#[test]
fn compare_file_pair_propagates_open_errors_and_hashes_multi_block_divergences() {
    let dir = tempfile::tempdir().unwrap();
    let left = dir.path().join("left.bin");
    let right = dir.path().join("right.bin");
    let missing = dir.path().join("absent.bin");

    let left_bytes = vec![b'a'; 64 * 1024];
    let mut right_bytes = left_bytes.clone();
    right_bytes[40_000] = b'z';
    fs::write(&left, &left_bytes).unwrap();
    fs::write(&right, &right_bytes).unwrap();

    assert!(compare_file_pair(&missing, &right).is_err());
    assert!(compare_file_pair(&left, &missing).is_err());

    let comparison = compare_file_pair(&left, &right).unwrap();
    let ComparisonOutcome::Different {
        offset,
        digests: Some(PairDigests {
            left_sha256,
            right_sha256,
        }),
    } = comparison.outcome
    else {
        panic!("expected equal-size divergence with both SHA-256 sums");
    };
    assert_eq!(offset, 40_000);
    assert_eq!(left_sha256, crate::checksum::checksum_file(&left).unwrap());
    assert_eq!(
        right_sha256,
        crate::checksum::checksum_file(&right).unwrap()
    );
}

#[test]
fn compare_file_pair_reports_single_side_eof_offset_without_hashing() {
    let dir = tempfile::tempdir().unwrap();
    let prefix = dir.path().join("prefix.bin");
    let longer = dir.path().join("longer.bin");
    fs::write(&prefix, b"abcdef").unwrap();
    fs::write(&longer, b"abcdef_extra").unwrap();

    let comparison = compare_file_pair(&prefix, &longer).unwrap();
    assert!(matches!(
        comparison.outcome,
        ComparisonOutcome::Different {
            offset: 6,
            digests: None
        }
    ));
}
