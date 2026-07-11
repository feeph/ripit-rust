/*
    extract data from a physical medium

    This code wraps 'makemkvcon backup' into a more convenient interface.
*/

mod os_utils;

use makemkv::api::MessageRecord;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use makemkv::{BackupEvent, DriveRecord};

use crate::unshackle::Events::CreateBackup;

pub enum UnshackleError {
    NoDrivesFound,
    NoMedium,
    ReadError,
    LogError,
}

fn find_drive_record(makemkvcon_bin: &Path, source: &str) -> Option<DriveRecord> {
    let drive_records = makemkv::drives(makemkvcon_bin);

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

struct BackupTask {
    device_name: String,
    current_try: u8,
}

enum Events {
    DetectedDrives(HashMap<String, DriveRecord>),
    DetectedMedium((String, String)),
    CreateBackup((String, String)),
    BackupFailure(String),
    BackupSuccess(String),
    NeedLibreDrive(String),
}

async fn detect_drives(makemkvcon_bin: PathBuf, tx: Sender<Events>) {
    info!("detect_drives()");
    let mut drive_map = HashMap::new();

    // TODO consider using a map comprehension to make this a one-liner
    for drive in makemkv::drives(&makemkvcon_bin) {
        let drive_name = drive.drive_name.clone();
        drive_map.insert(drive_name, drive);
    }

    tx.send(Events::DetectedDrives(drive_map)).unwrap();
    info!("drives detected");
}

async fn detect_volume(device_name: String, tx: Sender<Events>) {
    info!("detect_volume()");

    match os_utils::get_volume_id(&device_name) {
        Some(volume_id) => {
            debug!(
                "Detected a medium with volume id '{}' in drive '{}'.",
                volume_id, device_name
            );
            tx.send(Events::DetectedMedium((device_name, volume_id)))
                .unwrap();
        }
        None => {
            debug!("Detected no medium in drive '{}'.", device_name)
        }
    };
}

async fn backup_volume(
    makemkvcon_bin: PathBuf,
    device_name: String,
    target: PathBuf,
    volume_id: String,
    allow_overwrite: bool,
    tx: Sender<Events>,
) {
    info!("backup_volume()");

    // This call is going to slow down all currently running backups but it
    // can't be avoided. We need to know the content type and that
    // information is part of the DRV record.
    let dr = find_drive_record(&makemkvcon_bin, &device_name).unwrap();

    let volume_name = dr.device_name;
    let content_type = dr.content_type;

    // ------------------------------------------------------------
    // step 1 - generate filenames
    // ------------------------------------------------------------

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
        warn!(
            "[{}] Don't know how to handle AACS, BDSVM, HD-DVD. Assuming ISO.",
            volume_id
        );
        target_img.push(format!("{}_{}.iso", volume_name, volume_id));
    };

    let mut target_log = target.to_path_buf();
    target_log.push(format!("{}_{}.log", volume_name, volume_id));

    // ------------------------------------------------------------
    // step 2 - create log file
    // ------------------------------------------------------------

    // creating the log file may fail if the parent directory does not exist
    let target_dir = target_log.parent().unwrap();
    if !std::path::Path::new(&target_dir).exists() {
        std::fs::create_dir(target_dir).unwrap();
    }

    let mut fh = match std::fs::File::create(&target_log) {
        Ok(fh) => {
            info!("[{}] Using log file '{}'.", device_name, volume_id);
            fh
        }
        Err(_) => {
            tx.send(Events::BackupFailure(volume_id)).unwrap();
            return;
        }
    };

    // ------------------------------------------------------------
    // step 3 - prepare target
    // ------------------------------------------------------------

    if allow_overwrite && target_img.exists() {
        if target_img.is_dir() {
            info!(
                "[{}] Removing existing directory '{}'.",
                volume_id,
                target_img.to_string_lossy()
            );
            std::fs::remove_dir_all(&target_img).expect("Failed to remove existing directory!");
        } else if target_img.is_file() {
            info!(
                "[{}] Removing existing file '{}'.",
                volume_id,
                target_img.to_string_lossy()
            );
            std::fs::remove_file(&target_img).expect("Failed to remove existing file!");
        }
    }

    // ------------------------------------------------------------
    // step 4 - extract content
    // ------------------------------------------------------------

    let (tx_bkp, rx_bkp) = mpsc::channel::<BackupEvent>();

    let makemkvcon = makemkvcon_bin.to_path_buf();
    let disc_id = dr.index;

    info!(
        "[{}] Extracting '{}' to '{}'.",
        volume_id,
        device_name,
        target_img.to_string_lossy()
    );

    let time_start = std::time::Instant::now();
    std::thread::spawn(move || makemkv::backup(makemkvcon, disc_id, target_img, tx_bkp));

    let mut have_libre_drive = false;
    let mut event_sent = false;
    loop {
        match rx_bkp.recv() {
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
                    && !have_libre_drive
                    && !event_sent
                {
                    tx.send(Events::NeedLibreDrive(device_name.clone()))
                        .unwrap();
                    // this message may occur multiple times - only send once
                    event_sent = true;
                }
                match get_severity(msg.code) {
                    MsgSeverity::Noise => {
                        debug!("[{}] {}", volume_id, msg.message);
                        log_msg(&mut fh, msg, 'I');
                    }
                    MsgSeverity::Info => {
                        info!("[{}] {}", volume_id, msg.message);
                        log_msg(&mut fh, msg, 'I');
                    }
                    MsgSeverity::Warning => {
                        warn!("[{}] {}", volume_id, msg.message);
                        log_msg(&mut fh, msg, 'W');
                    }
                    MsgSeverity::Error => {
                        error!("[{}] {}", volume_id, msg.message);
                        log_msg(&mut fh, msg, 'E');
                    }
                    MsgSeverity::Unknown => {
                        warn!("[{}] {}", volume_id, msg.message);
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
                let time_end = std::time::Instant::now();
                let elapsed_seconds = time_end.duration_since(time_start).as_secs();
                info!(
                    "[{}] The backup completed after {} seconds.",
                    volume_id, elapsed_seconds
                );
                tx.send(Events::BackupSuccess(volume_id)).unwrap();
                return;
            }
            Ok(BackupEvent::Failure) => {
                let time_end = std::time::Instant::now();
                let elapsed_seconds = time_end.duration_since(time_start).as_secs();
                error!(
                    "[{}] The backup failed after {} seconds.",
                    volume_id, elapsed_seconds
                );
                tx.send(Events::BackupFailure(volume_id)).unwrap();
                return;
            }
            Err(x) => {
                // unable to read from communication channel
                // (channel was closed without Success/Failure event?)
                error!("Internal error: {}", x)
            }
        }
    }
}

pub async fn unshackle_discs(
    makemkvcon_bin: &Path,
    drives: &[String],
    target: &PathBuf,
    eject_when_done: bool,
    allow_overwrite: bool,
    continuous: bool,
) -> Result<bool, UnshackleError> {
    info!("unshackle_discs()");

    // --------------------------------------------------------------------
    // initialize communication channel
    // --------------------------------------------------------------------

    let (tx, rx) = mpsc::channel::<Events>();

    // --------------------------------------------------------------------
    // handle events
    // --------------------------------------------------------------------
    // 'drives'      - what the user wants us to scan
    // 'drives_have' - all drives currently present in the system
    //                 (may change over time)
    // 'drives_scan' - all drives that should be used for imaging
    //                 (same as 'drives_have' if user didn't select any)

    let mut drives_have = HashMap::new();
    let mut drives_scan = Vec::<DriveRecord>::new();

    // volume_id -> (device, current_try)
    let mut todo_list = HashMap::<String, BackupTask>::new();
    let max_tries = 3;

    let mut need_drive_scan = false;

    loop {
        // trigger required functions:
        // - validate drives
        // - determine if drive contains medium
        // - start extraction
        // - create log
        // - cleanup if fail or clobber existing files
        // - eject
        // - repeat

        // detect drives (including hot-plugged USB drives)
        if drives_have.is_empty() || need_drive_scan {
            let my_mmc = PathBuf::from(makemkvcon_bin);
            let my_tx = tx.clone();
            detect_drives(my_mmc, my_tx).await;
            // ensure that the child process completes
            // handle.join().unwrap();
        }

        // determine whether a drive contains medium
        // (avoid 'makemkv info' since it's slow and triggers all drives)
        for drive in drives_scan.clone() {
            info!("Need to scan drive '{}'.", drive.device_name);
            let my_tx = tx.clone();
            detect_volume(drive.device_name, my_tx).await;
        }

        // react to events

        match rx.recv() {
            Ok(Events::DetectedDrives(drives_found)) => {
                drives_have = drives_found;
                if drives.is_empty() {
                    // populate 'drives_scan' with all drives
                    for (_, drive_record) in drives_have.clone() {
                        drives_scan.push(drive_record);
                    }
                } else {
                    // populate 'drives_scan' with matching drives
                    for (drive_name, drive_record) in drives_have.clone() {
                        if drives.contains(&drive_name) {
                            drives_scan.push(drive_record);
                        }
                    }
                }
            }
            Ok(Events::DetectedMedium((device_name, volume_id))) => {
                // - start extraction
                // - create log
                // - cleanup if fail or clobber existing files
                let my_id = volume_id.clone();
                let my_dn = device_name.clone();
                todo_list.insert(
                    my_id,
                    BackupTask {
                        device_name: my_dn,
                        current_try: 1,
                    },
                );
                tx.send(CreateBackup((device_name, volume_id))).unwrap();
            }
            Ok(Events::CreateBackup((device_name, volume_id))) => {
                let my_mmc = PathBuf::from(makemkvcon_bin);
                let my_tgt = PathBuf::from(target);
                let my_tx = tx.clone();
                backup_volume(
                    my_mmc,
                    device_name,
                    my_tgt,
                    volume_id,
                    allow_overwrite,
                    my_tx,
                )
                .await;
            }
            Ok(Events::BackupSuccess(volume_id)) => {
                let task = todo_list.remove(&volume_id).unwrap();
                if eject_when_done || continuous {
                    info!("Ejecting medium from '{}'.", task.device_name);
                    crate::eject::eject_medium(&PathBuf::from(task.device_name));
                }
            }
            Ok(Events::BackupFailure(volume_id)) => {
                let task = todo_list.get_mut(&volume_id).unwrap();
                if task.current_try < max_tries {
                    task.current_try += 1;
                    tx.send(CreateBackup((task.device_name.clone(), volume_id)))
                        .unwrap();
                } else {
                    warn!("Giving up and ejecting medium from '{}'.", task.device_name);
                    crate::eject::eject_medium(&PathBuf::from(&task.device_name));
                }
            }
            Ok(Events::NeedLibreDrive(device_name)) => {
                warn!("Need LibreDrive to read disc in drive '{}'!", device_name);
                warn!(
                    "https://forum.makemkv.com/forum/viewtopic.php?t=18856 (What is LibreDrive?)"
                );

                info!("Please retry extraction using 'LibreDrive':");
                #[cfg(target_os = "linux")]
                info!("- grant elevated privileges, e.g. use 'sudo'");
                #[cfg(target_os = "windows")]
                info!("- grant elevated privileges, e.g. 'Run as Administrator'");
                info!("- confirm your drive is LibreDrive compatible");
            }
            Err(x) => {
                // unable to read from communication channel
                // (channel was closed without Success/Failure event?)
                error!("Internal error: {}", x)
            }
        }

        // TODO "one shot" may terminate too quickly

        info!("exit, stage left");

        if drives_have.is_empty() {
            if continuous {
                need_drive_scan = true;
                // suspend and wait for drives to be hot-added
                std::thread::sleep(std::time::Duration::from_secs(5));
            } else {
                // signal an error condition
                return Err(UnshackleError::NoDrivesFound);
            }
        } else if !continuous {
            return Ok(true);
        }
    }
}
