/*
    format a time given in seconds as "[HH:]MM:SS"
*/

// standard library imports
// <none>

// third-party imports
// <none>

// crate-provided imports
// <none>

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

/// Truncate string to max length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if max_len < 2 {
        // Can't fit ellipsis, return original (truncated to max_len)
        s.chars().take(max_len).collect()
    } else if s.len() > max_len {
        // Truncate and add ellipsis
        format!("{}…", &s[..max_len - 1])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string_preserve() {
        // ----------------------------------------------------------------
        let computed = truncate_string("hello world 1234", 16);
        let expected = "hello world 1234".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_truncate_string_truncate() {
        // ----------------------------------------------------------------
        let computed = truncate_string("hello world 1234", 15);
        let expected = "hello world 12…".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_truncate_string_zero_length() {
        // ----------------------------------------------------------------
        let computed = truncate_string("hello world", 0);
        let expected = "".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}
