/*!
    extract data from a physical medium

    This code wraps 'makemkvcon backup' into a more convenient interface.
*/

// standard library imports
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

    /// 'makemkvcon backup' failed in an unexpected way
    InternalError,
}

pub struct UnshackleError {
    source: String,
    prgt_code: u32,
    #[allow(dead_code)]
    prgc_code: u32,
    pub reason: UnshackleErrorReason,
    pub elapsed_secs: u64,
}

impl UnshackleError {
    // value must match ProgressTracker
    pub fn get_device_id(&self) -> String {
        self.source.clone()
    }

    // value must match ProgressTracker
    pub fn get_stage_id(&self) -> String {
        format!("{}_{}", self.source, self.prgt_code)
    }
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
                source: drive.device.to_string_lossy().to_string(),
                prgt_code: 0,
                prgc_code: 0,
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
    let th_mkv = spawn(async move {
        mm.backup(source_mkv, target_mkv_cpy, scan_mode, tx_mkv, logfile_mkv)
            .await
    });

    let device_str = drive.device.to_string_lossy();

    // PRGT and PRGC always come before PRGV
    // if at some point "<unknown>" shows up then this indicates makemkvcon
    // is doing something weird and unexpected -> check its output
    let mut pu = ProgressUpdate::new(&device_str, &target_upd, disc);

    let severity_map = default_severity_map();

    // monitor progress of worker tasks and terminate
    // parse incoming events until the spawned task finishes
    let mut events = Vec::new();
    let batch_size = 128;
    let mut ep = EventParser::new(&drive, &target, severity_map);
    while !th_mkv.is_finished() {
        rx_mkv.recv_many(&mut events, batch_size).await;
        ep.parse_events(&mut events, &mut pu, &tx).await;
    }
    let result = th_mkv.await;

    // process remaining events to ensure the queue is empty
    let remaining = rx_mkv.len();
    if remaining > 0 {
        debug!(
            "extract({}): Threads have finished. Draining remaining {} events.",
            device_str, remaining
        );
        let _ = rx_mkv.recv_many(&mut events, remaining).await;
        ep.parse_events(&mut events, &mut pu, &tx).await;
    } else {
        debug!(
            "extract({}): Threads have finished. No remaining events.",
            device_str
        );
    }

    // sanity check: this must never trigger
    // (if this condition triggers the code above does not work and skips
    // unprocessed messages)
    if !rx_mkv.is_empty() {
        panic!(
            "Internal error: Receiver queue still contains {} messages!",
            rx_mkv.len()
        );
    }

    // calculate runtime metrics and report them
    let elapsed = start_time.elapsed().as_secs();

