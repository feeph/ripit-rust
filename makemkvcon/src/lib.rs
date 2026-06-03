/*
    interface library for MakeMKV's console application (makemkvcon)
    https://www.makemkv.com/
*/

#[macro_use]
extern crate enum_primitive;

mod apdefs_h;
mod streams;

pub mod api;
pub mod parser;
pub mod source;

use itertools::Itertools;
use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;

pub use source::{Source, parse_source};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

pub use crate::api::DriveRecord;
use crate::parser::InfoRecordOut;

pub fn drives(makemkvcon_bin: &PathBuf) -> (Vec<api::DriveRecord>, usize) {
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

pub fn backup(makemkvcon_bin: &PathBuf, disc_id: u8, target: PathBuf, overwrite: bool) -> bool {
    let source_mkv = format!("disc:{}", disc_id);
    let target_mkv = target.to_string_lossy().to_string();
    info!("Extracting '{}' to '{}'.", source_mkv, target_mkv);

    if overwrite && target.exists() {
        if target.is_dir() {
            info!(
                "Removing existing directory '{}'.",
                target.to_string_lossy()
            );
            std::fs::remove_dir_all(&target).expect("Failed to remove existing directory");
        } else if target.is_file() {
            info!("Removing existing file '{}'.", target.to_string_lossy());
            std::fs::remove_file(&target).expect("Failed to remove existing file");
        }
    }

    let mut child = std::process::Command::new(makemkvcon_bin)
        .args(["--robot", "backup", &source_mkv, &target_mkv])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn process");
    let time_start = std::time::Instant::now();

    let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();

    // report size of 'target' while the extraction is running
    // - `makemkvcon backup` takes about a minute before writing to the
    //   filesystem; the presence of 'target' indicates that the backup
    //   process has begun
    // - if everything goes well `makemkvcon backup` itself creates no
    //   output while it's running; we add our own reporting of size and
    //   write rate so the user can see how fast/slow the extraction is
    let monitor = std::thread::spawn(move || {
        let mut write_start = std::time::Instant::now();
        while !stop_flag_clone.load(std::sync::atomic::Ordering::Relaxed) {
            if !target.exists() {
                // 'target' does not exist
                // -> update the start time and go back to sleep
                write_start = std::time::Instant::now();
                std::thread::sleep(std::time::Duration::from_millis(500));
            } else {
                // 'target' exists, the extraction has begun
                // -> trigger a wait cycle
                std::thread::sleep(std::time::Duration::from_secs(30));
                // at least one wait cycle has passed and there should
                // be some data available now
                // -> report progress to user
                let size = calculate_filesystem_size(&target);
                let size_gib = size as f64 / f64::powf(1024.0, 3.0);
                let write_rate = calculate_write_rate(size, write_start.elapsed());
                let write_rate_mibs = write_rate as f64 / f64::powf(1024.0, 2.0);
                info!(
                    "Processed: {:5.2} GiB ({:.1} MiB/s)",
                    size_gib, write_rate_mibs
                );
            }
        }
    });

    let mut result = true;
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);

        // minimum title length does not matter for `makemkvcon backup`
        let min_length = 0;

        // MSG:1005 - MakeMKV v1.18.3 win(x64-release) started
        // MSG:1011 - Using LibreDrive mode (v02.1 id=3F03CED516D5)
        // MSG:5042 - The program can't find any usable optical drives.
        // MSG:5072 - Backing up disc into folder \file://Layer Cake (2001)\\LOGICAL_VOLUME_ID_5911EE08\""
        // MSG:5085 - Loaded content hash table, will verify integrity of M2TS files.
        // makemkvcon emits 2 "Backup failed/done" messages with different
        // message codes
        // -> hide the messages without a full-stop (MSG:5069 & MSG:5070)
        // -- success --
        // MSG:5070 - Backup done
        // MSG:5081 - Backup done.
        // -- failure --
        // MSG:5069 - Backup failed
        // MSG:5080 - Backup failed.
        let severity_map = HashMap::from([
            (1005, parser::SeverityLevel::Info),
            (1011, parser::SeverityLevel::Info),
            (5042, parser::SeverityLevel::Error),
            (5069, parser::SeverityLevel::Debug),
            (5070, parser::SeverityLevel::Debug),
            (5072, parser::SeverityLevel::Debug),
            (5080, parser::SeverityLevel::Error),
            (5081, parser::SeverityLevel::Info),
            (5085, parser::SeverityLevel::Info),
        ]);
        let parsed_output = parser::process_output(reader, min_length, &severity_map);
        if parsed_output.errors > 0 {
            result = false;
        }
    }

    let _ = child.wait();
    let time_end = std::time::Instant::now();
    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = monitor.join();

    let elapsed_seconds = time_end.duration_since(time_start).as_secs();

    if result {
        info!("The backup completed after {} seconds.", elapsed_seconds);
    } else {
        error!("The backup failed after {} seconds.", elapsed_seconds);
    }

    result
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

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Attributes {
    pub chapters: usize,
    pub duration: String,
    pub filename: String,
    pub filesize: usize,
    pub segments: Vec<String>,
}

