//! Line parser for MakeMKV output.
//!
//! # Escaping Rules
//!
//! This parser implements MakeMKV's escaping conventions:
//! - `\"` → `"` (escaped quote)
//! - `\\` → `\` (escaped backslash)
//! - `\n` → newline (for rare multiline values)
//! - `\t` → tab (for rare tab-containing values)
//!
//! Fields are delimited by commas and may be quoted with double quotes.
//! Unquoted fields are treated as literals.
//!
//! # Why Not Use the `csv` Crate?
//!
//! MakeMKV's output format uses **non-standard escaping**:
//! - CSV: quotes inside fields are escaped by doubling them (`""`)
//! - MakeMKV: quotes inside fields are escaped with backslash (`\"`)
//!
//! The `csv` crate only understands standard CSV escaping, making it
//! unsuitable for this task.
//!
//! Additionally, the `csv` crate is designed for **full-file parsing** via
//! readers, but our architecture processes MakeMKV output **line-by-line**
//! to be able to track progress while makemkvcon is running. Creating a
//! backup or extracting MKV files can easily take half an hour to complete.
//! Creating a new CSV reader to process each line as it's being generated
//! is inefficient.

/// Parse a single line of MakeMKV output.
///
/// # Arguments
///
/// * `line` - A line of MakeMKV output as bytes (comma-separated fields
///   with backslash-escaping)
///
/// # Returns
///
/// A vector of parsed field values with escapes processed
///
/// # Examples
///
/// Without escaped characters:
///
/// ```ignore
/// let line = b"3007,0,0,\"Using direct disc access mode\",\"Using direct disc access mode\"";
/// let fields = parse_line(line);
/// assert_eq!(fields[0], "3007");
/// assert_eq!(fields[3], "Using direct disc access mode");
/// ```
///
/// With escaped characters:
///
/// ```ignore
/// let line = b"5072,131072,1,\"Backing up disc into folder \\\"file:///path\\\"\",\"format\"";
/// let fields = parse_line(line);
/// assert_eq!(fields[3], "Backing up disc into folder \"file:///path\"");
/// ```
pub fn parse_line(line: &[u8]) -> Vec<String> {
    let s = std::str::from_utf8(line).expect("MakeMKV output should be valid UTF-8");
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut chars = s.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            '\\' if in_quotes => {
                // Inside quoted field: handle escape sequences
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '"' => {
                            chars.next();
                            current_field.push('"');
                        }
                        '\\' => {
                            chars.next();
                            current_field.push('\\');
                        }
                        'n' => {
                            chars.next();
                            current_field.push('\n');
                        }
                        't' => {
                            chars.next();
                            current_field.push('\t');
                        }
                        _ => {
                            // Unknown escape: keep the backslash
                            current_field.push(ch);
                        }
                    }
                } else {
                    // Backslash at end of string: keep it
                    current_field.push(ch);
                }
            }
            ',' if !in_quotes => {
                // Field delimiter (only outside quotes)
                fields.push(current_field.clone());
                current_field.clear();
            }
            _ => {
                current_field.push(ch);
            }
        }
    }

    // Push the last field
    fields.push(current_field);
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_fields() {
        let line = b"3007,0,0,\"message\",\"format\"";
        let fields = parse_line(line);
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], "3007");
        assert_eq!(fields[1], "0");
        assert_eq!(fields[3], "message");
    }

    #[test]
    fn parse_escaped_quotes() {
        let line = b"5072,131072,1,\"Backing up disc into folder \\\"file:///path\\\"\",\"format\"";
        let fields = parse_line(line);
        assert_eq!(fields[3], "Backing up disc into folder \"file:///path\"");
    }

    #[test]
    fn parse_escaped_backslashes() {
        let line = b"100,0,0,\"path\\\\to\\\\file\",\"format\"";
        let fields = parse_line(line);
        assert_eq!(fields[3], "path\\to\\file");
    }

    #[test]
    fn parse_mixed_escapes() {
        let line = b"100,0,2,\"Quote: \\\"value\\\"\",\"Back: \\\\\",\"other\"";
        let fields = parse_line(line);
        assert_eq!(fields[3], "Quote: \"value\"");
        assert_eq!(fields[4], "Back: \\");
    }
}
