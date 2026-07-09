/*
    extract data from a physical medium

    This code wraps 'makemkvcon backup' into a more convenient interface.
*/

mod os_utils;

use makemkv::{api::MessageRecord, medium};
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
    MSG:3338 - Downloading latest SDF to <homedir>/.MakeMKV ...
    MSG:1011 - Using LibreDrive mode (v06.3 id=0FA242DD4D0B)
    MSG:5042 - The program can't find any usable optical drives.
    MSG:5072 - Backing up disc into folder \file://<directory>\""
    MSG:5085 - Loaded content hash table, will verify integrity of M2TS files.

    // make sure to use LibreDrive:
    MSG:2003 - Error 'Scsi error - ILLEGAL REQUEST:READ OF SCRAMBLED SECTOR WITHOUT AUTHENTICATION' occurred while reading '<device>' at offset '1048576'
    // check disc for damage / dirt:
    MSG:2003 - Error 'Scsi error - MEDIUM ERROR:L-EC UNCORRECTABLE ERROR' occurred while reading '<device>' at offset '5326917632'

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
    2003u32 => MsgSeverity::Error,   // read error
    2008u32 => MsgSeverity::Warning, // storage slower than drive
    3338u32 => MsgSeverity::Info,
    5042u32 => MsgSeverity::Error, // no drives found
    5069u32 => MsgSeverity::Noise, // duplicates MSG:5080
    5070u32 => MsgSeverity::Noise, // duplicates MSG:5081
    5072u32 => MsgSeverity::Info,
    5080u32 => MsgSeverity::Error, // backup failed
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

    debug!("step 1 - identify drive id for drive '{}'", source_str);

    let drive = match find_drive_record(makemkvcon_bin, &source_str) {
        Some(dr) => {
            debug!("Found optical drive: {} (id: {})", dr.device_name, dr.index);
            dr
        }
        None => {
            return Err(UnshackleError::NoDrive);
        }
    };

    // --------------------------------------------------------------------
    // step 2 - identify medium (block id, volume label and content type)
    // --------------------------------------------------------------------
    // target_img - the filename used by makemkvcon (a file or directory)
    // target_log - the filename used for logging makemkvcon's output

    debug!("step 2 - identify medium in drive '{}'", source_str);

    let (volume_name, content_type) = match medium(makemkvcon_bin, &drive, 3) {
        Some(x) => x,
        None => {
            return Err(UnshackleError::NoMedium);
        }
    };

    let volume_id = match os_utils::get_volume_id(&source_str) {
        Some(x) => {
            debug!(
                "Drive '{}' contains a medium with volume ID '{}'.",
                source_str, x
            );
            x
        }
        None => {
            return Err(UnshackleError::NoMedium);
        }
    };

    // append volume ID to target in order to create a unique
    // filesystem location for each disc (this is a precaution
    // since the volume name may be something stupid like
    // "DVDVolume", "LOGICAL_VOLUME_ID" or "UNDEFINED" and cause
    // filesystem conflicts if multiple discs share the same name)
    let mut target_img = target.to_path_buf();
    if content_type.has_dvd_files {
        // DVDs are extracted as images
        target_img.push(format!("{}_{}.iso", volume_name, volume_id));
    } else if content_type.has_bluray_files {
        // BluRays are extracted as directories
        target_img.push(format!("{}_{}", volume_name, volume_id));
    } else {
        // TODO figure out how AACS, BDSVM, HD-DVD are extracted
        warn!("Don't know how to handle AACS, BDSVM, HD-DVD. Assuming ISO.");
        target_img.push(format!("{}_{}.iso", volume_name, volume_id));
    };

    // derive log filename from the image filename
    let mut target_log = target_img.clone();
    target_log.set_extension(".log");

    // ------------------------------------------------------------
    // step 3 - create log file
    // ------------------------------------------------------------

    debug!(
        "step 3 - create log file '{}'",
        target_log.to_string_lossy()
    );

    debug!("log file '{}'.", target_log.to_string_lossy());

    // creating the log file may fail if the parent directory does not exist
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

    // ------------------------------------------------------------
    // step 4 - prepare target
    // ------------------------------------------------------------

    debug!("step 3 - prepare target '{}'", target_img.to_string_lossy());

    if allow_overwrite && target_img.exists() {
        if target_img.is_dir() {
            info!(
                "Removing existing directory '{}'.",
                target_img.to_string_lossy()
            );
            std::fs::remove_dir_all(&target_img).expect("Failed to remove existing directory!");
        } else if target_img.is_file() {
            info!("Removing existing file '{}'.", target_img.to_string_lossy());
            std::fs::remove_file(&target_img).expect("Failed to remove existing file!");
        }
    }

    // ------------------------------------------------------------
    // initialize communication channel
    // ------------------------------------------------------------

    let (tx, rx) = mpsc::channel::<BackupEvent>();

    // ------------------------------------------------------------
    // step 5 - extract content
    // ------------------------------------------------------------

    debug!("step 3 - extract content");

    let time_start = std::time::Instant::now();
    let makemkvcon = makemkvcon_bin.to_path_buf();
    let disc_id = drive.index;

    info!("Extracting '{}' to '{}'.", source_str, target_str);
    let handle = std::thread::spawn(move || makemkv::backup(makemkvcon, disc_id, target_img, tx));

    #[allow(unused_assignments)]
    let mut result: Result<bool, UnshackleError> = Err(UnshackleError::ReadError);

    let mut have_libre_drive = false;
    let mut need_libre_drive = false;
    loop {
        match rx.recv() {
            Ok(BackupEvent::Message(msg)) => {
                // MSG:1011 - Using LibreDrive mode (v06.3 id=0FA242DD4D0B)
                if msg.code == 1011 && msg.params[0].starts_with("Using LibreDrive mode") {
                    have_libre_drive = true;
                }
                // MSG:2003 indicates many kinds of read errors, these may
                // be related to physical disc damage or copy protection
                if msg.code == 2003
                    && msg.params[0]
                        == "Scsi error - ILLEGAL REQUEST:READ OF SCRAMBLED SECTOR WITHOUT AUTHENTICATION"
                {
                    need_libre_drive = true;
                }
                match get_severity(msg.code) {
                    MsgSeverity::Noise => {
                        debug!("[makemkv] {}", msg.message);
                        log_msg(&mut fh, msg, 'I');
                    }
                    MsgSeverity::Info => {
                        info!("[makemkv] {}", msg.message);
                        log_msg(&mut fh, msg, 'I');
                    }
                    MsgSeverity::Warning => {
                        warn!("[makemkv] {}", msg.message);
                        log_msg(&mut fh, msg, 'W');
                    }
                    MsgSeverity::Error => {
                        error!("[makemkv] {}", msg.message);
                        log_msg(&mut fh, msg, 'E');
                    }
                    MsgSeverity::Unknown => {
                        warn!("[makemkv] {}", msg.message);
                        log_msg(&mut fh, msg, 'U');
                    }
                }
            }
            Ok(BackupEvent::Status(size, write_rate)) => {
                let size_in_gib = size as f64 / f64::powf(1024.0, 3.0);
                let write_rate_in_mib = write_rate as f64 / f64::powf(1024.0, 2.0);
                // the formatted output is designed to stay aligned and
                // provide a useful indication for the entire value range:
                // --------------------------------------------------------
                // Processed:  0.12 GiB (4.3 MiB/s)
                // Processed:  6.22 GiB (7.3 MiB/s)
                // Processed: 72.69 GiB (18.1 MiB/s)
                // --------------------------------------------------------
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

    if need_libre_drive && !have_libre_drive {
        warn!("Need LibreDrive to read this disc.");
        warn!("https://forum.makemkv.com/forum/viewtopic.php?t=18856 (What is LibreDrive?)");

        info!("Please retry extraction using 'LibreDrive':");
        #[cfg(target_os = "linux")]
        info!("- grant elevated privileges, e.g. use 'sudo'");
        #[cfg(target_os = "windows")]
        info!("- grant elevated privileges, e.g. 'Run as Administrator'");
        info!("- confirm your drive is LibreDrive compatible");
    }

    // ensure that the child process completes
    handle.join().unwrap();

    // --------------------------------------------------------------------
    // step 6 - eject disk and return
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