fn parse_attributes(attributes: HashMap<String, InfoRecordOut>) -> Option<Attributes> {
    let mut chapter_count: Option<usize> = None;
    let mut disk_size_bytes: Option<usize> = None;
    let mut duration_hms: Option<String> = None;
    let mut segments_cnt: Option<usize> = None;
    let mut segments_map: Option<String> = None;
    let mut filename: Option<String> = None;
    for (attr_name, attr_record) in attributes {
        match attr_name.as_str() {
            // code: Unknown (0), value: '13'
            "ChapterCount" => {
                chapter_count = parse_as_usize(&attr_record.value);
            }
            // code: Unknown (0), value: B1
            "Comment" => {}
            // code: Unknown (0), value: 4.9 GB
            "DiskSize" => {}
            // code: Unknown (0), value: '5302022144'
            "DiskSizeBytes" => {
                disk_size_bytes = parse_as_usize(&attr_record.value);
            }
            // code: Unknown (0), value: 1:47:58
            "Duration" => {
                duration_hms = Some(attr_record.value);
            }
            // code: Unknown (0), value: '0'
            "OrderWeight" => {}
            // code: Unknown (0), value: '01'
            "OriginalTitleId" => {}
            // code: Unknown (0), value: B1_t00.mkv
            "OutputFileName" => {
                filename = Some(attr_record.value);
            }
            // code: AppInterfaceItemInfoTitle, value: <b>Title information</b><br>
            "PanelTitle" => {}
            // code: Unknown (0), value: '2'
            "SegmentsCount" => {
                segments_cnt = parse_as_usize(&attr_record.value);
            }
            // code: Unknown (0), value: 1-10,11-13
            "SegmentsMap" => {
                segments_map = Some(attr_record.value);
            }
            // code: Unknown (0), value: 13 chapter(s) , 4.9 GB (B1)
            "TreeInfo" => {}
            _ => {}
        }
    }

    // parse segments_map into a list and validate against segments_cnt
    let mut segments_normalized = Vec::new();
    if let Some(map) = segments_map {
        // split on commas, trim whitespace
        let parts: Vec<String> = map
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if let Some(expected) = segments_cnt
            && parts.len() != expected
        {
            panic!(
                "Invalid SegmentsMap: expected {} segments, found {} (value='{}')",
                expected,
                parts.len(),
                map
            );
        }
        segments_normalized = parts;
    }

    Some(Attributes {
        chapters: chapter_count.unwrap(),
        duration: duration_hms.unwrap(),
        filename: filename.unwrap(),
        filesize: disk_size_bytes.unwrap(),
        segments: segments_normalized,
    })
}

// ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Streams {
    pub audio: HashMap<usize, streams::AudioStream>,
    pub video: HashMap<usize, streams::VideoStream>,
    pub subtitle: HashMap<usize, streams::SubtitleStream>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Title {
    pub attributes: Attributes,
    pub streams: Streams,
}

// ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ParsedOutput {
    pub content: parser::ContentRecord,
    pub extract: HashMap<usize, Title>,
    pub makemkv: parser::MakeMkvRecord,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ScanResult {
    pub parsed: Option<ParsedOutput>,
    pub drives: Vec<DriveRecord>,
    pub issues: usize,
    pub errors: usize,
}

pub fn info(makemkvcon_bin: &PathBuf, source: &str, min_length: usize) -> Option<ScanResult> {
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
        // MSG:1011 - Using LibreDrive mode (v02.1 id=3F03CED516D5)
        // MSG:3007 - Using direct disc access mode
        // MSG:3028 - Title #1 was added (13 cell(s), 1:47:58)
        // MSG:3028 - Title #2 was added (7 cell(s), 0:06:07)
        // MSG:3025 - Title #3 has length of 20 seconds which is less than minimum title length of 120 seconds and was therefore skipped
        // MSG:3025 - Title #4 has length of 14 seconds which is less than minimum title length of 120 seconds and was therefore skipped
        // MSG:3038 - Cells 3-7 were removed from title end
        // <...>
        // MSG:5011 - Operation successfully completed
        // MSG:5014 - Saving 1 titles into directory file://extracted
        // MSG:2019 - Error 'OS error - The system cannot find the path specified' occurred while creating 'extracted/B1_t00.mkv'
        // MSG:2024 - Unknown device - 'D:'
        // MSG:5003 - Failed to save title 0 to file extracted/B1_t00.mkv
        // MSG:5004 - 0 titles saved, 1 failed
        // MSG:5010 - Failed to open disc
        // MSG:5037 - Copy complete. 0 titles saved, 1 failed.
        let mut severity_map = HashMap::from([
            (1005, parser::SeverityLevel::Info),
            (1011, parser::SeverityLevel::Info),
            (2019, parser::SeverityLevel::Error),
            (2024, parser::SeverityLevel::Error),
            (3007, parser::SeverityLevel::Info),
            (3025, parser::SeverityLevel::Info),
            (3028, parser::SeverityLevel::Info),
            (3038, parser::SeverityLevel::Info),
            (5003, parser::SeverityLevel::Error),
            (5004, parser::SeverityLevel::Error),
            (5010, parser::SeverityLevel::Error),
            (5011, parser::SeverityLevel::Info),
            (5014, parser::SeverityLevel::Info),
            (5037, parser::SeverityLevel::Info),
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

        let mut extract_map = HashMap::new();
        // process titles in sorted order
        // (sorted() requires crate itertools)
        for title_idx in parsed_output.content.titles.keys().sorted() {
            let title_record = parsed_output.content.titles[title_idx].clone();
            // for (title_idx, title_record) in parsed_output.content.titles.clone() {
            let attributes = match parse_attributes(title_record.attributes) {
                Some(x) => x,
                None => {
                    warn!(
                        "Internal error - unable to parse attributes of title '{}'! Skipping.",
                        title_idx
                    );
                    continue;
                }
            };
            let mut streams = Streams {
                audio: HashMap::new(),
                subtitle: HashMap::new(),
                video: HashMap::new(),
            };
            for (stream_idx, stream_record) in title_record.streams {
                match streams::parse_stream_record(stream_record.attributes) {
                    streams::Stream::Audio(audio_stream) => {
                        let mut duration = attributes.duration.clone();
                        if duration.len() == 7 {
                            // provided in 'H:MM:SS' format, e.g. '0:08:15'
                            // (add a leading zero or speedate won't parse)
                            duration = format!("0{}", attributes.duration);
                        };
                        // don't check titles shorter than 2 minutes
                        // (probably a menu or legal warning)
                        match speedate::Time::parse_str(&duration) {
                            // warn if languages aren't provided (may cause issues later on)
                            Ok(x) => {
                                if x.total_seconds() > 120 {
                                    if audio_stream.lang_code == "<unknown>" {
                                        if audio_stream.lang_name == "<unknown>" {
                                            warn!(
                                                "Audio stream {} in title {} has no \"LangCode\" or \"LangName\"!",
                                                stream_idx, title_idx
                                            );
                                        } else {
                                            warn!(
                                                "Audio stream {} in title {} has no \"LangCode\"!",
                                                stream_idx, title_idx
                                            );
                                        }
                                    } else if audio_stream.lang_name == "<unknown>" {
                                        warn!(
                                            "Audio stream {} in title {} has no \"LangName\"!",
                                            stream_idx, title_idx
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                warn!(
                                    "Unable to parse duration {} of title {}!",
                                    attributes.duration, title_idx
                                );
                            }
                        };
                        streams.audio.insert(stream_idx, audio_stream);
                    }
                    streams::Stream::Subtitle(subtitle_stream) => {
                        streams.subtitle.insert(stream_idx, subtitle_stream);
                    }
                    streams::Stream::Video(video_stream) => {
                        streams.video.insert(stream_idx, video_stream);
                    }
                }
            }
            extract_map.insert(
                *title_idx,
                Title {
                    attributes,
                    streams,
                },
            );
        }

        let parsed = ParsedOutput {
            content: parsed_output.content,
            extract: extract_map,
            makemkv: parsed_output.makemkv,
        };

        scan_result.parsed = Some(parsed);
        scan_result.issues += parsed_output.issues;
        scan_result.errors += parsed_output.errors;
    }
    let _ = child.wait();

    Some(scan_result)
}

pub fn mkv(
    makemkvcon_bin: &PathBuf,
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
        // MSG:3007 - Using direct disc access mode
        // MSG:3028 - Title #1 was added (13 cell(s), 1:47:58)
        // MSG:3028 - Title #2 was added (7 cell(s), 0:06:07)
        // MSG:3025 - Title #3 has length of 20 seconds which is less than minimum title length of 120 seconds and was therefore skipped
        // MSG:3025 - Title #4 has length of 14 seconds which is less than minimum title length of 120 seconds and was therefore skipped
        // MSG:3038 - Cells 3-7 were removed from title end
        // MSG:5014 - Saving 1 titles into directory file://extracted
        // <...>
        // -- success --
        // MSG:5005 - 1 titles saved
        // MSG:5011 - Operation successfully completed
        // MSG:5036 - Copy complete. 1 titles saved.
        // -- conflict --
        // MSG:5001 File ./B1_t00.mkv already exist. Do you want to overwrite it?
        // MSG:5005 1 titles saved
        // -- failure --
        // MSG:2019 - Error 'OS error - The system cannot find the path specified' occurred while creating 'extracted/B1_t00.mkv'
        // MSG:5003 - Failed to save title 0 to file extracted/B1_t00.mkv
        // MSG:5004 - 0 titles saved, 1 failed
        // MSG:5037 - Copy complete. 0 titles saved, 1 failed.
        let mut severity_map = HashMap::from([
            (1005, parser::SeverityLevel::Debug),
            (2019, parser::SeverityLevel::Error),
            (3007, parser::SeverityLevel::Debug),
            (3025, parser::SeverityLevel::Debug),
            (3028, parser::SeverityLevel::Debug),
            (3038, parser::SeverityLevel::Debug),
            (5001, parser::SeverityLevel::Warning),
            (5003, parser::SeverityLevel::Error),
            (5004, parser::SeverityLevel::Error),
            (5005, parser::SeverityLevel::Debug),
            (5011, parser::SeverityLevel::Debug),
            (5014, parser::SeverityLevel::Debug),
            (5036, parser::SeverityLevel::Debug),
            (5037, parser::SeverityLevel::Error),
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
        if parsed_output.errors > 0 {
            result = false;
        }
    }

    let _ = child.wait();

    result
}

// TODO makemkvcon f

// TODO makemkvcon reg
