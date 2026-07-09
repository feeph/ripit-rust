/*
    interface library for MakeMKV's console application (makemkvcon)
    https://www.makemkv.com/
*/

// TODO consider adding special logic for "MSG:3309"+"MSG:3041"
/*
The Grand Budapest Hotel (2014) BluRay:
  MSG:3309 Title 00851.mpls is equal to title 00801.mpls and was skipped
  MSG:3041 Failed to add angle #7 for title #851
*/

#[macro_use]
extern crate enum_primitive;

mod apdefs_h;
mod streams;

pub mod api;
pub mod parser;
pub mod source;

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

pub use source::{Source, parse_source};
pub use streams::{AudioStream, SubtitleStream, VideoStream};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

pub use crate::api::DriveRecord;
use crate::api::{ContentType, MessageRecord};
pub use crate::parser::{ParsedOutputLine, parse_output_line};

pub fn drives(makemkvcon_bin: &Path) -> (Vec<api::DriveRecord>, usize) {
    // intentionally using an invalid drive specification 'disc:-1' since
    // we're interested only in the 'disc id' to 'drive' mapping and don't
    // want to parse any inserted disc
    let mut child = std::process::Command::new(makemkvcon_bin)
        .args(["--robot", "info", "disc:-1"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn process");

    let mut result: Vec<api::DriveRecord> = Vec::new();
    let mut issues: usize = 0;
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);

        // we can safely ignore the startup message and the intentional
        // usage of the non-existing drive
        // MSG:1005 - MakeMKV v1.18.3 win(x64-release) started
        // MSG:5010 - Failed to open disc
        // silence this message:
        // MSG:5042 - The program can't find any usable optical drives.
        let severity_map = HashMap::from([
            (1005, parser::SeverityLevel::Debug), // version info
            (5010, parser::SeverityLevel::Debug), // no disc
            (5042, parser::SeverityLevel::Debug), // no drives
        ]);
        let parsed_output = parser::process_output(reader, 0, &severity_map);

        result = parsed_output.drives;
        issues = parsed_output.issues;
    }
    let _ = child.wait();

    (result, issues)
}

pub fn medium(
    makemkvcon_bin: &Path,
    dr: &DriveRecord,
    max_tries: u8,
) -> Option<(String, ContentType)> {
    for i in 1..max_tries {
        debug!(
            "Detecting medium in drive '{}'. ({}/{})",
            dr.index, i, max_tries
        );
        for drive in drives(makemkvcon_bin).0 {
            if drive.index == dr.index {
                let content_type = dr.content_type.clone();
                return Some((drive.disc_name, content_type));
            }
        }
        debug!("No medium detected: Wait and retry.");
        std::thread::sleep(std::time::Duration::from_millis(2000));
    }
    None
}

fn calculate_filesystem_size(path: &std::path::Path) -> u64 {
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

fn calculate_write_rate(bytes_written: u64, elapsed: std::time::Duration) -> u64 {
    let seconds = elapsed.as_secs();
    // return 0 if no time has elapsed
    bytes_written.checked_div(seconds).unwrap_or_default()
}

pub enum BackupEvent {
    Failure,
    Message(MessageRecord),
    Success,
    Status(u64, u64),
}

pub fn backup(makemkvcon_bin: PathBuf, disc_id: u8, target: PathBuf, tx: Sender<BackupEvent>) {
    let source_mkv = format!("disc:{}", disc_id);
    let target_mkv = target.to_string_lossy().to_string();
    debug!("Extracting '{}' to '{}'.", source_mkv, target_mkv);

    // 'makemkvcon backup <...>' requires the source to be provided as
    // 'disc:#', using 'dev:<DeviceName>' is not supported
    let mut child = std::process::Command::new(makemkvcon_bin)
        .args(["--robot", "backup", &source_mkv, &target_mkv])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn process");

    let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    // 'makemkvcon backup <...>' reports nothing while it's writing the
    // image to disc. If the reading slows down the user has no idea what's
    // going on.
    // - To provide a better user experience this function reports the file
    //   size and write rate while the extraction is running.
    // - Additionally this helps to keep the terminal session alive if
    //   connected remotely. (The network connection may stall since there
    //   is no output / traffic for ~20 minutes.)
    //
    // When creating an image it takes about a minute before anything is
    // written to the filesystem. The presence of 'target' indicates that
    // the backup process has begun and we can start the reporting.
    let tx2 = tx.clone();
    let monitor = std::thread::spawn(move || {
        let mut write_start = std::time::Instant::now();
        while !stop_flag_clone.load(std::sync::atomic::Ordering::Relaxed) {
            if !target.exists() {
                // 'target' does not exist, backup hasn't started
                // -> update the start time and go back to sleep
                write_start = std::time::Instant::now();
                std::thread::sleep(std::time::Duration::from_millis(500));
            } else {
                // 'target' exists, the extraction has begun
                // -> trigger a wait cycle and start the reporting
                std::thread::sleep(std::time::Duration::from_secs(30));
                // at least one wait cycle has passed and there should
                // be some data available now
                // -> report progress to user
                let size = calculate_filesystem_size(&target);
                let write_rate = calculate_write_rate(size, write_start.elapsed());
                tx2.send(BackupEvent::Status(size, write_rate)).unwrap();
            }
        }
    });

    let mut result = false; // assume failure until we saw MSG:5081
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    debug!("{}", line);
                    let parsed = parse_output_line(line.as_bytes());
                    match parsed {
                        // 'makemkvcon backup' generates MSG and DRV records
                        ParsedOutputLine::DRV(_) => {
                            // ignore drive records
                        }
                        ParsedOutputLine::MSG(msg) => {
                            // MSG:5080 - Backup failed.
                            // MSG:5081 - Backup done.
                            if msg.code == 5081 {
                                result = true;
                            }
                            tx.send(BackupEvent::Message(msg)).unwrap();
                        }
                        _ => {
                            warn!("Found an unexpected record during backup:");
                            warn!("{}", line);
                        }
                    }
                }
                Err(e) => error!("Error reading line: {}", e),
            }
        }
    }

    let _ = child.wait();
    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = monitor.join();

    if result {
        tx.send(BackupEvent::Success).unwrap();
    } else {
        tx.send(BackupEvent::Failure).unwrap();
    }
}

