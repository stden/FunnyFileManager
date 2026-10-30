//! Purpose: unit tests for the `FileSize` display form: units, boundaries and precision.

use super::*;

#[test]
fn test_format_size_bytes() {
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
    assert_eq!(FileSize::from_bytes(1).to_string(), "1 B");
    assert_eq!(FileSize::from_bytes(999).to_string(), "999 B");
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
}

#[test]
fn test_format_size_kilobytes() {
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1536).to_string(), "1.50 KB");
    assert_eq!(FileSize::from_bytes(2048).to_string(), "2.00 KB");
    assert_eq!(FileSize::from_bytes(1_048_575).to_string(), "1024.00 KB");
}

#[test]
fn test_format_size_megabytes() {
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(FileSize::from_bytes(1_572_864).to_string(), "1.50 MB");
    assert_eq!(FileSize::from_bytes(2_097_152).to_string(), "2.00 MB");
    assert_eq!(
        FileSize::from_bytes(1_073_741_823).to_string(),
        "1024.00 MB"
    );
}

#[test]
fn test_format_size_gigabytes() {
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
    assert_eq!(FileSize::from_bytes(1_610_612_736).to_string(), "1.50 GB");
    assert_eq!(FileSize::from_bytes(2_147_483_648).to_string(), "2.00 GB");
    assert_eq!(FileSize::from_bytes(10_737_418_240).to_string(), "10.00 GB");
}

#[test]
fn test_format_size_edge_cases() {
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
    assert_eq!(FileSize::from_bytes(1).to_string(), "1 B");
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1536).to_string(), "1.50 KB");
    assert_eq!(FileSize::from_bytes(1_048_575).to_string(), "1024.00 KB");
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(
        FileSize::from_bytes(1_073_741_823).to_string(),
        "1024.00 MB"
    );
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
    assert_eq!(
        FileSize::from_bytes(u64::MAX).to_string(),
        "17179869184.00 GB"
    );
}

#[test]
fn test_format_size_mutation_killer() {
    // Test that kills mutations that replace function with String::new() or "xyzzy"
    assert_ne!(FileSize::from_bytes(1000).to_string().len(), 0);
    assert!(!FileSize::from_bytes(1000).to_string().contains("xyzzy"));

    // Test exact KB constant (1024, not 1024 + 0 or 1024 / 1)
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_ne!(FileSize::from_bytes(1025).to_string(), "1025 B"); // Should be KB

    // Test exact MB constant (1024 * 1024, not 1024 + 1024)
    assert_eq!(FileSize::from_bytes(1_048_576).to_string(), "1.00 MB");
    assert_eq!(FileSize::from_bytes(1_048_575).to_string(), "1024.00 KB");
    assert_ne!(FileSize::from_bytes(1_048_577).to_string(), "1048577 B"); // Should be MB not bytes

    // Test exact GB constant
    assert_eq!(FileSize::from_bytes(1_073_741_824).to_string(), "1.00 GB");
    assert_eq!(
        FileSize::from_bytes(1_073_741_823).to_string(),
        "1024.00 MB"
    );

    // Test >= vs < mutations
    assert_ne!(
        FileSize::from_bytes(1024).to_string(),
        FileSize::from_bytes(1023).to_string()
    ); // KB boundary
    assert_ne!(
        FileSize::from_bytes(1_048_576).to_string(),
        FileSize::from_bytes(1_048_575).to_string()
    ); // MB boundary
    assert_ne!(
        FileSize::from_bytes(1_073_741_824).to_string(),
        FileSize::from_bytes(1_073_741_823).to_string()
    ); // GB boundary
}

#[test]
fn test_format_size_precision() {
    assert_eq!(FileSize::from_bytes(1500).to_string(), "1.46 KB");
    assert_eq!(FileSize::from_bytes(1536).to_string(), "1.50 KB");
    assert_eq!(FileSize::from_bytes(1999).to_string(), "1.95 KB");
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 + 512 * 1024).to_string(),
        "1.50 MB"
    );
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 * 1024 + 512 * 1024 * 1024).to_string(),
        "1.50 GB"
    );
}
