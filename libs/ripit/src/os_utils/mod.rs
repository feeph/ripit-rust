/*!
    operating system related utilities
*/

// standard library imports
use std::path::Path;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};

// crate-provided imports
// <none>

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub fn calculate_filesystem_size(path: &Path) -> u64 {
    let mut total = 0;

    if path.is_file() {
        total = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    } else if path.is_dir()
        && let Ok(entries) = std::fs::read_dir(path)
    {
        for entry in entries.flatten() {
            total += calculate_filesystem_size(&entry.path());
        }
    }

    total
}