// ------------------------------------------------------------------------

fn parse_as_usize(value: &str) -> Option<usize> {
    match value.parse::<usize>() {
        Ok(x) => Some(x),
        Err(_) => {
            warn!("Unable to parse '{}' as usize!", value);
            None
        }
    }
}

// ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Streams {
    pub audio: HashMap<usize, streams::AudioStream>,
    pub video: HashMap<usize, streams::VideoStream>,
    pub subtitle: HashMap<usize, streams::SubtitleStream>,
}

// ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ScanResult {
    pub parsed: Option<parser::ContentRecord>,
    pub drives: Vec<DriveRecord>,
    pub issues: usize,
    pub errors: usize,
}

pub fn info(makemkvcon_bin: &Path, source: &str, min_length: usize) -> Option<ScanResult> {
    let source_mkv = match source::parse_source(source) {
        Some(x) => x,
        None => {
            error!(
                "Usage error: Source must be a disc id, drive letter, device name, iso file, or directory!"
            );
            return None;
        }
    };
    debug!("Processing '{}'.", source);

    let arg_min = format!("--minlength={}", min_length);

    let mut child = std::process::Command::new(makemkvcon_bin)
        .args(["--robot", "info", &arg_min, &source_mkv.to_string()])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn process");

    // TODO return actual makemkvcon version number
    let mut scan_result = ScanResult {
        drives: Vec::new(),
        parsed: None,
        issues: 0,
        errors: 0,
    };
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);

        // MSG:1005 - MakeMKV v1.18.3 win(x64-release) started
        // MSG:5075 The new version 1.18.3 is available for download at http://www.makemkv.com/download/
        // MSG:3324 Processing BD+ code using generic SVQ from builtin/generic.svq
        // MSG:3328 BD+ code processed, got 1 FUT(s) for 20 clip(s)
        // MSG:3338 Downloading latest SDF to <homedir>/.MakeMKV ...
        // MSG:1011 - Using LibreDrive mode (v02.1 id=3F03CED516D5)
        // MSG:3006 Opening files on harddrive at file://<...>
        // MSG:3007 - Using direct disc access mode
        // MSG:3028 - Title #2 was added (7 cell(s), 0:06:07)
        // MSG:3025 - Title #3 has length of 20 seconds which is less than minimum title length of 120 seconds and was therefore skipped
        // MSG:3307 File 00006.mpls was added as title #0
        // MSG:3308 File 00800.mpls (angle 1) was added as title #8
        // MSG:3309 Title 00021.mpls(1) is equal to title 00006.mpls and was skipped
        // MSG:3041 Failed to add angle #2 for title #850
        // MSG:3026 Title #11 declared length is 0:00:00 while its real length is 0:00:16 - assuming fake title
        // MSG:3027 Title #03 in VTS 1 is equal to title #01 and was skipped
        // MSG:3038 - Cells 3-7 were removed from title end
        // MSG:3344 Using Java runtime from /usr/lib/jvm/java-17-openjdk-amd64/bin/java
        // MSG:5085 Loaded content hash table, will verify integrity of M2TS files.
        // <...>
        // MSG:5011 - Operation successfully completed
        // MSG:5014 - Saving 1 titles into directory file://<...>
        // MSG:2019 - Error 'OS error - The system cannot find the path specified' occurred while creating '<...>/B1_t00.mkv'
        // MSG:2024 - Unknown device - 'D:'
        // MSG:5003 - Failed to save title 0 to file <...>/B1_t00.mkv
        // MSG:5004 - 0 titles saved, 1 failed
        // MSG:5010 - Failed to open disc
        // MSG:5037 - Copy complete. 0 titles saved, 1 failed.
        let mut severity_map = HashMap::from([
            (1005, parser::SeverityLevel::Info),
            (1011, parser::SeverityLevel::Info),
            (2019, parser::SeverityLevel::Error),
            (2024, parser::SeverityLevel::Error),
            (3006, parser::SeverityLevel::Info),
            (3007, parser::SeverityLevel::Info),
            (3025, parser::SeverityLevel::Info),
            (3026, parser::SeverityLevel::Info),
            (3027, parser::SeverityLevel::Info),
            (3028, parser::SeverityLevel::Info),
            (3038, parser::SeverityLevel::Info),
            (3041, parser::SeverityLevel::Warning),
            (3306, parser::SeverityLevel::Info),
            (3307, parser::SeverityLevel::Info),
            (3308, parser::SeverityLevel::Info),
            (3309, parser::SeverityLevel::Info),
            (3324, parser::SeverityLevel::Info),
            (3326, parser::SeverityLevel::Info),
            (3328, parser::SeverityLevel::Info),
            (3338, parser::SeverityLevel::Info),
            (3344, parser::SeverityLevel::Info),
            (5003, parser::SeverityLevel::Error),
            (5004, parser::SeverityLevel::Error),
            (5010, parser::SeverityLevel::Error),
            (5011, parser::SeverityLevel::Info),
            (5014, parser::SeverityLevel::Info),
            (5037, parser::SeverityLevel::Info),
            (5075, parser::SeverityLevel::Warning),
            (5085, parser::SeverityLevel::Info),
        ]);

        // silence messages related to optical drives when using the
        // filesystem as a source, promote to error when using a drive
        match source_mkv {
            // MSG:5042 - The program can't find any usable optical drives.
            Source::Directory(_) | Source::IsoFile(_) => {
                severity_map.insert(5042, parser::SeverityLevel::Debug);
            }
            Source::DriveId(_) | Source::DriveLetter(_) | Source::DeviceName(_) => {
                severity_map.insert(5042, parser::SeverityLevel::Error);
            }
        };

        let parsed_output = parser::process_output(reader, min_length, &severity_map);

        scan_result.drives = parsed_output.drives;

        scan_result.parsed = Some(parsed_output.content);
        scan_result.issues += parsed_output.issues;
        scan_result.errors += parsed_output.errors;
    }
    let _ = child.wait();

    Some(scan_result)
}

