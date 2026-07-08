/*
    extract data from a physical medium

    This code wraps 'makemkvcon backup' into a more convenient interface.
*/

mod os_utils;

use makemkv::api::MessageRecord;
use std::io::Write;
use std::path::Path;
use std::sync::mpsc;

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use makemkv::{BackupEvent, DriveRecord};

pub enum UnshackleError {
    NoDrive,
    NoMedium,
    ReadError,
    LogError,
}

fn find_drive_record(makemkvcon_bin: &Path, source: &str) -> Option<DriveRecord> {
    let (drive_records, _) = makemkv::drives(makemkvcon_bin);

    // find the drive matching the provided device name
    // - provides mapping from device name to disc id (required for backup)
    // - provides disc content type
    drive_records
        .into_iter()
        .find(|dr| dr.device_name == source)
}

#[derive(Clone)]
enum MsgSeverity {
    Error,
    Warning,
    Info,
    Noise,
    Unknown,
}

/*
    Create a static HashMap for mapping message codes to severity levels.

    Severities are context-dependent, e.g. 2008 (HDD too slow) and 5042 (no
    optical drive) are irrelevant when reading from a disc image but must
    be treated as an issue when reading from an optical drive.

    MSG:1005 - MakeMKV v1.18.3 win(x64-release) started
    MSG:1011 - Using LibreDrive mode (v02.1 id=3F03CED516D5)
    MSG:5042 - The program can't find any usable optical drives.
    MSG:5072 - Backing up disc into folder \file://<directory>\""
    MSG:5085 - Loaded content hash table, will verify integrity of M2TS files.

    makemkvcon emits 2 "Backup failed/done" messages with different
    message codes
    -> hide the messages without a full-stop (MSG:5069 & MSG:5070)
    -- failure --
    MSG:5069 - Backup failed
    MSG:5080 - Backup failed.
    -- success --
    MSG:5070 - Backup done
    MSG:5081 - Backup done.
*/
static SEVERITY_MAP: phf::Map<u32, MsgSeverity> = phf::phf_map! {
    1005u32 => MsgSeverity::Info,
    1011u32 => MsgSeverity::Info,
    2008u32 => MsgSeverity::Warning,
    5042u32 => MsgSeverity::Error,
    5069u32 => MsgSeverity::Noise,
    5070u32 => MsgSeverity::Noise,
    5072u32 => MsgSeverity::Info,
    5080u32 => MsgSeverity::Error,
    5081u32 => MsgSeverity::Info,
    5085u32 => MsgSeverity::Info,
};

fn get_severity(msg_code: u32) -> MsgSeverity {
    SEVERITY_MAP
        .get(&msg_code)
        .unwrap_or(&MsgSeverity::Unknown)
        .clone()
}

fn log_msg(fh: &mut std::fs::File, msg: MessageRecord, severity: char) {
    let dt_now = chrono::Local::now();
    let timestamp = dt_now.format("%Y-%m-%d %H:%M:%S");

    writeln!(
        fh,
        "[{}] {}: MSG:{} {}",
        timestamp, severity, msg.code, msg.message
    )
    .unwrap();
}

