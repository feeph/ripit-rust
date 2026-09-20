/*!
    provide drive-related functions

    We're using the output of 'makemkvcon info' to identify which drives
    and discs were found by MakeMKV.
*/

mod optical_drive;

// standard library imports
use std::path::PathBuf;
use std::time::Instant;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tokio::spawn;
use tokio::sync::mpsc;

// crate-provided imports
use makemkv::{MakeMkvCli, MakeMkvEvent, ScanMode};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub use makemkv::DrvStatus as DriveStatus;
pub use optical_drive::{BluRay, Dvd, HdDvd, OpticalDisc, OpticalDrive}; // create an alias

/// returns all drives recognized by MakeMKV
/// (expected output: 0 to 15 drives)
///
///   - the available drives may change over time, e.g. after adding
///     or removing a USB drive
///   - adding/removing a SATA drive is possible (may require a bus scan),
///     e.g.: `echo "- - -" > /sys/class/scsi_host/host1/scan`
///
/// Setting 'scan_drives' to false greatly improves the speed of this
/// operation at the cost of not knowing if a medium is inserted. It is
/// recommended to set this parameter to false whenever you are interested
/// exclusively in the drive and don't need to identify if there's a media
/// loaded and if it's a DVD, HD-DVD or Blu-Ray.
///
///   - 3 drives with media and no scanning:  ~1.4 seconds
///   - 3 drives with media and scanning:     ~16  seconds
pub async fn find_drives(
    mm: impl MakeMkvCli + std::marker::Send + 'static,
    scan_mode: ScanMode,
) -> Result<Vec<OpticalDrive>, String> {
    let (tx, mut rx) = mpsc::channel::<MakeMkvEvent>(256);
    let start_time = Instant::now();
    let mut th = spawn(async move { mm.info("disc:-1".to_string(), scan_mode, tx).await });
    let mut result = None;

    let mut drives = Vec::new();

    // parse incoming events until the spawned task finishes
    loop {
        tokio::select! {
            // parse generated events
            Some(event) = rx.recv() => {
                match event {
                    MakeMkvEvent::MSG(msg) => {
                        if msg.code == 1005 {
                            // MSG:1005 - MakeMKV v1.18.4 linux(x64-release) started
                            info!("[makemkv] MSG:{} - {}", msg.code, msg.message);
                        } else if msg.code == 5010 {
                            // MSG:5010 - Failed to open disc
                            // Always hide MSG:5010 since we intentionally
                            // specified an invalid device for this
                            // operation.
                        } else {
                            // show everything else that remains
                            // (shouldn't print anything)
                            info!("[makemkv] MSG:{} - {}", msg.code, msg.message);
                        }
                    },
                    MakeMkvEvent::DRV(drv) => {
                        debug!("find_drives(): Received a DRV event.");
                        let drive = OpticalDrive::from(drv);
                        drives.push(drive);
                    },
                    // the following outputs are returned by makemkvcon but
                    // completely irrelevant to finding drives -> hide them
                    MakeMkvEvent::TCOUNT(title_count) => {
                        debug!("[TCOUNT] {}", title_count);
                    },
                    MakeMkvEvent::CINFO(_) => {
                        warn!("Found an unexpected CINFO record during 'makemkvcon backup'! Ignoring.");
                    },
                    MakeMkvEvent::TINFO(_) => {
                        warn!("Found an unexpected TINFO record during 'makemkvcon backup'! Ignoring.");
                    },
                    MakeMkvEvent::SINFO(_) => {
                        warn!("Found an unexpected SINFO record during 'makemkvcon backup'! Ignoring.");
                    },
                    MakeMkvEvent::PRGC(prgc) => {
                        debug!("[PRGC:{}] {} {}", prgc.code, prgc.name, prgc.id);
                    },
                    MakeMkvEvent::PRGT(prgt) => {
                        debug!("[PRGT:{}] {} {}", prgt.code, prgt.name, prgt.id);
                    },
                    MakeMkvEvent::PRGV(prgv) => {
                        debug!("[PRGV] current: {} total: {} maximum: {}", prgv.current, prgv.total, prgv.maximum);
                    },
                }
            },
            // record completion and drain buffered events before returning
            res = &mut th, if result.is_none() => {
                result = Some(res);
            },
            else => break,
        }
    }

    // calculate total runtime and report it
    // (`makemkvcon info` may take anything between seconds and minutes,
    // depending on how many drives are attached, a disc is present, the
    // disc is scanned and/or some other process is currently reading.)
    let elapsed = start_time.elapsed().as_secs();

    match result.expect("`makemkvcon info` failed to run!") {
        Ok(Ok(())) => {
            info!(
                "find_drives() finished normally after {} seconds. ({} drives found)",
                elapsed,
                drives.len()
            );
            Ok(drives)
        }
        Ok(Err(makemkv::InfoError::DataError)) => {
            error!("find_drives() failed after {} seconds: DataError", elapsed);
            Err("DataError".to_string())
        }
        Err(e) => {
            error!("find_drives() failed after {} seconds: {}", elapsed, e);
            Err("GenericError".to_string())
        }
    }
}

pub fn find_matching_drive(
    drives_have: &[OpticalDrive],
    drive_want: &PathBuf,
) -> Option<OpticalDrive> {
    debug!("find_matching_drives(): drives_have: {:#?}", drives_have);
    debug!("find_matching_drives(): drive_want:  {:#?}", drive_want);

    for drive in drives_have {
        if drive_want == &drive.device {
            // found the drive
            return Some(drive.to_owned());
        }
    }

    // did not find the drive
    None
}

pub fn find_matching_drives(
    drives_have: &[OpticalDrive],
    drives_want: &[PathBuf],
) -> Vec<OpticalDrive> {
    // debug!("find_matching_drives(): drives_have: {:#?}", drives_have);
    // debug!("find_matching_drives(): drives_want: {:#?}", drives_want);
    let mut drives = Vec::new();

    if drives_want.is_empty() {
        // user specified no drives, use all available drives
        drives.append(&mut drives_have.to_vec());
    } else {
        // user specified one or more drives, use them
        // (the drive might -currently- not be present)
        for drive in drives_have {
            if drives_want.contains(&PathBuf::from(&drive.device)) {
                drives.push(drive.clone());
            }
        }
    }

    // sort alphabetically by device name
    drives.sort_by(|a, b| b.device.cmp(&a.device));

    // debug!("find_matching_drives(): drives: {:#?}", drives);
    drives
}

// ------------------------------------------------------------------------
// helper functions
// ------------------------------------------------------------------------

// <none>

// ------------------------------------------------------------------------
// unit tests
// ------------------------------------------------------------------------

// <none>
