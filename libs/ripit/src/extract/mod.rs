/*
    extract titles from a physical disc or disc image

    This code wraps 'makemkvcon mkv' into a more convenient interface.
*/

// standard library imports
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tokio::spawn;
use tokio::sync::mpsc::{self, Sender};

// crate-provided imports
use crate::drives::{Dvd, BluRay, OpticalDisc, OpticalDrive};
use crate::os_utils::calculate_filesystem_size;
use crate::progress_update::ProgressUpdate;
use crate::severities::{Severity, default_severity_map};
use makemkv::{MakeMkv, MakeMkvCli, MakeMkvEvent, MsgRecord, ScanMode};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

#[derive(Debug)]
pub struct ExtractMessage {
    pub source: String,
    pub target: PathBuf,
    pub message: MsgRecord,
}

#[derive(Debug)]
pub enum ExtractEvent {
    MsgInfo(ExtractMessage),
    MsgWarn(ExtractMessage),
    MsgFail(ExtractMessage),
    ProgressT(ProgressUpdate),
    ProgressC(ProgressUpdate),
    ProgressValue(ProgressUpdate),
}

#[derive(Debug)]
pub struct ExtractResult {
    pub elapsed_secs: u64,
    pub fs_size: u64,
    pub title_count: usize,
}

#[derive(Debug)]
pub enum ExtractErrorReason {
    NoDisc,
    FilesystemIssue,
    ExtractFailed,
    TitleMismatch,
}

#[derive(Debug)]
pub struct ExtractError {
    pub reason: ExtractErrorReason,
    pub elapsed_secs: u64,
}

pub async fn extract_from_drive(
    mm: impl MakeMkvCli + std::marker::Send + 'static,
    source: OpticalDrive,
    target: PathBuf,
    titles: Vec<String>,
    eject_when_done: bool,
    tx: Sender<ExtractEvent>,
) -> Result<ExtractResult, ExtractError> {
    info!("extract_from_drive()");

    let disc = match &source.disc {
        Some(disc) => disc,
        None => {
            // makes no sense to continue without a disc
            return Err(
                ExtractError {
                    reason: ExtractErrorReason::NoDisc,
                    elapsed_secs: 0,
                }
            );
        },
    };

    let source_mkv = format!("dev:{}", source.device.to_string_lossy());

    let severity_map = default_severity_map();

    let source_upd = source_mkv.clone();
    let target_upd = target.to_string_lossy().to_string();

    // PRGT and PRGC always come before PRGV
    // if at some point "<unknown>" shows up then this indicates makemkvcon
    // is doing something weird and unexpected -> check its output
    let pu = ProgressUpdate::new(&source_upd, &target_upd, disc);

    let result = extract(mm, source_mkv, target, titles, pu, tx, severity_map).await;
    match &result {
        Ok(_) => {
            debug!("Completed extraction of disc in drive '{}'.", source.device.to_string_lossy());
            if eject_when_done {
                info!("Ejecting disc from drive '{}'.", source.device.to_string_lossy());
                source.eject_disc();
            }
        },
        Err(_) => {
            debug!("Failed to extract disc in drive '{}'!", source.device.to_string_lossy());
        },
    }

    result
}

pub async fn extract_from_image(
    mm: MakeMkv,
    source: PathBuf,
    target: PathBuf,
    titles: Vec<String>,
    tx: Sender<ExtractEvent>,
) -> Result<ExtractResult, ExtractError> {
    info!("extract_from_image()");

    let source_mkv = if source.is_dir() {
        format!("file:{}", source.to_string_lossy())
    } else {
        format!("iso:{}", source.to_string_lossy())
    };

    let mut severity_map = default_severity_map();

    // MSG:2008 - Program reads data faster than it can write to disk, <...>
    // Reduce severity from 'Warning' to 'Info' because we're reading from
    // an image, not an optical drive.
    severity_map.insert(2008, Severity::Info);

    let source_upd = source_mkv.clone();
    let disc = if source.is_file() {
        OpticalDisc::Dvd(
            Dvd{
                name: source.file_stem().expect("no filename?").to_string_lossy().to_string(),
                uid: "deadbeef".to_string(),
            }
        )
    } else if source.is_dir() {
        OpticalDisc::BluRay(
            BluRay{
                name: source.file_stem().expect("no filename?").to_string_lossy().to_string(),
                uid: "deadbeef".to_string(),
                has_aacs: false,
                has_bdsvm: false,
            }
        )
    }else {
        return Err(
            ExtractError {
                reason: ExtractErrorReason::NoDisc,
                elapsed_secs: 0,
            }
        );
    };
    
    let target_upd = target.to_string_lossy().to_string();

    // PRGT and PRGC always come before PRGV
    // if at some point "<unknown>" shows up then this indicates makemkvcon
    // is doing something weird and unexpected -> check its output
    let pu = ProgressUpdate::new(&source_upd, &target_upd, &disc);

    let result = extract(mm, source_mkv, target, titles, pu, tx, severity_map).await;
    match &result {
        Ok(_) => {
            debug!("Completed extraction of disc image '{}'.", source.to_string_lossy());
        },
        Err(_) => {
            debug!("Failed to extract disc image '{}'!", source.to_string_lossy());
        },
    }

    result
}

