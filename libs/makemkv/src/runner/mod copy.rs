/*
    abstraction layer for executing the 'makemkvcon' command and parsing
    its output while its running

    This code in this file is intended for exclusive use within this
    library and may change at any point. Consumers of this library are
    expected to use the public functions 'makemkv::backup()',
    'makemkv::info()' and 'makemkv::mkv()'.
*/

mod disc_content;
mod source;
mod streams;

// standard library imports
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tokio::io::{BufReader, AsyncBufReadExt};
use tokio::process::Command;
use tokio::spawn;
use tokio::sync::mpsc::{self, Sender};

use crate::DrvStatus;
// crate-provided imports
use crate::api::{DrvRecord, MsgRecord, ProgressCurrentRecord, ProgressTotalRecord, ProgressValueRecord};
use crate::parser::{ParsedOutputLine, parse_output_line};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub use disc_content::{DiscContent, UpdateError};
pub use source::{Source, parse_source};
pub use streams::{StreamRecord, parse_stream_attributes};

#[derive(Clone, Debug, PartialEq)]
pub enum OutputType {
    StdOut,
    StdErr,
    Silent,
    File(PathBuf)
}

#[derive(Clone, Debug, PartialEq)]
pub enum DirectIO {
    Enabled,
    Disabled,
}

#[derive(Debug)]
pub enum MakeMkvEvent {
    DRV(DrvRecord),
    MSG(MsgRecord),
    TCOUNT(usize),
    PRGC(ProgressCurrentRecord),
    PRGT(ProgressTotalRecord),
    PRGV(ProgressValueRecord),
}

#[derive(Debug)]
pub enum MakeMkvError {
    DataError(String),
}

