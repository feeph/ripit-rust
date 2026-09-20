/*
    extract data from a physical medium

    This code wraps 'makemkvcon backup' into a more convenient interface.
*/

// standard library imports
use std::path::PathBuf;
use std::time::Instant;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tokio::spawn;
use tokio::sync::mpsc::{self, Sender};

// crate-provided imports
use crate::drives::{OpticalDisc, OpticalDrive};
use crate::os_utils::calculate_filesystem_size;
use crate::progress_update::ProgressUpdate;
use crate::severities::{Severity, default_severity_map};
use makemkv::{MakeMkvCli, MakeMkvEvent, MsgRecord, ScanMode};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

#[derive(Debug)]
pub struct UnshackleMessage {
    pub device: PathBuf,
    pub disc: OpticalDisc,
    pub message: MsgRecord,
}

#[derive(Debug)]
pub enum UnshackleEvent {
    MsgInfo(UnshackleMessage),
    MsgWarn(UnshackleMessage),
    MsgFail(UnshackleMessage),
    ProgressT(ProgressUpdate),
    ProgressC(ProgressUpdate),
    ProgressValue(ProgressUpdate),
}

pub struct UnshackleResult {
    pub elapsed_secs: u64,
    pub fs_size: u64,
}

#[derive(Debug, PartialEq)]
pub enum UnshackleErrorReason {
    /// no disc inserted or unable to read it
    NoDisc,

    /// disc is present but couldn't be read
    ReadError,

    /// 'makemkvcon backup' failed with error
    BackupFailed,

    /// unable to tell if the backup succeeded or failed
    UnknownResult,

    /// 'makemkvcon backup' failed in an unexpected way
    InternalError,
}

pub struct UnshackleError {
    pub reason: UnshackleErrorReason,
    pub elapsed_secs: u64,
}

#[derive(Debug, PartialEq)]
enum BackupState {
    Running,
    Success,
    Failure,
    Unknown,
}

