use super::*;
use tempfile::TempDir;

#[test]
fn totals_rank_frequent_waits_above_a_single_slow_action() {
    let mut stats = Performance::default();
    for _ in 0..3 {
        stats.record(
            "Copy/Enter",
            Duration::from_millis(8),
            Duration::from_millis(2),
        );
    }
    stats.record("View", Duration::from_millis(20), Duration::ZERO);
    let rows = stats.rows();
    assert_eq!(rows[0].action, "Copy/Enter");
    assert_eq!(rows[0].count, 3);
    assert!((rows[0].total_ms - 30.0).abs() < 0.001);
    assert!((rows[0].mean_ms - 10.0).abs() < 0.001);
    assert!((rows[0].max_ms - 10.0).abs() < 0.001);
    assert!((rows[0].render_ms - 6.0).abs() < 0.001);
    assert!(stats.report()[3].contains("Copy/Enter"));
}

#[test]
fn reports_are_saved_atomically_to_an_explicit_path() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("performance/session.json");
    let mut stats = Performance::default();
    stats.record(
        "Editor/Insert",
        Duration::from_millis(2),
        Duration::from_millis(1),
    );
    stats.save_to(&path).expect("save");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("JSON");
    assert_eq!(report["version"], 1);
    assert_eq!(report["actions"][0]["total_ms"], 3.0);
    stats.save_to(&path).expect("replace");
    assert_eq!(
        std::fs::read_dir(path.parent().expect("parent"))
            .expect("entries")
            .count(),
        1
    );
}

#[test]
fn an_empty_report_and_an_unwritable_destination_are_explicit() {
    let temp = TempDir::new().expect("tempdir");
    let stats = Performance::default();
    assert!(stats
        .report()
        .iter()
        .any(|line| line.contains("No actions")));
    assert!(stats.save_to(temp.path()).is_err());
}

#[test]
fn separate_sessions_keep_separate_reports_and_directory_errors_are_returned() {
    let temp = TempDir::new().expect("config directory");
    let mut stats = Performance::default();
    stats.record("View", Duration::from_millis(20), Duration::ZERO);
    let first = std::time::UNIX_EPOCH + Duration::from_secs(100);
    stats
        .save_session_in(Some(temp.path()), first)
        .expect("first session");
    stats
        .save_session_in(Some(temp.path()), first + Duration::from_micros(1))
        .expect("second session");
    let paths: Vec<_> = std::fs::read_dir(temp.path().join("performance"))
        .expect("reports")
        .collect();
    assert_eq!(paths.len(), 2);
    for entry in paths {
        let json = std::fs::read_to_string(entry.expect("entry").path()).expect("report");
        assert!(json.contains("View"));
        assert!(json.contains("20.0"));
    }
    assert!(stats.save_session_in(None, first).is_err());
    assert!(stats
        .save_session_in(
            Some(temp.path()),
            std::time::UNIX_EPOCH - Duration::from_secs(1)
        )
        .is_err());
}