pub async fn run_makemkvcon(makemkvcon: PathBuf, args: Vec::<String>, tx: Sender<MakeMkvEvent>) -> Result<DiscContent, MakeMkvError> {
    let (tx_mkv, mut rx_mkv) = mpsc::channel::<String>(256);

    // see 'docs/makemkv-output.md' for examples of makemkvcon's output
    let th_mkv = spawn(run_program(makemkvcon, args, tx_mkv));

    // video, audio and subtitle streams needs special attention:
    // the attributes are provided as a sequential stream but we need
    // random access to be able to identify the stream's type and return
    // the correct type -> create an intermediate stream attribute cache
    let mut sac = HashMap::<usize, HashMap<usize, HashMap<u32, String>>>::new();

    let mut dc = DiscContent::new();
    let mut title_count = 0;
    let mut prgv_last = ProgressValueRecord{ current: 0, total: 0, maximum: 0};
    let mut new_stage = false;
    while !th_mkv.is_finished() {
        if let Some(line) = rx_mkv.recv().await {
            debug!("run_makemkvcon(): {}", line);
            let parsed = parse_output_line(line.as_bytes());
            match &parsed {
                // extract messages
                ParsedOutputLine::MSG(msg) => {
                    let event = MakeMkvEvent::MSG(msg.to_owned());
                    tx.send(event).await.unwrap();
                }
                // extract drive-related data
                ParsedOutputLine::DRV(drv ) => {
                    debug!("Parsing drive '{}'.", line);
                    // skip DRV records relating to non-existing drives
                    if drv.drive_status != Some(DrvStatus::NoDrive) {
                        let event = MakeMkvEvent::DRV(drv.to_owned());
                        debug!("Parsed drive '{}'.", line);
                        tx.send(event).await.unwrap();
                        debug!("Sent the DRV event.");
                    }
                },
                // extract content-related data
                ParsedOutputLine::TCOUNT(value) => {
                    // update title count variable with actual value
                    title_count = *value;
                    // and report its value to the caller
                    let event = MakeMkvEvent::TCOUNT(*value);
                    tx.send(event).await.unwrap();
                },
                ParsedOutputLine::CINFO(ir) => {
                    match dc.update_disc_attribute(&ir) {
                        Ok(_) => {},
                        Err(UpdateError::InternalError) => {
                            panic!(
                                "Failed to process CINFO record: Internal error! (id: {}, line: {})",
                                ir.attr_num, line
                            );
                        }
                        Err(UpdateError::UnknownStreamType) => {
                            // do nothing, limited to SINFO
                        }
                        Err(UpdateError::UnknownValue) => {
                            todo!(
                                "Failed to process CINFO record: Unknown attribute! (id: {}, line: {})",
                                ir.attr_val.as_ref().unwrap(),
                                line
                            );
                        }
                    }
                },
                ParsedOutputLine::TINFO((tid, ir))  => {
                    match dc.update_title_attribute(*tid, &ir) {
                        Ok(_) => {},
                        Err(UpdateError::InternalError) => {
                            panic!(
                                "Failed to process TINFO record: Internal error! (id: {}, line: {})",
                                ir.attr_num, line
                            );
                        }
                        Err(UpdateError::UnknownStreamType) => {
                            // do nothing, limited to SINFO
                        }
                        Err(UpdateError::UnknownValue) => {
                            todo!(
                                "Failed to process TINFO record: Unknown attribute! (id: {}, line: {})",
                                ir.attr_val.as_ref().unwrap(),
                                line
                            );
                        }
                    }
                },
                ParsedOutputLine::SINFO((tid, sid, ir)) => {
                    // store this attribute in the stream attribute cache
                    let tir = sac.entry(*tid).or_default();
                    let sar = tir.entry(*sid).or_default();
                    sar.insert(ir.attr_num, ir.value.clone());
                },
                // extract progress-related data
                ParsedOutputLine::PRGT((code, id, name)) => {
                    // ensure a 100% indication is printed for the previous stage
                    if prgv_last.current < prgv_last.maximum {
                        prgv_last.current = prgv_last.maximum;
                        let event = MakeMkvEvent::PRGV(prgv_last.clone());
                        tx.send(event).await.unwrap();
                    }

                    // process PRGT event
                    let prgt = ProgressTotalRecord{code: *code, id: *id, name: name.to_owned()};
                    let event = MakeMkvEvent::PRGT(prgt);
                    tx.send(event).await.unwrap();
                },
                ParsedOutputLine::PRGC((code, id, name)) => {
                    // ensure a 100% indication is printed for the previous stage
                    if prgv_last.current < prgv_last.maximum {
                        prgv_last.current = prgv_last.maximum;
                        let event = MakeMkvEvent::PRGV(prgv_last.clone());
                        tx.send(event).await.unwrap();
                    }

                    // process PRGC event
                    let prgc = ProgressCurrentRecord{code: *code, id: *id, name: name.to_owned()};
                    let event = MakeMkvEvent::PRGC(prgc);
                    tx.send(event).await.unwrap();

                    // remember stage change (for PRGV)
                    new_stage = true;
                },
                ParsedOutputLine::PRGV((current, total, maximum)) => {
                    // ensure a 0% indication is printed for each stage
                    if new_stage {
                        if *current > 0 {
                            let prgv_zero = ProgressValueRecord{ current: *current, total: *total, maximum: *maximum};
                            let event_zero = MakeMkvEvent::PRGV(prgv_zero);
                            tx.send(event_zero).await.unwrap();
                        }
                        new_stage = false;
                    }
                    // process PRGV event
                    let prgv = ProgressValueRecord{ current: *current, total: *total, maximum: *maximum};
                    let event = MakeMkvEvent::PRGV(prgv.clone());
                    tx.send(event).await.unwrap();
                    // remember current progress value
                    prgv_last = prgv;
                },
            }
        }
    }

    // ensure a 100% indication is printed for the last stage
    if prgv_last.current < prgv_last.maximum {
        prgv_last.current = prgv_last.maximum;
        let event = MakeMkvEvent::PRGV(prgv_last.clone());
        tx.send(event).await.unwrap();
    }

    debug!("run_makemkvcon(): run_program() has returned.");

    // process the stream attribute cache and convert to the correct record type
    for (tid, title) in sac.iter() {
        for (sid, stream) in title.iter() {
            debug!("Processing title {} / stream {}.", tid, sid);
            match parse_stream_attributes(stream) {
                Some(sr) => {
                    if dc.insert_stream_record(*tid, *sid, &sr).is_err() {
                        return Err(MakeMkvError::DataError("makemkv: data update error".to_string()));
                    };
                }
                None => {
                    // this is really bad - it makes no sense to continue
                    panic!("Detected an unknown stream type: {:#?}", stream);
                }
            }
        }
    }

    debug!("run_makemkvcon(): Stream attributes have been processed.");

    if dc.titles.len() != title_count {
        let msg = "Found content but didn't see TCOUNT in output!".to_string();
        return Err(MakeMkvError::DataError(msg));
    }

    // close the communication channel to signal the receiver we're done
    drop(tx);

    debug!("run_makemkvcon(): Finished 'run_program()'. Returning.");

    // return the disc's content
    // - if there is content it's now hierarchically structured and offers
    //   random access to any record
    // - potentially an empty husk with all values remaining at their
    //   initial state; it is the caller's responsibility to interpret the
    //   result and act accordingly)
    Ok(dc)
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

async fn run_program(program: PathBuf, args: Vec<String>, tx: Sender<String>) {
    info!("Running command '{} {}'.", program.to_string_lossy(), args.join(" "));
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        // .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn process");

    let stdout = child.stdout.take().expect("Failed to capture StdOut!");
    let mut reader_out = BufReader::new(stdout).lines();

    // let stderr = child.stdout.take().expect("Failed to capture StdErr!");
    // let mut reader_err = BufReader::new(stderr).lines();

    let th = tokio::spawn(async move {
        child.wait().await.expect("Child process encountered an error.");
    });
    
    while !th.is_finished() {
        // TODO capture StdErr
        // TODO read output in batches
        while let Ok(Some(line)) = reader_out.next_line().await {
            debug!("[makemkv] {}", line);
            tx.send(line).await.unwrap();
        }
        // this code will never run due to previous loop!
        // while let Ok(Some(line)) = reader_err.next_line().await {
        //     tx.send(line).await.unwrap();
        // }
    }

    // cleanup, ensure the child process is reaped
    // (makemkvcon always returns the same exit code -> ignore it)
    th.await.unwrap();

    // close the communication channel to signal the receiver we're done
    drop(tx);

    debug!("run_program(): Finished call to 'makemkvcon'. Returning.");
}
