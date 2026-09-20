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
        // let sty = ProgressStyle::with_template(
        //     "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        // )
        // .unwrap()
        // .progress_chars("##-");
        // let style = create_progress_style();

        Self {
            mp: MultiProgress::new(),
            pb: HashMap::new(),
            style: create_progress_style(),
        }
    }

    pub fn create_progress_bar(&mut self, id: &str, device: &str, stage: &str) {
        let prefix = format!("[{}] {:40}", device, stage);
        let message = format_time(0);
        let length = 100; // 100% = done

        // order of operations is important as outlined in indicatif:src/multi.rs
        // <https://github.com/console-rs/indicatif/blob/92cf1ae5f47c4bc2fb83556aa96f021727082ccc/src/multi.rs#L25-L29>

        // 1. create ProgressBar and attach it to MultiProgress
        // (!!do nothing else!!)
        let pb = self.mp.add(ProgressBar::new(length));

        // 2. configure the progress bar
        pb.set_style(self.style.clone());
        pb.set_prefix(prefix);
        pb.set_message(message);

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

    pub fn finish_progress_bar(&mut self, id: &str, message: &str) -> Result<(), bool> {
        if let Some(pb) = self.pb.get_mut(id) {
            pb.finish_with_message(message.to_owned());
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

    // pub fn stop(&mut self, ) {
    //     self.mp.clear().unwrap();
    // }

    pub fn get_seen(&self) -> Vec<String> {
        self.pb.keys().cloned().collect()
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        for (pb_id, pb) in self.pb.iter_mut() {
            debug!("{:30}: {}", pb_id, pb.position());
            pb.finish();
        }
        self.mp.clear().unwrap();
    }
}

// #[derive(Clone, Debug)]
// struct ProgressRecord {
//     device: String,
//     disc: String,
//     start: Instant,
//     status: String,
//     percentage: f32,
// }

// /// manages multi-device progress bars (rendered using indicatif)
// /// <https://github.com/console-rs/indicatif/blob/main/examples/multi.rs>
// pub struct ProgressRenderer {
//     // device-specific metrics
//     device: HashMap<String, ProgressRecord>,
//
//     // progress bars
//     mp: MultiProgress,
//     ps: ProgressStyle,
//
//     // global metrics
//     total_jobs: usize,
//     total_size: u64,
// }

// impl ProgressRenderer {
//
//     pub fn new() -> Self {
//
//         Self {
//             device: HashMap::new(),
//
//             mp: MultiProgress::new(),
//             ps: create_progress_style(),
//
//             total_jobs: 0,
//             total_size: 0,
//         }
//     }
//
//     pub fn create(&mut self, device: &str, status: &str) {
//         // let value = ProgressRecord {
//         //     device: device.to_owned(),
//         //     disc: "".to_string(),
//         //     start: Instant::now(),
//         //     status: status.to_owned(),
//         //     percentage: f32::NAN,
//         // };
//         // let key = device.to_string_lossy().to_string();
//         // self.device.insert(key, value);
//
//         // add the progress bar
//         let n = 200;
//         let pb = self.mp.add(ProgressBar::new(n));
//         pb.set_style(self.ps.clone());
//         pb.set_message(format!("{} {}", device, status));
//     }
//
//     pub fn update(&mut self, device: &PathBuf, disc_name: &str, status: &str, percentage: f32) {
//         let key = device.to_string_lossy().to_string();
//         if let Some(value) = self.device.get_mut(&key) {
//             // update existing entry
//             value.disc = disc_name.to_owned();
//             value.status = status.to_owned();
//             value.percentage = percentage;
//         } else {
//             // create a new entry
//             let value = ProgressRecord {
//                 device: device.clone(),
//                 disc: disc_name.to_owned(),
//                 start: Instant::now(),
//                 status: status.to_owned(),
//                 percentage,
//             };
//             self.device.insert(key, value);
//         };
//     }
//
//     pub fn delete(&mut self, device: &PathBuf) {
//         let key = device.to_string_lossy().to_string();
//         self.device.remove(&key);
//     }
//
//     pub fn add_completed_job(&mut self, filesize: u64) {
//         self.total_jobs += 1;
//         self.total_size += filesize;
//     }
//
//     fn update_progress(&mut self, drive_name: &PathBuf, disc_name: &str, status: &str) {
//         self.mp.println("Starting!").unwrap();
//
//         //let progress_bars = x;
//
//         // // get or create progress bar for this device
//         // let device_pb = self.device_progress.entry(drive_name.to_owned()).or_insert({
//         //     let pb_style = create_progress_style();
//         //     let bar = {
//         //         let mp = multi.lock().unwrap();
//         //         let progress_bar = mp.add(ProgressBar::new(100));
//         //         progress_bar.set_style(pb_style);
//         //         progress_bar
//         //     };
//         //     Arc::new(Mutex::new(DeviceProgress {
//         //             progress_bar: bar,
//         //             status: status.clone(),
//         //     }))
//         // });
//
//         // Update the progress bar
//         let mut dp = device_pb.lock().unwrap();
//         dp.status = status.clone();
//
//         match status {
//             // ⠙ [/dev/sr1]                      | Idle (drive empty)         [░░░░░░░░░░░░░░░░░░░░] Time: 00:26
//             DeviceStatus::Idle { reason } => {
//                 let stage = format!("Idle (reason: {})", reason);
//                 let percentage = f32::NAN;
//                 let prefix = format_prefix(drive_name, disc_name, &stage, percentage);
//                 dp.progress_bar.set_position(0);
//                 dp.progress_bar.set_prefix(prefix);
//                 dp.progress_bar.set_message("");
//             }
//             // ⠙ [/dev/sr1] OTAKU_NO_VIDEO (DVD) | Scanning contents (1%)     [░░░░░░░░░░░░░░░░░░░░] 00:10:20
//             DeviceStatus::Processing {
//                 stage,
//                 percentage,
//                 elapsed_secs,
//             } => {
//                 let prefix = format_prefix(drive_name, disc_name, stage, *percentage);
//                 let elapsed_fmt = format_time(*elapsed_secs);
//                 dp.progress_bar.set_position(*percentage as u64);
//                 dp.progress_bar.position();
//                 dp.progress_bar.set_prefix(prefix);
//                 dp.progress_bar.set_message(format!("{:>9}", elapsed_fmt));
//             }
//             // ⠙ [/dev/sr1]                      | ✓ Complete (7.3GiB)        [░░░░░░░░░░░░░░░░░░░░] 00:10:20
//             DeviceStatus::Completed {
//                 fs_size_bytes,
//                 elapsed_secs,
//             } => {
//                 let filesize = format_filesize(*fs_size_bytes);
//                 let stage = format!("✓ Complete ({})", filesize);
//                 let percentage = f32::NAN;
//                 let prefix = format_prefix(drive_name, disc_name, &stage, percentage);
//                 let elapsed_fmt = format_time(*elapsed_secs);
//                 dp.progress_bar.set_position(100);
//                 dp.progress_bar.set_prefix(prefix);
//                 dp.progress_bar.set_message(format!("{:>9}", elapsed_fmt));
//                 dp.progress_bar.finish();
//             }
//             // ⠙ [/dev/sr1]                      | ✗ Failed: Permission error [░░░░░░░░░░░░░░░░░░░░] 00:00:01
//             DeviceStatus::Failed {
//                 error_description,
//                 elapsed_secs
//             } => {
//                 let stage = format!("✗ Failed: {}", truncate_string(error_description, 30));
//                 let percentage = f32::NAN;
//                 let prefix = format_prefix(drive_name, disc_name, &stage, percentage);
//                 let elapsed_fmt = format_time(*elapsed_secs);
//                 dp.progress_bar.set_position(0);
//                 dp.progress_bar.set_prefix(prefix);
//                 dp.progress_bar.set_message(format!("{:>9}", elapsed_fmt));
//                 dp.progress_bar.finish();
//             }
//         }
//     }
//
//     /// Render structured log format for non-interactive output
//     pub fn render_structured_log(&self, _event_type: &str, drive_name: &PathBuf, disc_name: &str, status: &DeviceStatus) -> String {
//         let device_name_str = drive_name.to_string_lossy();
//
//         match status {
//             DeviceStatus::Idle {
//                 reason
//             } => {
//                 format!(
//                     "S: {} | IDLE | Disc={} | Reason={}",
//                     device_name_str,
//                     disc_name,
//                     reason
//                 )
//             }
//             DeviceStatus::Processing {
//                 stage,
//                 percentage,
//                 elapsed_secs,
//             } => {
//                 format!(
//                     "S: {} | PROG | Disc={} | Stage={} | {}% | Time={}",
//                     device_name_str,
//                     disc_name,
//                     stage,
//                     *percentage as u32,
//                     format_time(*elapsed_secs)
//                 )
//             }
//             DeviceStatus::Completed {
//                 fs_size_bytes,
//                 elapsed_secs,
//             } => {
//                 let size_fmt = format_filesize(*fs_size_bytes);
//                 format!(
//                     "S: {} | DONE | Disc={} | Size={} | Time={}",
//                     device_name_str,
//                     disc_name,
//                     size_fmt,
//                     format_time(*elapsed_secs)
//                 )
//             }
//             DeviceStatus::Failed {
//                 error_description,
//                 elapsed_secs,
//             } => {
//                 format!(
//                     "S: {} | FAIL | Disc={} | Error={} | Time={}",
//                     device_name_str,
//                     disc_name,
//                     error_description,
//                     format_time(*elapsed_secs)
//                 )
//             }
//         }
//     }
// }

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
