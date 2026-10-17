//! Purpose: Generates many `format_size` cases to guard size formatting boundaries.
//!

use funny_file_manager::models::FileSize;
macro_rules! generate_format_size_tests {
    ($(($name:ident, $size:expr, $expected:expr),)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(FileSize::from_bytes($size).to_string(), $expected);
            }
        )*
    };
}

#[path = "generated_format_size_1000_test/bytes_000_099.rs"]
mod bytes_000_099;
#[path = "generated_format_size_1000_test/bytes_100_199.rs"]
mod bytes_100_199;
#[path = "generated_format_size_1000_test/bytes_200_299.rs"]
mod bytes_200_299;
#[path = "generated_format_size_1000_test/bytes_300_399.rs"]
mod bytes_300_399;
#[path = "generated_format_size_1000_test/bytes_400_499.rs"]
mod bytes_400_499;
#[path = "generated_format_size_1000_test/bytes_500_599.rs"]
mod bytes_500_599;
#[path = "generated_format_size_1000_test/bytes_600_699.rs"]
mod bytes_600_699;
#[path = "generated_format_size_1000_test/bytes_700_799.rs"]
mod bytes_700_799;
#[path = "generated_format_size_1000_test/bytes_800_899.rs"]
mod bytes_800_899;
#[path = "generated_format_size_1000_test/bytes_900_999.rs"]
mod bytes_900_999;
