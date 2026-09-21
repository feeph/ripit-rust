/*
    extract titles from a physical disc or disc image

    This code wraps 'makemkvcon mkv' into a more convenient interface.
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
use crate::drives::{BluRay, Dvd, OpticalDisc, OpticalDrive};
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
    pub source: String,
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
            return Err(ExtractError {
                reason: ExtractErrorReason::NoDisc,
                elapsed_secs: 0,
            });
        }
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
            debug!(
                "Completed extraction of disc in drive '{}'.",
                source.device.to_string_lossy()
            );
            if eject_when_done {
                info!(
                    "Ejecting disc from drive '{}'.",
                    source.device.to_string_lossy()
                );
                source.eject_disc();
            }
        }
        Err(_) => {
            debug!(
                "Failed to extract disc in drive '{}'!",
                source.device.to_string_lossy()
            );
        }
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

    // MSG:5042 - The program can't find any usable optical drives.
    // Since we're reading from an image there's no drive needed.
    // --> Reduce severity from warning to informational.
    severity_map.insert(5042, Severity::Info);

    let source_upd = source_mkv.clone();
    let disc = if source.is_file() {
        OpticalDisc::Dvd(Dvd {
            name: source
                .file_stem()
                .expect("no filename?")
                .to_string_lossy()
                .to_string(),
            uid: "deadbeef".to_string(),
        })
    } else if source.is_dir() {
        OpticalDisc::BluRay(BluRay {
            name: source
                .file_stem()
                .expect("no filename?")
                .to_string_lossy()
                .to_string(),
            uid: "deadbeef".to_string(),
            has_aacs: false,
            has_bdsvm: false,
        })
    } else {
        return Err(ExtractError {
            reason: ExtractErrorReason::NoDisc,
            elapsed_secs: 0,
        });
    };

    let target_upd = target.to_string_lossy().to_string();

    // PRGT and PRGC always come before PRGV
    // if at some point "<unknown>" shows up then this indicates makemkvcon
    // is doing something weird and unexpected -> check its output
    let pu = ProgressUpdate::new(&source_upd, &target_upd, &disc);

    let result = extract(mm, source_mkv, target, titles, pu, tx, severity_map).await;
    match &result {
        Ok(_) => {
            debug!(
                "Completed extraction of disc image '{}'.",
                source.to_string_lossy()
            );
        }
        Err(_) => {
            debug!(
                "Failed to extract disc image '{}'!",
                source.to_string_lossy()
            );
        }
    }

    result
}

async fn extract(
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
            }
            Err(_e) => {
                debug!("Failed to create directory {:#?}!", target);
                return Err(ExtractError {
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
    let th_mkv = spawn(async move {
        mm.mkv(
            source_mkv,
            titles_mkv,
            target_mkv,
            scan_mode,
            tx_mkv,
            Some(logfile),
        )
        .await
    });

    // parse incoming events until the spawned task finishes
    let mut events = Vec::new();
    let batch_size = 128;
    let mut ep = EventParser::new(&source, &target, severity_map);
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
            source, remaining
        );
        let _ = rx_mkv.recv_many(&mut events, remaining).await;
        ep.parse_events(&mut events, &mut pu, &tx).await;
    } else {
        debug!(
            "extract({}): Threads have finished. No remaining events.",
            source
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

    // unsure if this is needed
    drop(tx);

    // record completion and drain buffered events before returning
    match result {
        Ok(Ok(_result)) => {
            // TODO derive 'title_count_have' from result (disc structure)
            let title_count_have: usize = 0;
            let title_count_want = ep.get_title_count();
            if title_count_have == title_count_want {
                debug!(
                    "Extracted {} titles in {} seconds.",
                    title_count_have, elapsed
                );
                let fs_size = calculate_filesystem_size(&target);
                Ok(ExtractResult {
                    source: source.clone(),
                    title_count: title_count_have,
                    elapsed_secs: elapsed,
                    fs_size,
                })
            } else {
                debug!(
                    "Title count mismatch: (have: {} != want: {})",
                    title_count_have, title_count_want
                );
                Err(ExtractError {
                    reason: ExtractErrorReason::TitleMismatch,
                    elapsed_secs: elapsed,
                })
            }
        }
        Ok(Err(_err)) => {
            warn!("task failed (1)");
            Err(ExtractError {
                reason: ExtractErrorReason::ExtractFailed,
                elapsed_secs: elapsed,
            })
        }
        Err(_err) => {
            warn!("task failed (2)");
            Err(ExtractError {
                reason: ExtractErrorReason::ExtractFailed,
                elapsed_secs: elapsed,
            })
        }
    }
}

struct EventParser {
    source: String,
    target: PathBuf,
    title_count_want: usize,
    severity_map: HashMap<u32, Severity>,
}

impl EventParser {
    fn new(source: &str, target: &Path, severity_map: HashMap<u32, Severity>) -> Self {
        EventParser {
            source: source.to_owned(),
            target: target.to_owned(),
            title_count_want: 0,
            severity_map: severity_map.clone(),
        }
    }

    async fn parse_events(
        &mut self,
        events: &mut Vec<MakeMkvEvent>,
        pu: &mut ProgressUpdate,
        tx: &Sender<ExtractEvent>,
    ) {
        for event in events.drain(..) {
            match event {
                MakeMkvEvent::MSG(msg) => {
                    let msg_code = msg.code;
                    let msg_type = self.severity_map.get(&msg_code);
                    let message = ExtractMessage {
                        source: self.source.clone(),
                        target: self.target.clone(),
                        message: msg.clone(),
                    };
                    let event = match msg_type {
                        Some(Severity::Info) => {
                            debug!(
                                "[{}] MSG:{} - {}",
                                self.source, msg_code, message.message.message
                            );
                            ExtractEvent::MsgInfo(message)
                        }
                        Some(Severity::Warn) => {
                            debug!(
                                "[{}] MSG:{} - {}",
                                self.source, msg_code, message.message.message
                            );
                            ExtractEvent::MsgWarn(message)
                        }
                        Some(Severity::Fail) => {
                            debug!(
                                "[{}] MSG:{} - {}",
                                self.source, msg_code, message.message.message
                            );
                            ExtractEvent::MsgFail(message)
                        }
                        // unknown
                        None => ExtractEvent::MsgWarn(message),
                    };
                    tx.send(event).await.unwrap();
                }
                MakeMkvEvent::DRV(_) => {
                    // ignore all DRV events
                }
                MakeMkvEvent::TCOUNT(tc) => {
                    // ignore all TCOUNT events
                    self.title_count_want = tc;
                }
                MakeMkvEvent::CINFO(_cinfo) => {
                    // TODO process CINFO record
                }
                MakeMkvEvent::TINFO(_tinfo) => {
                    // TODO process TINFO record
                }
                MakeMkvEvent::SINFO(_sinfo) => {
                    // TODO process SINFO record
                }
                MakeMkvEvent::PRGT(prgt) => {
                    info!("[PRGT:{}] {} {}", prgt.code, prgt.name, prgt.id);

                    // update 'progress total' values
                    pu.prgt.code = prgt.code;
                    pu.prgt.name = prgt.name;
                    pu.prgt.percentage = f32::NAN;

                    // reset 'progress current' values
                    pu.prgc.code = 0;
                    pu.prgc.name = "<n/a>".to_string();
                    pu.prgc.percentage = f32::NAN;

                    // send event with updated values
                    tx.send(ExtractEvent::ProgressT(pu.clone())).await.unwrap();
                }
                MakeMkvEvent::PRGC(prgc) => {
                    info!("[PRGC:{}] {} {}", prgc.code, prgc.name, prgc.id);

                    // update 'progress current' values
                    pu.prgc.code = prgc.code;
                    pu.prgc.name = prgc.name.clone();
                    pu.prgc.percentage = f32::NAN;

                    // send event with updated values
                    tx.send(ExtractEvent::ProgressC(pu.clone())).await.unwrap();
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
                    tx.send(ExtractEvent::ProgressValue(pu.clone()))
                        .await
                        .unwrap();
                }
            }
        }
    }

    fn get_title_count(&self) -> usize {
        self.title_count_want
    }
}
