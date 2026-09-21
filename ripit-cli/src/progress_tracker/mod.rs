/*
    Progress rendering for unshackle command

    Handles both interactive (TTY) and non-interactive output modes with graceful
    degradation. In TTY mode, uses indicatif's multi-progress for in-place table
    updates. In pipe/file mode, uses structured log format.
*/

mod format_time;
mod truncate_string;

// standard library imports
use std::collections::HashMap;
use std::path::PathBuf;
// use std::sync::{Arc, Mutex};
// use std::thread;
// use std::time::{Instant, Duration};

// third-party imports
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

// crate-provided imports
use truncate_string::truncate_string;

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub use format_time::{format_time, format_time_with_units};

/// manages multi-device progress bars (rendered using indicatif)
/// <https://github.com/console-rs/indicatif/blob/main/examples/multi.rs>
pub struct ProgressTracker {
    mp: MultiProgress,
    style: ProgressStyle,

    pb: HashMap<String, ProgressBar>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            mp: MultiProgress::new(),
            pb: HashMap::new(),
            style: create_progress_style(),
        }
    }

    pub fn get_mp(&self) -> MultiProgress {
        self.mp.clone()
    }

    pub fn create_progress_bar(&mut self, id: &str, device: &str, stage: &str) {
        let prefix = format!("[{}] {:40}", device, stage);
        let message = format_time(0);
        let length = 100; // 100% = done

        // !! order of operations is extremely important !!
        // see indicatif's documentation for details:
        // <https://github.com/console-rs/indicatif/blob/92cf1ae5f47c4bc2fb83556aa96f021727082ccc/src/multi.rs#L25-L29>
        let pb = self
            .mp
            .add(ProgressBar::new(length))
            .with_style(self.style.clone())
            .with_prefix(prefix)
            .with_message(message);

        self.pb.insert(id.to_owned(), pb);
    }

    pub fn create_progress_bar_after(
        &mut self,
        id: &str,
        device: &str,
        stage: &str,
        id_parent: &str,
    ) -> Result<(), bool> {
        let prefix = format!("[{}] {:40}", device, stage);
        let message = format_time(0);
        let length = 100; // 100% = done

        if let Some(pb_parent) = self.pb.get_mut(id_parent) {
            let pb = self
                .mp
                .insert_after(pb_parent, ProgressBar::new(length))
                .with_style(self.style.clone())
                .with_prefix(prefix)
                .with_message(message);

            self.pb.insert(id.to_owned(), pb);
            Ok(())
        } else {
            Err(false)
        }
    }

    pub fn update_progress_bar(
        &mut self,
        id: &str,
        device: &str,
        stage: &str,
        percentage: f32,
    ) -> Result<(), bool> {
        let stage_str = if percentage.is_nan() {
            stage.to_string()
        } else {
            format!("{} ({:.0}%)", stage, percentage)
        };
        let message = format!("[{}] {:40}", device, stage_str);

        if let Some(pb) = self.pb.get_mut(id) {
            pb.set_message(message);
            pb.set_position(percentage.floor() as u64);
            Ok(())
        } else {
            Err(false)
        }
    }

    pub fn clear_progress_bar(&mut self, id: &str) -> Result<(), bool> {
        if let Some(pb) = self.pb.get_mut(id) {
            pb.finish_and_clear();
            Ok(())
        } else {
            Err(false)
        }
    }

    pub fn send_text_message(&mut self, message: &str) {
        self.mp.println(message).unwrap();
    }
}

// use here and by cmd_unshackle
pub fn format_filesize(bytes: u64) -> String {
    if bytes == 0 {
        return String::from("-");
    }

    let gib_divisor = 1024.0_f64 * 1024.0 * 1024.0;
    let mib_divisor = 1024.0_f64 * 1024.0;
    let kib_divisor = 1024.0_f64;

    let size_gib = bytes as f64 / gib_divisor;
    if size_gib >= 1.0 {
        return format!("{:.1} GiB", size_gib);
    }

    let size_mib = bytes as f64 / mib_divisor;
    if size_mib >= 1.0 {
        return format!("{:.1} MiB", size_mib);
    }

    let size_kib = bytes as f64 / kib_divisor;
    if size_kib >= 1.0 {
        return format!("{:.1} KiB", size_kib);
    }

    format!("{} B", bytes)
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

/// create the progress style for indicatif bars
fn create_progress_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.green} {msg:80} [{bar:20.cyan/blue}] {elapsed_precise}")
        .unwrap()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
}

fn format_message(drive_name: &PathBuf, disc_name: &str, stage: &str, percentage: f32) -> String {
    let device_name_str = drive_name.to_string_lossy();

    // pre-format these values so we can align the combined value
    let disc_name = disc_name.to_string();
    let stage_fmt = if percentage.is_nan() {
        // e.g. '✓ Complete'
        truncate_string(stage, 26)
    } else {
        // e.g. 'Copying file (42%)'
        format!("{} ({:.0}%)", truncate_string(stage, 26), percentage)
    };

    // generate the full output
    format!("[{}] {:30} | {:30}", device_name_str, disc_name, stage_fmt)
}

// ------------------------------------------------------------------------
// unit tests
// ------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_format_filesize() {
        assert_eq!(format_filesize(0), "-");
        assert_eq!(format_filesize(1024), "1.0 KiB");
        assert_eq!(format_filesize(1_048_576), "1.0 MiB");
        assert_eq!(format_filesize(4_700_000_000), "4.4 GiB");
    }

    #[test]
    fn test_format_prefix() {
        let drive_name = PathBuf::from("/dev/sr0");
        let disc_name = "DVDVolume";
        let stage = "Scanning contents";
        let percentage = 12.34;
        // ----------------------------------------------------------------
        let computed = format_message(&drive_name, disc_name, stage, percentage);
        let expected = "[/dev/sr0] DVDVolume                      | Scanning contents (12%)       "
            .to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}