    match &result {
        Ok(Ok(())) => {
            // `makemkvcon` has returned successfully but it might still
            // be a failure in disguise. Let's check in detail.
            if ep.get_error_count() == 0 {
                // enter happy path
                debug!(
                    "Completed backup of disc in drive '{}' after {} seconds.",
                    device_str, elapsed
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
                Ok(result)
            } else {
                debug!("Backup task failed after {} seconds. (1)", elapsed);
                Err(UnshackleError {
                    source: drive.device.to_string_lossy().to_string(),
                    prgt_code: pu.prgt.code,
                    prgc_code: pu.prgc.code,
                    reason: UnshackleErrorReason::BackupFailed,
                    elapsed_secs: elapsed,
                })
            }
        }
        Ok(Err(makemkv::BackupError::DataError)) => {
            // it's unclear under which conditions this could happen
            debug!("Backup task failed after {} seconds. (DataError)", elapsed);
            Err(UnshackleError {
                source: drive.device.to_string_lossy().to_string(),
                prgt_code: pu.prgt.code,
                prgc_code: pu.prgc.code,
                reason: UnshackleErrorReason::InternalError,
                elapsed_secs: elapsed,
            })
        }
        Err(_e) => {
            // something went seriously wrong, potentially a general issue
            // with the child process, e.g. unable to execute the binary
            debug!("unshackle_disc() encountered an issue with makemkvcon's child process.");
            Err(UnshackleError {
                source: drive.device.to_string_lossy().to_string(),
                prgt_code: pu.prgt.code,
                prgc_code: pu.prgc.code,
                reason: UnshackleErrorReason::InternalError,
                elapsed_secs: elapsed,
            })
        }
    }
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

struct EventParser {
    source: OpticalDrive,
    disc: OpticalDisc,

    // TODO consider using 'target'
    #[allow(dead_code)]
    target: PathBuf,

    errors: usize,
    severity_map: HashMap<u32, Severity>,
}

impl EventParser {
    fn new(source: &OpticalDrive, target: &Path, severity_map: HashMap<u32, Severity>) -> Self {
        EventParser {
            source: source.to_owned(),
            target: target.to_owned(),
            disc: source.disc.clone().expect("drive must contain a disc"),
            errors: 0,
            severity_map: severity_map.clone(),
        }
    }

    async fn parse_events(
        &mut self,
        events: &mut Vec<MakeMkvEvent>,
        pu: &mut ProgressUpdate,
        tx: &Sender<UnshackleEvent>,
    ) {
        for event in events.drain(..) {
            match event {
                MakeMkvEvent::MSG(msg) => {
                    let msg_code = msg.code;
                    let msg_type = self.severity_map.get(&msg_code);
                    let message = UnshackleMessage {
                        device: self.source.device.clone(),
                        disc: self.disc.clone(),
                        message: msg,
                    };
                    let event = match msg_type {
                        Some(Severity::Info) => {
                            debug!(
                                "[{}] MSG:{} - {}",
                                self.source.device.to_string_lossy(),
                                msg_code,
                                message.message.message
                            );
                            UnshackleEvent::MsgInfo(message)
                        }
                        Some(Severity::Warn) => {
                            debug!(
                                "[{}] MSG:{} - {}",
                                self.source.device.to_string_lossy(),
                                msg_code,
                                message.message.message
                            );
                            UnshackleEvent::MsgWarn(message)
                        }
                        Some(Severity::Fail) => {
                            self.errors += 1;
                            debug!(
                                "[{}] MSG:{} - {}",
                                self.source.device.to_string_lossy(),
                                msg_code,
                                message.message.message
                            );
                            UnshackleEvent::MsgFail(message)
                        }
                        // unknown
                        None => UnshackleEvent::MsgWarn(message),
                    };
                    tx.send(event).await.unwrap();
                }
                MakeMkvEvent::DRV(_) => {
                    // ignore all DRV events
                }
                MakeMkvEvent::TCOUNT(_) => {
                    // ignore all TCOUNT events
                }
                MakeMkvEvent::CINFO(_) => {
                    warn!("Found an unexpected CINFO record during 'makemkvcon backup'! Ignoring.");
                }
                MakeMkvEvent::TINFO(_) => {
                    warn!("Found an unexpected TINFO record during 'makemkvcon backup'! Ignoring.");
                }
                MakeMkvEvent::SINFO(_) => {
                    warn!("Found an unexpected SINFO record during 'makemkvcon backup'! Ignoring.");
                }
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
                    tx.send(UnshackleEvent::ProgressT(pu.clone()))
                        .await
                        .unwrap();
                }
                MakeMkvEvent::PRGC(prgc) => {
                    info!("[PRGC:{}] {} {}", prgc.code, prgc.name, prgc.id);

                    // update 'progress current' values
                    pu.prgc.code = prgc.code;
                    pu.prgc.name = prgc.name.clone();
                    pu.prgc.percentage = 0.0;

                    // send event with updated values
                    tx.send(UnshackleEvent::ProgressC(pu.clone()))
                        .await
                        .unwrap();
                }
                MakeMkvEvent::PRGV(prgv) => {
                    info!(
                        "[PRGV] current: {} total: {} maximum: {}",
                        prgv.current, prgv.total, prgv.maximum
                    );

                    // update 'progress current' values
                    pu.prgt.percentage = 100.0 * (prgv.total as f32) / (prgv.maximum as f32);
                    pu.prgc.percentage = 100.0 * (prgv.current as f32) / (prgv.maximum as f32);

                    // send event with updated values
                    tx.send(UnshackleEvent::ProgressValue(pu.clone()))
                        .await
                        .unwrap();
                }
            }
        }
    }

    fn get_error_count(&self) -> usize {
        self.errors
    }
}