/// The optical drive must be in "Ready" state. The drive's state changes
/// while unshackle is running. When returning the drive will be back in
/// "Ready" state. (This is enforced by Rust's type system.)
pub async fn unshackle_disc(
    mm: impl MakeMkvCli + std::marker::Send + 'static,
    drive: OpticalDrive,
    target: PathBuf,
    allow_overwrite: bool,
    eject_when_done: bool,
    tx: Sender<UnshackleEvent>,
) -> Result<UnshackleResult, UnshackleError> {
    info!(
        "unshackle_disc(): {} -> {}",
        drive.device.to_string_lossy(),
        target.to_string_lossy()
    );

    let source_mkv = format!("disc:{}", drive.index);
    let source_upd = source_mkv.clone();
    info!("unshackle_disc(): source_mkv {}", source_mkv);

    // ------------------------------------------------------------
    // step 1 - prepare target
    // ------------------------------------------------------------

    info!("unshackle_disc(): drive {:#?}", drive);

    let disc = match &drive.disc {
        Some(disc) => disc,
        None => {
            // makes no sense to continue without a disc
            return Err(UnshackleError {
                reason: UnshackleErrorReason::NoDisc,
                elapsed_secs: 0,
            });
        }
    };

    let filename = match &disc {
        OpticalDisc::Dvd(x) => {
            format!("{}_{}.iso", x.name, x.uid)
        }
        OpticalDisc::HdDvd(x) => {
            format!("{}_{}.iso", x.name, x.uid)
        }
        OpticalDisc::BluRay(x) => {
            format!("{}_{}", x.name, x.uid)
        }
    };
    info!("unshackle_disc(): filename {}", filename);

    let target_mkv = target.join(filename);
    let target_upd = target_mkv.to_string_lossy().to_string();

    info!(
        "unshackle_disc(): {} -> {}",
        source_mkv,
        target_mkv.to_string_lossy()
    );

    if allow_overwrite && target_mkv.exists() {
        info!(
            "Found existing backup file '{}'.",
            target_mkv.to_string_lossy()
        );
        if target_mkv.is_dir() {
            info!(
                "[{}] Removing existing directory '{}'.",
                drive.device.to_string_lossy(),
                target_mkv.to_string_lossy()
            );
            std::fs::remove_dir_all(&target_mkv).expect("Failed to remove existing directory!");
        } else if target_mkv.is_file() {
            info!(
                "[{}] Removing existing file '{}'.",
                drive.device.to_string_lossy(),
                target_mkv.to_string_lossy()
            );
            std::fs::remove_file(&target_mkv).expect("Failed to remove existing file!");
        }
    }

    let mut logfile = target_mkv.clone();
    logfile.set_extension("log");
    info!("Using logfile '{}'.", logfile.to_string_lossy());

    // ------------------------------------------------------------
    // step 2 - extract content
    // ------------------------------------------------------------

    let (tx_mkv, mut rx_mkv) = mpsc::channel::<MakeMkvEvent>(256);
    let logfile_mkv = Some(logfile.clone());

    let target_mkv_cpy = target_mkv.clone();

    let scan_mode = ScanMode::DriveOnly;

    let start_time = Instant::now();
    let mut th_mkv = spawn(async move {
        mm.backup(source_mkv, target_mkv_cpy, scan_mode, tx_mkv, logfile_mkv)
            .await
    });

    // PRGT and PRGC always come before PRGV
    // if at some point "<unknown>" shows up then this indicates makemkvcon
    // is doing something weird and unexpected -> check its output
    let mut pu = ProgressUpdate::new(&source_upd, &target_upd, disc);

    let severity_map = default_severity_map();

    // monitor progress of worker tasks and terminate
    // parse incoming events until the spawned task finishes
    let mut result = None;
    let mut backup_state = BackupState::Running;
    loop {
        tokio::select! {
            // parse generated events
            Some(event) = rx_mkv.recv() => {
                match event {
                    MakeMkvEvent::MSG(msg) => {
                        let msg_code = msg.code;
                        let msg_type = severity_map.get(&msg_code);
                        let msg_text = msg.message.clone();
                        let message = UnshackleMessage{
                            device: drive.device.clone(),
                            disc: disc.clone(),
                            message: msg,
                        };
                        let event = match msg_type {
                            Some(Severity::Info) => {
                                debug!("[{}] MSG:{} - {}", drive.device.to_string_lossy(), msg_code, message.message.message);
                                UnshackleEvent::MsgInfo(message)
                            },
                            Some(Severity::Warn) => {
                                debug!("[{}] MSG:{} - {}", drive.device.to_string_lossy(), msg_code, message.message.message);
                                UnshackleEvent::MsgWarn(message)
                            },
                            Some(Severity::Fail) => {
                                debug!("[{}] MSG:{} - {}", drive.device.to_string_lossy(), msg_code, message.message.message);
                                UnshackleEvent::MsgFail(message)
                            },
                            // unknown
                            None => {
                                UnshackleEvent::MsgWarn(message)
                            },
                        };
                        tx.send(event).await.unwrap();
                        match msg_code {
                            // MSG:5010 - Failed to open disc
                            5010 => {
                                error!("{}", msg_text);
                                backup_state = BackupState::Failure;
                            }
                            // MSG:5070 - Backup done
                            // MSG:5081 - Backup done.
                            5070 | 5081 => {
                                if backup_state == BackupState::Failure {
                                    // sanity check failed
                                    error!("Inconsistent state: Saw both 'Success' and 'Failure'?!");
                                    backup_state = BackupState::Unknown;
                                } else {
                                    debug!("Saw '{}' -> Marking as success.", msg_text);
                                    backup_state = BackupState::Success;
                                };
                            },
                            // MSG:5069 - Backup failed
                            // MSG:5080 - Backup failed.
                            5069 | 5080 => {
                                if backup_state == BackupState::Success {
                                    // sanity check failed
                                    error!("Inconsistent state: Saw both 'Success' and 'Failure'?!");
                                    backup_state = BackupState::Unknown;
                                } else {
                                    debug!("Saw '{}' -> Marking as failure.", msg_text);
                                    backup_state = BackupState::Failure;
                                };
                            },
                            _ => {
                                // do nothing
                            }
                        }
                    },
                    MakeMkvEvent::DRV(_) => {
                        // ignore all DRV events
                    },
                    MakeMkvEvent::TCOUNT(_) => {
                        // ignore all TCOUNT events
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
                    MakeMkvEvent::PRGT(prgt) => {
                        info!("[PRGT:{}] {} {}", prgt.code, prgt.name, prgt.id);

                        // update 'progress total' values
                        pu.prgt.code = prgt.code;
                        pu.prgt.name = prgt.name.clone();
                        pu.prgt.percentage = 0.0;

                        // reset 'progress current' values
                        pu.prgc.code = 0;
                        pu.prgc.name = "<n/a>".to_string();
                        pu.prgc.percentage = 0.0;

                        // send event with updated values
                        tx.send(UnshackleEvent::ProgressT(pu.clone())).await.unwrap();
                    },
                    MakeMkvEvent::PRGC(prgc) => {
                        info!("[PRGC:{}] {} {}", prgc.code, prgc.name, prgc.id);

                        // update 'progress current' values
                        pu.prgc.code = prgc.code;
                        pu.prgc.name = prgc.name.clone();
                        pu.prgc.percentage = 0.0;

                        // send event with updated values
                        tx.send(UnshackleEvent::ProgressC(pu.clone())).await.unwrap();
                    },
                    MakeMkvEvent::PRGV(prgv) => {
                        info!("[PRGV] current: {} total: {} maximum: {}", prgv.current, prgv.total, prgv.maximum);

                        // update 'progress current' values
                        pu.prgt.percentage = 100.0 * (prgv.total as f32) / (prgv.maximum as f32);
                        pu.prgc.percentage = 100.0 * (prgv.current as f32) / (prgv.maximum as f32);

                        // send event with updated values
                        tx.send(UnshackleEvent::ProgressValue(pu.clone())).await.unwrap();
                    },
                }
            },
            // record completion and drain buffered events before returning
            res = &mut th_mkv, if result.is_none() => {
                result = Some(res);
            },
            else => {
                break;
            },
        }
    }

    // calculate runtime metrics and report them
    let elapsed = start_time.elapsed().as_secs();

    let err_reason: UnshackleErrorReason;
    match &result.expect("`makemkvcon backup` failed to run!") {
        Ok(Ok(())) => {
            // `makemkvcon` has returned successfully but it might still
            // be a failure in disguise. Let's check in detail.
            match backup_state {
                BackupState::Running => {
                    error!(
                        "`makemkvcon backup` has run but didn't send MSG:5069, MSG:5070, MSG:5080 or MSG:5081!"
                    );
                    error!(
                        "Something is seriously wrong, please check logfile '{}'.",
                        logfile.to_string_lossy()
                    );
                    err_reason = UnshackleErrorReason::BackupFailed;
                }
                BackupState::Success => {
                    debug!(
                        "Completed backup of disc in drive '{}' after {} seconds.",
                        drive.device.to_string_lossy(),
                        elapsed
                    );
                    if eject_when_done {
                        debug!("Ejecting disc.");
                        drive.eject_disc();
                    }

                    // update device state and return Ok()
                    let fs_size = calculate_filesystem_size(&target_mkv);
                    let result = UnshackleResult {
                        elapsed_secs: elapsed,
                        fs_size,
                    };
                    return Ok(result);
                }
                BackupState::Failure => {
                    debug!("Backup task failed after {} seconds. (1)", elapsed);
                    err_reason = UnshackleErrorReason::BackupFailed;
                }
                BackupState::Unknown => {
                    error!("`makemkvcon backup` has run and claimed both success and failure?!");
                    error!(
                        "Something is seriously wrong, please check logfile '{}'.",
                        logfile.to_string_lossy()
                    );
                    err_reason = UnshackleErrorReason::UnknownResult;
                }
            }
        }
        Ok(Err(makemkv::BackupError::DataError)) => {
            // it's unclear under which conditions this could happen
            debug!("Backup task failed after {} seconds. (DataError)", elapsed);
            err_reason = UnshackleErrorReason::InternalError;
        }
        Err(_e) => {
            // something went seriously wrong, potentially a general issue
            // with the child process, e.g. unable to execute the binary
            debug!("unshackle_disc() encountered an issue with makemkvcon's child process.");
            err_reason = UnshackleErrorReason::InternalError;
        }
    }

    // update device state and return Err()
    let error = UnshackleError {
        reason: err_reason,
        elapsed_secs: elapsed,
    };
    Err(error)
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

// <none>