async fn extract (
    mm: impl MakeMkvCli + std::marker::Send + 'static,
    source: String,
    target: PathBuf,
    titles: Vec<String>,
    mut pu: ProgressUpdate,
    tx: Sender<ExtractEvent>,
    severity_map: HashMap<u32, Severity>,
) -> std::result::Result<ExtractResult, ExtractError> {
    info!("extract(): source = {}", source);

    // ------------------------------------------------------------
    // step 1 - prepare target
    // ------------------------------------------------------------

    if !target.exists() {
        match std::fs::create_dir_all(&target) {
            Ok(_) => {
                debug!("Created directory {:#?}.", target);
                // all good
            },
            Err(_e) => {
                println!("Failed to create directory {:#?}!", target);
                return Err(ExtractError{
                    reason: ExtractErrorReason::FilesystemIssue,
                    elapsed_secs: 0,
                });
            }
        }
    }

    // let logfile = target.join("extract.log");
    let logfile = target.with_added_extension("log");
    info!("Using logfile '{}'.", logfile.to_string_lossy());

    // ------------------------------------------------------------
    // step 2 - extract content
    // ------------------------------------------------------------

    let (tx_mkv, mut rx_mkv) = mpsc::channel::<MakeMkvEvent>(256);

    let source_mkv = source.clone();
    let titles_mkv = titles.join(",");
    let target_mkv = target.clone();
    let scan_mode = ScanMode::DriveOnly;
    let start_time = Instant::now();
    let mut th_mkv = spawn(async move {
        mm.mkv(source_mkv, titles_mkv, target_mkv, scan_mode, tx_mkv, Some(logfile)).await
    });

    // monitor progress of worker tasks and terminate
    // parse incoming events until the spawned task finishes
    let mut result = None;
    let mut title_count_want = 0;
    let mut title_count_have = 0;
    loop {
        tokio::select! {
            // parse generated events
            Some(event) = rx_mkv.recv() => {
                match event {
                    MakeMkvEvent::MSG(msg) => {
                        let msg_code = msg.code;
                        let msg_type = severity_map.get(&msg_code);
                        let message = ExtractMessage {
                            source: source.clone(),
                            target: target.clone(),
                            message: msg,
                        };
                        let event = match msg_type {
                            Some(Severity::Info) => {
                                debug!("[{}] MSG:{} - {}", source, msg_code, message.message.message);
                                ExtractEvent::MsgInfo(message)
                            },
                            Some(Severity::Warn) => {
                                debug!("[{}] MSG:{} - {}", source, msg_code, message.message.message);
                                ExtractEvent::MsgWarn(message)
                            },
                            Some(Severity::Fail) => {
                                debug!("[{}] MSG:{} - {}", source, msg_code, message.message.message);
                                ExtractEvent::MsgFail(message)
                            },
                            // unknown
                            None => {
                                ExtractEvent::MsgWarn(message)
                            },
                        };
                        tx.send(event).await.unwrap();
                    },
                    MakeMkvEvent::DRV(_) => {
                        // ignore all DRV events
                    },
                    MakeMkvEvent::TCOUNT(tc) => {
                        // ignore all TCOUNT events
                        title_count_want = tc;
                    },
                    MakeMkvEvent::CINFO(_cinfo) => {
                        // TODO process CINFO record
                    },
                    MakeMkvEvent::TINFO(_tinfo) => {
                        // TODO process TINFO record
                    },
                    MakeMkvEvent::SINFO(_sinfo) => {
                        // TODO process SINFO record
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
                        tx.send(ExtractEvent::ProgressT(pu.clone())).await.unwrap();
                    },
                    MakeMkvEvent::PRGC(prgc) => {
                        info!("[PRGC:{}] {} {}", prgc.code, prgc.name, prgc.id);

                        // update 'progress current' values
                        pu.prgc.code = prgc.code;
                        pu.prgc.name = prgc.name.clone();
                        pu.prgc.percentage = 0.0;

                        // send event with updated values
                        tx.send(ExtractEvent::ProgressC(pu.clone())).await.unwrap();
                    },
                    MakeMkvEvent::PRGV(prgv) => {
                        info!("[PRGV] current: {} total: {} maximum: {}", prgv.current, prgv.total, prgv.maximum);

                        // update 'progress current' values
                        pu.prgt.percentage = 100.0 * (prgv.total as f32) / (prgv.maximum as f32);
                        pu.prgc.percentage = 100.0 * (prgv.current as f32) / (prgv.maximum as f32);
                        
                        // send event with updated values
                        tx.send(ExtractEvent::ProgressValue(pu.clone())).await.unwrap();
                    },
                }
            },
            // record completion and drain buffered events before returning
            res = &mut th_mkv, if result.is_none() => {
                result = Some(res);
            },
            else => break,
        }
    }

    // calculate runtime metrics and report them
    let elapsed = start_time.elapsed().as_secs();

    // unsure if this is needed
    drop(tx);

    // record completion and drain buffered events before returning
    match result.expect("Exited the loop without a result!") {
        Ok(Ok(_result)) => {
            if title_count_have == title_count_want {
                debug!("Extracted {} titles in {} seconds.", title_count_have, elapsed);
                let fs_size = calculate_filesystem_size(&target);
                Ok(ExtractResult{title_count: title_count_have, elapsed_secs: elapsed, fs_size})
            } else {
                debug!("Title count mismatch: (have: {} != want: {})", title_count_have, title_count_want);
                Err(
                    ExtractError {
                        reason: ExtractErrorReason::TitleMismatch,
                        elapsed_secs: elapsed,
                    }
                )
            }
        },
        Ok(Err(_err)) => {
            warn!("task failed (1)");
            Err(
                ExtractError {
                    reason: ExtractErrorReason::ExtractFailed,
                    elapsed_secs: elapsed,
                }
            )
        },
        Err(_e) => {
            warn!("task failed (2)");
            Err(
                ExtractError {
                    reason: ExtractErrorReason::ExtractFailed,
                    elapsed_secs: elapsed,
                }
            )
        },
    }
}
