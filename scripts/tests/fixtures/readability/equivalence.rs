//! Behavioral comparisons compiled directly by the optional lint self-test.

mod complex;
mod refactored;

#[test]
fn flag_counts_agree_for_low_patterns_each_bit_and_combined_masks() {
    for bits in 0..4096 {
        assert_eq!(complex::count_flags(bits), refactored::count_flags(bits));
    }
    for bit in 0..32 {
        assert_eq!(
            complex::count_flags(1 << bit),
            refactored::count_flags(1 << bit)
        );
    }
    for bits in [u32::MAX, 0x03ff_ffff, 0xfc00_0000, 0xaaaa_aaaa] {
        assert_eq!(complex::count_flags(bits), refactored::count_flags(bits));
    }
}

#[test]
fn mixing_agrees_including_wrapping_inputs() {
    for value in (0..1024).chain([u64::MAX, u64::MAX - 1, 1 << 63]) {
        assert_eq!(
            complex::mix_repeatedly(value),
            refactored::mix_repeatedly(value)
        );
    }
}

#[test]
fn nested_sum_agrees_for_empty_groups_and_boundary_values() {
    let cases = [
        vec![],
        vec![vec![]],
        vec![vec![vec![]]],
        vec![vec![vec![-1, 0, 1, 9, 10, 11]]],
        vec![vec![vec![1, 2], vec![]], vec![vec![9, -1], vec![0, 3]]],
    ];
    for groups in cases {
        assert_eq!(
            complex::sum_small_values(&groups),
            refactored::sum_small_values(&groups)
        );
    }
}
