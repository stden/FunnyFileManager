use super::*;
use std::fs;

#[test]
fn a_fully_occupied_name_sequence_reports_failure_without_overwriting() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("file.txt");
    fs::write(&original, b"original").unwrap();
    // NTFS caps hard links per inode at 1023; shard across 11 seed inodes.
    let seeds: Vec<PathBuf> = (0..11)
        .map(|slot| {
            let seed = root.path().join(format!(".seed_{slot}"));
            fs::write(&seed, b"x").unwrap();
            seed
        })
        .collect();
    for index in 1usize..=9999 {
        fs::hard_link(
            &seeds[index / 1000],
            root.path().join(format!("file_{index}.txt")),
        )
        .unwrap();
    }
    let error = unique_destination(root.path(), "file.txt").unwrap_err();
    assert!(error.contains("after 9999 attempts"));
    assert_eq!(fs::read(original).unwrap(), b"original");
}

#[test]
fn unique_destination_skips_occupied_runs_for_extended_and_extensionless_names() {
    let root = tempfile::tempdir().unwrap();
    let seed = root.path().join("report.csv");
    fs::write(&seed, b"1").unwrap();
    for index in 1..=12 {
        fs::hard_link(&seed, root.path().join(format!("report_{index}.csv"))).unwrap();
    }
    assert_eq!(
        unique_destination(root.path(), "report.csv").unwrap(),
        root.path().join("report_13.csv")
    );

    let plain = root.path().join("README");
    fs::write(&plain, b"1").unwrap();
    for index in 1..=6 {
        fs::hard_link(&plain, root.path().join(format!("README_{index}"))).unwrap();
    }
    assert_eq!(
        unique_destination(root.path(), "README").unwrap(),
        root.path().join("README_7")
    );
}
