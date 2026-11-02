use super::*;

#[test]
fn a_zero_length_range_does_not_open_the_file() {
    assert_eq!(read_line_range(Path::new("absent"), 1, 0).unwrap(), (Vec::new(), false));
}
