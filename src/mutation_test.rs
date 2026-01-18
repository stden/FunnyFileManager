// Small module for mutation testing demonstration

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}

pub fn is_positive(n: i32) -> bool {
    n > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-1, 1), 0);
    }

    #[test]
    fn test_subtract() {
        assert_eq!(subtract(5, 3), 2);
        assert_eq!(subtract(0, 1), -1);
    }

    #[test]
    fn test_is_positive() {
        assert!(is_positive(1));
        assert!(!is_positive(0));
        assert!(!is_positive(-1));
    }
}
