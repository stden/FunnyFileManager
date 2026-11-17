use super::{Bounded, Write};

#[test]
fn streaming_limit_preserves_accepted_bytes_after_a_later_chunk_fails() {
    let mut output = Bounded {
        bytes: Vec::new(),
        limit: 5,
    };
    assert_eq!(output.write(b"first").expect("within limit"), 5);
    assert!(output.write(b"second").is_err());
    output.flush().expect("in-memory output flushes");
    assert_eq!(output.bytes, b"first");
}
