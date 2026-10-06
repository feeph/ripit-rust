/*!
    show structure of a physical medium or disc image

    This code wraps 'makemkvcon info' into a more convenient interface.
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
use crate::progress_update::ProgressUpdate;
use crate::severities::{Severity, default_severity_map};
use makemkv::{DiscContent, MakeMkv, MakeMkvCli, MakeMkvEvent, MsgRecord, ScanMode};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

#[derive(Debug)]
pub struct ScanMessage {
    pub source: String,
    pub target: PathBuf,
    pub message: MsgRecord,
}

#[derive(Debug)]
pub enum ScanEvent {
    MsgInfo(ScanMessage),
    MsgWarn(ScanMessage),
    MsgFail(ScanMessage),
    ProgressT(ProgressUpdate),
    ProgressC(ProgressUpdate),
    ProgressValue(ProgressUpdate),
}

#[derive(Debug)]
pub struct ScanResult {
    pub source: String,
    pub disc_content: DiscContent,
    pub elapsed_secs: u64,
}

#[derive(Debug)]
pub enum ScanErrorReason {
    NoDisc,
    FilesystemIssue,
    ScanFailed,
    TitleMismatch,
}

#[derive(Debug)]
pub struct ScanError {
    pub reason: ScanErrorReason,
    pub elapsed_secs: u64,
}

pub async fn scan_drive(
    mm: impl MakeMkvCli + std::marker::Send + 'static,
    source: OpticalDrive,
    eject_when_done: bool,
    tx: Sender<ScanEvent>,
) -> Result<ScanResult, ScanError> {
    info!("scan_from_drive()");

    let disc = match &source.disc {
        Some(disc) => disc,
        None => {
            // makes no sense to continue without a disc
            return Err(ScanError {
                reason: ScanErrorReason::NoDisc,
                elapsed_secs: 0,
            });
        }
    };

    let source_mkv = format!("dev:{}", source.device.to_string_lossy());

    let severity_map = default_severity_map();

    let source_upd = source_mkv.clone();

    // PRGT and PRGC always come before PRGV
    // if at some point "<unknown>" shows up then this indicates makemkvcon
    // is doing something weird and unexpected -> check its output
    let pu = ProgressUpdate::new(&source_upd, "<none>", disc);

    let result = scan(mm, source_mkv, pu, tx, severity_map).await;
    match &result {
        Ok(_) => {
            debug!(
                "Completed scan of disc in drive '{}'.",
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
                "Failed to scan disc in drive '{}'!",
                source.device.to_string_lossy()
            );
        }
    }

    result
}

pub async fn scan_image(
    mm: MakeMkv,
    source: PathBuf,
    tx: Sender<ScanEvent>,
) -> Result<ScanResult, ScanError> {
    info!("scan_from_image()");

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
        })
    } else if source.is_dir() {
        OpticalDisc::BluRay(BluRay {
            name: source
                .file_stem()
                .expect("no filename?")
                .to_string_lossy()
                .to_string(),
            has_aacs: false,
            has_bdsvm: false,
        })
    } else {
        return Err(ScanError {
            reason: ScanErrorReason::NoDisc,
            elapsed_secs: 0,
        });
    };

    // PRGT and PRGC always come before PRGV
    // if at some point "<unknown>" shows up then this indicates makemkvcon
    // is doing something weird and unexpected -> check its output
    let pu = ProgressUpdate::new(&source_upd, "<none>", &disc);

    let result = scan(mm, source_mkv, pu, tx, severity_map).await;
    match &result {
        Ok(_) => {
            debug!(
                "Completed extraction of disc image '{}'.",
                source.to_string_lossy()
            );
        }
        Err(_) => {
            debug!("Failed to scan disc image '{}'!", source.to_string_lossy());
        }
    }

    result
}

async fn scan(
    mm: impl MakeMkvCli + std::marker::Send + 'static,
    source: String,
    mut pu: ProgressUpdate,
    tx: Sender<ScanEvent>,
    severity_map: HashMap<u32, Severity>,
) -> std::result::Result<ScanResult, ScanError> {
    info!("scan(): source = {}", source);

    // ------------------------------------------------------------
    // step 1 - scan content
    // ------------------------------------------------------------

    let (tx_mkv, mut rx_mkv) = mpsc::channel::<MakeMkvEvent>(256);

    let source_mkv = source.clone();
    let scan_mode = ScanMode::DriveOnly;
    let start_time = Instant::now();
    let th_mkv = spawn(async move { mm.info(source_mkv, scan_mode, tx_mkv).await });

    // parse incoming events until the spawned task finishes
    let mut events = Vec::new();
    let batch_size = 128;
    let mut ep = EventParser::new(&source, &PathBuf::new(), severity_map);
    while !th_mkv.is_finished() {
        rx_mkv.recv_many(&mut events, batch_size).await;
        ep.parse_events(&mut events, &mut pu, &tx).await;
    }
    let result = th_mkv.await;

    // process remaining events to ensure the queue is empty
    let remaining = rx_mkv.len();
    if remaining > 0 {
        debug!(
            "scan({}): Threads have finished. Draining remaining {} events.",
            source, remaining
        );
        let _ = rx_mkv.recv_many(&mut events, remaining).await;
        ep.parse_events(&mut events, &mut pu, &tx).await;
    } else {
        debug!(
            "scan({}): Threads have finished. No remaining events.",
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
        Ok(Ok(dc)) => {
            // TODO derive 'title_count_have' from result (disc structure)
            let title_count_have: usize = dc.titles.len();
            let title_count_want = ep.get_title_count();
            if title_count_have == title_count_want {
                debug!(
                    "Scanned {} titles in {} seconds.",
                    title_count_have, elapsed
                );
                Ok(ScanResult {
                    source: source.clone(),
                    disc_content: dc,
                    elapsed_secs: elapsed,
                })
            } else {
                debug!(
                    "Title count mismatch: (have: {} != want: {})",
                    title_count_have, title_count_want
                );
                error!(
                    ">>> Title count mismatch: (have: {} != want: {})",
                    title_count_have, title_count_want
                );
                Err(ScanError {
                    reason: ScanErrorReason::TitleMismatch,
                    elapsed_secs: elapsed,
                })
            }
        }
        Ok(Err(_err)) => {
            warn!("task failed (1)");
            Err(ScanError {
                reason: ScanErrorReason::ScanFailed,
                elapsed_secs: elapsed,
            })
        }
        Err(_err) => {
            warn!("task failed (2)");
            Err(ScanError {
                reason: ScanErrorReason::ScanFailed,
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
        tx: &Sender<ScanEvent>,
    ) {
        for event in events.drain(..) {
            match event {
                MakeMkvEvent::MSG(msg) => {
                    let msg_code = msg.code;
                    let msg_type = self.severity_map.get(&msg_code);
                    let message = ScanMessage {
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
                            ScanEvent::MsgInfo(message)
                        }
                        Some(Severity::Warn) => {
                            debug!(
                                "[{}] MSG:{} - {}",
                                self.source, msg_code, message.message.message
                            );
                            ScanEvent::MsgWarn(message)
                        }
                        Some(Severity::Fail) => {
                            debug!(
                                "[{}] MSG:{} - {}",
                                self.source, msg_code, message.message.message
                            );
                            ScanEvent::MsgFail(message)
                        }
                        // unknown
                        None => ScanEvent::MsgWarn(message),
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
                    tx.send(ScanEvent::ProgressT(pu.clone())).await.unwrap();
                }
                MakeMkvEvent::PRGC(prgc) => {
                    info!("[PRGC:{}] {} {}", prgc.code, prgc.name, prgc.id);

                    // update 'progress current' values
                    pu.prgc.code = prgc.code;
                    pu.prgc.name = prgc.name.clone();
                    pu.prgc.percentage = f32::NAN;

                    // send event with updated values
                    tx.send(ScanEvent::ProgressC(pu.clone())).await.unwrap();
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
                    tx.send(ScanEvent::ProgressValue(pu.clone())).await.unwrap();
                }
            }
        }
    }

    fn get_title_count(&self) -> usize {
        self.title_count_want
    }
}