pub fn unshackle_disc(
    makemkvcon_bin: &Path,
    source: &Path,
    target: &Path,
    eject_when_done: bool,
    allow_overwrite: bool,
) -> Result<bool, UnshackleError> {
    let source_str = source.to_string_lossy().into_owned();
    let target_str = target.to_string_lossy().to_string();

    // --------------------------------------------------------------------
    // step 1 - scan drives
    // --------------------------------------------------------------------
    // Performing 'scan drive' before 'identify medium' allows us to
    // distinguish between:
    //   a) the user made an error and the specified drive does not exist
    //   b) the drive exists but there is no medium or it is unreadable

    debug!(
        "step 1 - identify drive id for optical drive '{}'",
        source_str
    );

    // find the drive matching the provided device name or drive letter
    // - provides mapping from device name to disc id (required for backup)
    // - provides disc content type
    let drive = match find_drive_record(makemkvcon_bin, &source_str) {
        Some(dr) => {
            debug!(
                "Found optical drive: {} -> {}",
                dr.device_name, dr.disc_name
            );
            dr
        }
        None => {
            return Err(UnshackleError::NoDrive);
        }
    };

    // --------------------------------------------------------------------
    // step 2 - identify medium (volume label and block id)
    // --------------------------------------------------------------------

    debug!("step 2 - identify medium in drive '{}'", source_str);

    let mut target_mkv = target.to_path_buf();
    let result = os_utils::get_volume_id(&source_str);
    match result {
        Some(x) => {
            debug!(
                "Drive '{}' contains a medium with volume ID '{}'.",
                source_str, x
            );
            // append volume ID to target in order to create a unique
            // filesystem location for each disc (this is a precaution
            // since the volume name may be something stupid like
            // "DVDVolume", "LOGICAL_VOLUME_ID" or "UNDEFINED" and cause
            // filesystem conflicts if multiple discs share the same name)
            target_mkv.push(&x);
        }
        None => {
            return Err(UnshackleError::NoMedium);
        }
    }

    let mut target_log = target_mkv.clone();
    target_log.add_extension("log");

    debug!("log file '{}'.", target_log.to_string_lossy());

    // file creation may fail if parent dir does not exist
    let target_dir = target_log.parent().unwrap();
    if !std::path::Path::new(&target_dir).exists() {
        std::fs::create_dir(target_dir).unwrap();
    }

    let mut fh = match std::fs::File::create(&target_log) {
        Ok(fh) => {
            info!("Using log file '{}'.", target_log.to_string_lossy());
            fh
        }
        Err(_) => {
            return Err(UnshackleError::LogError);
        }
    };

    if drive.content_type.has_dvd_files {
        // DVDs are extracted as images
        target_mkv.add_extension("iso");
    }

    // ------------------------------------------------------------
    // step 3 - prepare target
    // ------------------------------------------------------------

    debug!("step 3 - prepare target '{}'", target_mkv.to_string_lossy());

    if allow_overwrite && target_mkv.exists() {
        if target_mkv.is_dir() {
            info!(
                "Removing existing directory '{}'.",
                target_mkv.to_string_lossy()
            );
            std::fs::remove_dir_all(&target_mkv).expect("Failed to remove existing directory");
        } else if target_mkv.is_file() {
            info!("Removing existing file '{}'.", target_mkv.to_string_lossy());
            std::fs::remove_file(&target_mkv).expect("Failed to remove existing file");
        }
    }

    // ------------------------------------------------------------
    // initialize communication channel
    // ------------------------------------------------------------

    let (tx, rx) = mpsc::channel::<BackupEvent>();

    // ------------------------------------------------------------
    // step 4 - extract content
    // ------------------------------------------------------------

    debug!("step 3 - extract content");

    let time_start = std::time::Instant::now();
    let makemkvcon = makemkvcon_bin.to_path_buf();
    let disc_id = drive.index;

    info!("Extracting '{}' to '{}'.", source_str, target_str);
    let handle = std::thread::spawn(move || makemkv::backup(makemkvcon, disc_id, target_mkv, tx));

    #[allow(unused_assignments)]
    let mut result: Result<bool, UnshackleError> = Err(UnshackleError::ReadError);

    loop {
        match rx.recv() {
            Ok(BackupEvent::Message(msg)) => match get_severity(msg.code) {
                MsgSeverity::Noise => {
                    debug!("[makemkv] {}", msg.message);
                    log_msg(&mut fh, msg, 'I')
                }
                MsgSeverity::Info => {
                    info!("[makemkv] {}", msg.message);
                    log_msg(&mut fh, msg, 'I')
                }
                MsgSeverity::Warning => {
                    warn!("[makemkv] {}", msg.message);
                    log_msg(&mut fh, msg, 'W')
                }
                MsgSeverity::Error => {
                    error!("[makemkv] {}", msg.message);
                    log_msg(&mut fh, msg, 'E')
                }
                MsgSeverity::Unknown => {
                    warn!("[makemkv] {}", msg.message);
                    log_msg(&mut fh, msg, 'U')
                }
            },
            Ok(BackupEvent::Status(size, write_rate)) => {
                let size_in_gib = size as f64 / f64::powf(1024.0, 3.0);
                let write_rate_in_mib = write_rate as f64 / f64::powf(1024.0, 2.0);
                info!(
                    "Processed: {:5.2} GiB ({:.1} MiB/s)",
                    size_in_gib, write_rate_in_mib
                );
            }
            Ok(BackupEvent::Success) => {
                result = Ok(true);
                break;
            }
            Ok(BackupEvent::Failure) => {
                result = Err(UnshackleError::ReadError);
                break;
            }
            Err(x) => {
                // unable to read from communication channel
                // (channel was closed without Success/Failure event?)
                error!("Internal error: {}", x)
            }
        }
    }

    // ensure that the child process completes
    handle.join().unwrap();

    // --------------------------------------------------------------------
    // step 5 - eject disk and return
    // --------------------------------------------------------------------

    debug!("step 3 - eject disk and return");

    let time_end = std::time::Instant::now();
    let elapsed_seconds = time_end.duration_since(time_start).as_secs();

    match result {
        Ok(_) => {
            info!("The backup completed after {} seconds.", elapsed_seconds);
            if eject_when_done {
                info!("Ejecting medium from '{}'.", source_str);
                crate::eject::eject_medium(source);
            }
        }
        Err(_) => {
            error!("The backup failed after {} seconds.", elapsed_seconds);
        }
    }

    result
}
