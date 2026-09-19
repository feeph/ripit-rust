/*
    provide drive-related functions

    We're using the output of 'makemkvcon info' to identify which drives
    and discs were found by MakeMKV.
*/

mod drive;
mod os_utils;

// standard library imports
use std::path::PathBuf;
use std::time::Instant;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tokio::spawn;
use tokio::sync::mpsc;

// crate-provided imports
use makemkv::{ContentType, MakeMkvCli, MakeMkvEvent, ScanMode};
use os_utils::get_volume_id;

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub use drive::{BluRay, Disc, Drive, Dvd, HdDvd, IdleState, OpticalDrive};
pub use makemkv::DrvStatus as DriveStatus; // create an alias

// #[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
// pub struct Drive {
//     pub index: u8,
//     pub status: DriveStatus,
//     pub is_enabled: u32,
//     pub model: String,
//     pub device: PathBuf,
//     pub disc: Disc,
// }

// impl Drive {

//     /// eject the inserted medium
//     pub fn eject(&self) {
//         eject_medium(&self.device);
//     }

// }

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
pub async fn find_drives<'a>(mm: impl MakeMkvCli + std::marker::Send + 'static, scan_mode: ScanMode) -> Result<Vec<&'a impl OpticalDrive>, String> {

    let (tx, mut rx) = mpsc::channel::<MakeMkvEvent>(256);
    let start_time = Instant::now();
    let mut th = spawn(async move {mm.info("disc:-1".to_string(), scan_mode, tx).await});
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
                            println!("[makemkv] MSG:{} - {}", msg.code, msg.message);
                        }
                    },
                    MakeMkvEvent::DRV(drv) => {
                        debug!("find_drives(): Received a DRV event.");
                        let device = PathBuf::from(&drv.device_name);
                        let model = drv.drive_name;
                        let index = drv.index;
                        let is_enabled = drv.is_enabled;
                        // let drive_status = drv.drive_status.expect("Encountered an unknown drive status!");
                        let drive = Drive::new(device, model, index, is_enabled);
// TODO reimplement "drive.mark_as_ready(disc);"
// if drive_status == DriveStatus::DiscInserted {
//     let uid = get_volume_id(&drv.device_name).expect("Unable to determine volume id!");
//     let disc = parse_disc_type(drv.volume_name, uid, &drv.content_type);
//     drive = drive.mark_as_ready(disc);
// }
                        drives.push(&drive);
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
            info!("find_drives() finished normally after {} seconds. ({} drives found)", elapsed, drives.len());
            Ok(drives)
        },
        Ok(Err(makemkv::InfoError::DataError)) => {
            error!("find_drives() failed after {} seconds: DataError", elapsed);
            Err("DataError".to_string())
        },
        Err(e) => {
            error!("find_drives() failed after {} seconds: {}", elapsed, e);
            Err("GenericError".to_string())
        }
    }
}

pub fn find_matching_drive<'a>(drives_have: &'a [Drive<IdleState>], drive_want: &PathBuf) -> Option<&'a Drive<IdleState>> {
    debug!("find_matching_drives(): drives_have: {:#?}", drives_have);
    debug!("find_matching_drives(): drive_want:  {:#?}", drive_want);

    for drive in drives_have {
        if drive_want == &drive.device {
            // found the drive
            return Some(&drive);
        }
    }

    // did not find the drive
    None
}

pub fn find_matching_drives<'a>(drives_have: &'a [&impl OpticalDrive], drives_want: &[PathBuf]) -> Vec<&'a impl OpticalDrive> {
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
            if drives_want.contains(drive.get_name()) {
                drives.push(drive.clone());
            }
        }
    }

    // sort alphabetically by device name
    drives.sort_by(|a, b| b.get_name().cmp(&a.get_name()));

    // debug!("find_matching_drives(): drives: {:#?}", drives);
    drives
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

fn parse_disc_type(name: String, uid: String, content_type: &ContentType) -> Disc {
    if content_type.has_dvd_files {
        Disc::Dvd(Dvd{name, uid})
    } else if content_type.has_hddvd_files {
        Disc::HdDvd(HdDvd{name, uid})
    } else if content_type.has_bluray_files {
        Disc::BluRay(
            BluRay{
                name,
                uid,
                has_aacs: content_type.has_aacs_files,
                has_bdsvm: content_type.has_bdsvm_files,
            }
        )
    } else {
        Disc::NoDisc
    }
}