pub fn mkv(
    makemkvcon_bin: &Path,
    source: &str,
    title: usize,
    target: &Path,
    min_length: usize,
) -> bool {
    let source_mkv = match source::parse_source(source) {
        Some(x) => x,
        None => {
            error!(
                "Usage error: Source must be a disc id, drive letter, device name, iso file, or directory!"
            );
            return false;
        }
    };
    let target_mkv = target.to_string_lossy().to_string();

    if !target.exists() {
        match std::fs::create_dir_all(target) {
            Ok(_) => {
                debug!("Successfully created directory '{}'.", target_mkv)
            }
            Err(e) => {
                error!("Failed to create directory '{}': {}", target_mkv, e);
                return false;
            }
        }
    }

    let mut child = std::process::Command::new(makemkvcon_bin)
        .args([
            "--robot",
            "mkv",
            &source_mkv.to_string(),
            &title.to_string(),
            &target_mkv,
        ])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn process");

    let mut result = true;
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);

        // MSG:1005 - MakeMKV v1.18.3 win(x64-release) started
        // MSG:5075 The new version 1.18.3 is available for download at http://www.makemkv.com/download/
        // MSG:3324 Processing BD+ code using generic SVQ from builtin/generic.svq
        // MSG:3328 BD+ code processed, got 1 FUT(s) for 20 clip(s)
        // MSG:3338 Downloading latest SDF to <homedir>/.MakeMKV ...
        // MSG:3007 - Using direct disc access mode
        // MSG:3028 - Title #2 was added (7 cell(s), 0:06:07)
        // MSG:3025 - Title #3 has length of 20 seconds which is less than minimum title length of 120 seconds and was therefore skipped
        // MSG:3038 - Cells 3-7 were removed from title end
        // MSG:3006 Opening files on harddrive at file://<...>
        // MSG:3307 File 00006.mpls was added as title #0
        // MSG:3308 File 00800.mpls (angle 1) was added as title #8
        // MSG:3309 Title 00021.mpls(1) is equal to title 00006.mpls and was skipped
        // MSG:3041 Failed to add angle #2 for title #850
        // MSG:3026 Title #11 declared length is 0:00:00 while its real length is 0:00:16 - assuming fake title
        // MSG:3344 Using Java runtime from /usr/lib/jvm/java-17-openjdk-amd64/bin/java
        // MSG:5014 - Saving 1 titles into directory file://<...>
        // MSG:5085 Loaded content hash table, will verify integrity of M2TS files.
        // <...>
        // -- success --
        // MSG:5005 - 1 titles saved
        // MSG:5011 - Operation successfully completed
        // MSG:5036 - Copy complete. 1 titles saved.
        // -- conflict --
        // MSG:5001 File ./B1_t00.mkv already exist. Do you want to overwrite it?
        // MSG:5005 1 titles saved
        // -- failure --
        // MSG:2019 - Error 'OS error - The system cannot find the path specified' occurred while creating '<...>/B1_t00.mkv'
        // MSG:5003 - Failed to save title 0 to file <...>/B1_t00.mkv
        // MSG:5004 - 0 titles saved, 1 failed
        // MSG:5037 - Copy complete. 0 titles saved, 1 failed.
        let mut severity_map = HashMap::from([
            (1005, parser::SeverityLevel::Debug),
            (2019, parser::SeverityLevel::Error),
            (3006, parser::SeverityLevel::Info),
            (3007, parser::SeverityLevel::Debug),
            (3025, parser::SeverityLevel::Debug),
            (3026, parser::SeverityLevel::Info),
            (3028, parser::SeverityLevel::Debug),
            (3038, parser::SeverityLevel::Debug),
            (3041, parser::SeverityLevel::Warning),
            (3307, parser::SeverityLevel::Debug),
            (3308, parser::SeverityLevel::Debug),
            (3309, parser::SeverityLevel::Debug),
            (3324, parser::SeverityLevel::Debug),
            (3328, parser::SeverityLevel::Debug),
            (3338, parser::SeverityLevel::Info),
            (3344, parser::SeverityLevel::Debug),
            (5001, parser::SeverityLevel::Warning),
            (5003, parser::SeverityLevel::Error),
            (5004, parser::SeverityLevel::Error),
            (5005, parser::SeverityLevel::Debug),
            (5011, parser::SeverityLevel::Debug),
            (5014, parser::SeverityLevel::Debug),
            (5036, parser::SeverityLevel::Debug),
            (5037, parser::SeverityLevel::Error),
            (5075, parser::SeverityLevel::Warning),
            (5085, parser::SeverityLevel::Debug),
        ]);

        // MSG:2008 Program reads data faster than it can write to disk, consider upgrading your hard drive if you see many of these messages.
        match &source_mkv {
            Source::IsoFile(_) | Source::Directory(_) => {
                // not reading from an optical drive; silence this message
                severity_map.insert(2008, parser::SeverityLevel::Debug);
            }
            Source::DriveId(_) | Source::DeviceName(_) | Source::DriveLetter(_) => {
                // reading from an optical drive; promote to warning
                severity_map.insert(2008, parser::SeverityLevel::Warning);
            }
        }

        // silence messages related to optical drives when using the
        // filesystem as a source, promote to error when using a drive
        match source_mkv {
            // MSG:5042 - The program can't find any usable optical drives.
            Source::Directory(_) | Source::IsoFile(_) => {
                severity_map.insert(5042, parser::SeverityLevel::Debug);
            }
            Source::DriveId(_) | Source::DriveLetter(_) | Source::DeviceName(_) => {
                severity_map.insert(5042, parser::SeverityLevel::Error);
            }
        };

        let parsed_output = parser::process_output(reader, min_length, &severity_map);
        if parsed_output.errors > 0 {
            result = false;
        }
    }

    let _ = child.wait();

    result
}

// TODO makemkvcon f

// TODO makemkvcon reg
