//! Purpose: The session viewport and private PTY use the same validated size.

use super::*;
use ratatui::layout::Size;

#[test]
fn preferred_size_is_validated_scoped_and_preserved_between_jobs() {
    let host = HeadlessHost::new();
    let _scope = host.enter();
    assert_eq!(viewport(), Some(Size::new(120, 40)));
    assert!(host.resize(0, 40).is_err());
    assert!(host.resize(120, u16::MAX).is_err());
    resize(100, 30).unwrap();
    assert_eq!(host.size(), Size::new(100, 30));
    host.reset().unwrap();
    assert_eq!(viewport(), Some(Size::new(100, 30)));
}
