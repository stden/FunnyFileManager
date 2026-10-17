//! Purpose: Size formatting: units, multiplications and boundaries.

use super::*;

#[test]
fn test_format_size_not_empty() {
    // Kill: replace format_size -> String with String::new()
    let result = FileSize::from_bytes(12345).to_string();
    assert!(!result.is_empty());
}

#[test]
fn test_format_size_not_xyzzy() {
    // Kill: replace format_size -> String with "xyzzy".into()
    let result = FileSize::from_bytes(5000).to_string();
    assert!(!result.contains("xyzzy"));
    assert!(result.contains("KB") || result.contains('B'));
}

#[test]
fn test_format_size_kb_multiplication() {
    // Kill: replace * with + in KB = 1024
    // Kill: replace * with / in KB = 1024
    // If KB was 1024 + 1024 = 2048, then 1024 bytes would show as 0.5 KB
    // If KB was 1024 / 1024 = 1, then 1024 bytes would show as 1024.00 KB
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(2048).to_string(), "2.00 KB");
    assert_ne!(FileSize::from_bytes(1024).to_string(), "0.50 KB");
    assert_ne!(FileSize::from_bytes(1024).to_string(), "1024.00 KB");
}

#[test]
fn test_format_size_mb_multiplication() {
    // Kill: replace * with + in MB = KB * 1024
    // Kill: replace * with / in MB = KB * 1024
    // MB should be 1048576, not 1024 + 1024 = 2048
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(FileSize::from_bytes(2_097_152).to_string(), "2.00 MB");
    // If MB was KB + 1024 = 2048, then 1MB would show differently
    assert_ne!(FileSize::from_bytes(2048).to_string(), "1.00 MB");
    assert_ne!(FileSize::from_bytes(1_048_576).to_string(), "512.00 MB");
}

#[test]
fn test_format_size_gb_comparison() {
    // Kill: replace >= with < in if size >= GB
    let gb_minus_1 = FileSize::from_bytes(1_073_741_823);
    let gb_exact = FileSize::from_bytes(1_073_741_824);
    let gb_plus_1 = FileSize::from_bytes(1_073_741_825);

    assert!(gb_minus_1.to_string().contains("MB"));
    assert!(gb_exact.to_string().contains("GB"));
    assert!(gb_plus_1.to_string().contains("GB"));

    // These should be different
    assert_ne!(gb_minus_1.to_string(), gb_exact.to_string());
}

#[test]
fn test_format_size_mb_comparison() {
    // Kill: replace >= with < in if size >= MB
    let mb_minus_1 = FileSize::from_bytes(1_048_575);
    let mb_exact = FileSize::from_bytes(1_048_576);
    let mb_plus_1 = FileSize::from_bytes(1_048_577);

    assert!(mb_minus_1.to_string().contains("KB"));
    assert!(mb_exact.to_string().contains("MB"));
    assert!(mb_plus_1.to_string().contains("MB"));

    // These should be different
    assert_ne!(mb_minus_1.to_string(), mb_exact.to_string());
}

#[test]
fn test_format_size_kb_comparison() {
    // Kill: replace >= with < in if size >= KB
    let kb_minus_1 = FileSize::from_bytes(1023);
    let kb_exact = FileSize::from_bytes(1024);
    let kb_plus_1 = FileSize::from_bytes(1025);

    assert!(kb_minus_1.to_string().contains('B') && !kb_minus_1.to_string().contains("KB"));
    assert!(kb_exact.to_string().contains("KB"));
    assert!(kb_plus_1.to_string().contains("KB"));

    // These should be different
    assert_ne!(kb_minus_1.to_string(), kb_exact.to_string());
}
