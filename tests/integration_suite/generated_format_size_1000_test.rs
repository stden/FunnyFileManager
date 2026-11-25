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

mod bytes_000_099;
mod bytes_100_199;
mod bytes_200_299;
mod bytes_300_399;
mod bytes_400_499;
mod bytes_500_599;
mod bytes_600_699;
mod bytes_700_799;
mod bytes_800_899;
mod bytes_900_999;
