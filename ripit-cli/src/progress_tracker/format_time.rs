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

/// Format time as MM:SS or HH:MM:SS
pub fn format_time(secs: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

pub fn format_time_with_units(secs: u64) -> String {
    if secs >= 3600 {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    } else if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_seconds() {
        // ----------------------------------------------------------------
        let computed = format_time(4);
        let expected = "00:00:04";
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_time_minutes() {
        // ----------------------------------------------------------------
        let computed = format_time(65);
        let expected = "00:01:05";
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_time_hours() {
        // ----------------------------------------------------------------
        let computed = format_time(3661);
        let expected = "01:01:01";
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_time_with_units_seconds() {
        // ----------------------------------------------------------------
        let computed = format_time_with_units(4);
        let expected = "4s";
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_time_with_units_minutes() {
        // ----------------------------------------------------------------
        let computed = format_time_with_units(65);
        let expected = "1m 5s";
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_time_with_units_hours() {
        // ----------------------------------------------------------------
        let computed = format_time_with_units(3723);
        let expected = "1h 2m 3s";
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}
