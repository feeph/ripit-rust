/*
    abstraction layer for executing the 'makemkvcon' command and parsing
    its output while its running

    This code in this file is intended for exclusive use within this
    library and may change at any point. Consumers of this library are
    expected to use the public functions 'makemkv::backup()',
    'makemkv::info()' and 'makemkv::mkv()'.
*/

mod disc_content;
mod log_writer;
mod source;
mod streams;

// standard library imports
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::spawn;
use tokio::sync::mpsc::{self, Sender};
use tokio::task::JoinHandle;

// crate-provided imports
use crate::apdefs_h::DrvStatus;
use crate::api::{
    DrvRecord, MsgRecord, ProgressCurrentRecord, ProgressTotalRecord, ProgressValueRecord,
};
use crate::parser::{ParsedOutputLine, parse_output_line};
use crate::runner::log_writer::LogWriter;

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
    File(PathBuf),
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
    CINFO(String),
    TINFO(String),
    SINFO(String),
    PRGC(ProgressCurrentRecord),
    PRGT(ProgressTotalRecord),
    PRGV(ProgressValueRecord),
}

#[derive(Debug)]
pub enum MakeMkvError {
    DataError(String),
}

pub async fn run_makemkvcon(
    source: &str,
    makemkvcon: PathBuf,
    args: Vec<String>,
    tx: Sender<MakeMkvEvent>,
    logfile: Option<PathBuf>,
) -> Result<DiscContent, MakeMkvError> {
    let (tx_mkv, mut rx_mkv) = mpsc::channel::<String>(256);

    // see 'docs/makemkv-output.md' for examples of makemkvcon's output
    let th_mkv = spawn(run_program(makemkvcon, args, tx_mkv));

    let mut lw: LogWriter = LogWriter::new(logfile).await;

    let mut lines = Vec::new();
    let batch_size = 128;

    // parse incoming events until the spawned task finishes
    let mut pc = ParserContext::new(source);
    while !th_mkv.is_finished() {
        rx_mkv.recv_many(&mut lines, batch_size).await;
        for line in lines.drain(..) {
            debug!("run_makemkvcon(): {}", line);

            // preserve MakeMkv's original output
            lw.write(&line).await;

            // process MakeMkv's output and generate events
            pc.process_output(&line, &tx).await;
        }
    }
    // makemkvcon does not return any particularly interesting result
    // TODO consider checking for Err()
    let _result = th_mkv.await;

    // makemkvcon has finished - ensure all remaining events are processed
    let remaining = rx_mkv.len();
    if remaining > 0 {
        rx_mkv.recv_many(&mut lines, batch_size).await;
        for line in lines {
            pc.process_output(&line, &tx).await;
        }
    }

    // ensure a 100% indication is printed for the last stage
    let mut prgv_last = pc.get_prgv();
    if prgv_last.current < prgv_last.maximum {
        prgv_last.current = prgv_last.maximum;
        let event = MakeMkvEvent::PRGV(prgv_last.clone());
        tx.send(event).await.unwrap();
    }

    debug!("run_makemkvcon(): run_program() has returned.");

    // process the stream attribute cache and convert to the correct record type
    let mut dc = pc.get_dc();
    let sac = pc.get_sac();
    for (tid, title) in sac.iter() {
        for (sid, stream) in title.iter() {
            debug!("Processing title {} / stream {}.", tid, sid);
            match parse_stream_attributes(stream) {
                Some(sr) => {
                    if dc.insert_stream_record(*tid, *sid, &sr).is_err() {
                        return Err(MakeMkvError::DataError(
                            "makemkv: data update error".to_string(),
                        ));
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

    let title_count = pc.get_title_count();
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
    info!(
        "[makemkv] run_program(): Running command '{} {}'.",
        program.to_string_lossy(),
        args.join(" ")
    );

    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn process!");

    let th_stdout = create_output_reader(child.stdout.take(), "StdOut", tx.clone());
    let th_stderr = create_output_reader(child.stderr.take(), "StdErr", tx.clone());

    // wait for the child process to complete, then let the reader tasks
    // finish draining the pipes
    // (exitcode is always 0, even if makemkvcon fails)
    let _status = child
        .wait()
        .await
        .expect("Child process encountered an error!");

    let _ = tokio::join!(th_stdout, th_stderr);
    drop(tx);

    debug!("[makemkv] run_program(): Finished call to 'makemkvcon'. Returning.");
}

/// create a reader for StdOut or StdErr streams
fn create_output_reader<R>(
    output_stream: Option<R>,
    label: &'static str,
    tx: Sender<String>,
) -> JoinHandle<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        if let Some(stream) = output_stream {
            debug!(
                "[makemkv] create_output_reader(): Command opened '{}'.",
                label
            );
            let mut reader = BufReader::new(stream).lines();

            while let Ok(Some(line)) = reader.next_line().await {
                debug!("[makemkv][{}] {}", label.to_lowercase(), line);
                tx.send(line).await.expect("Receiver dropped!");
            }
        } else {
            debug!(
                "[makemkv] create_output_reader(): Command didn't open '{}'.",
                label
            );
        }
    })
}

type SacType = HashMap<usize, HashMap<usize, HashMap<u32, String>>>;

struct ParserContext {
    // video, audio and subtitle streams needs special attention:
    // the attributes are provided as a sequential stream but we need
    // random access to be able to identify the stream's type and return
    // the correct type -> create an intermediate stream attribute cache
    sac: SacType,
    dc: DiscContent,
    source: String,
    title_count: usize,
    prgt_last: u32,
    prgc_last: u32,
    prg_max: u32,
}

impl ParserContext {
    fn new(source: &str) -> Self {
        ParserContext {
            sac: SacType::new(),
            dc: DiscContent::new(),
            source: source.to_string(),
            title_count: 0,
            prgt_last: 0,
            prgc_last: 0,
            prg_max: 0,
        }
    }

    async fn process_output(&mut self, line: &str, tx: &Sender<MakeMkvEvent>) {
        let parsed = parse_output_line(line.as_bytes());
        match parsed {
            // extract messages
            ParsedOutputLine::MSG(msg) => {
                let event = MakeMkvEvent::MSG(msg.to_owned());
                tx.send(event).await.unwrap();
            }
            // extract drive-related data
            ParsedOutputLine::DRV(drv) => {
                debug!("Parsing drive '{}'.", line);
                // skip DRV records relating to non-existing drives
                if drv.drive_status != Some(DrvStatus::NoDrive) {
                    let event = MakeMkvEvent::DRV(drv.to_owned());
                    debug!("Parsed drive '{}'.", line);
                    tx.send(event).await.unwrap();
                    debug!("Sent the DRV event.");
                }
            }
            // extract content-related data
            ParsedOutputLine::TCOUNT(value) => {
                // update title count variable with actual value
                self.title_count = value;
                // and report its value to the caller
                let event = MakeMkvEvent::TCOUNT(value);
                tx.send(event).await.unwrap();
            }
            ParsedOutputLine::CINFO(ir) => {
                match self.dc.update_disc_attribute(&ir) {
                    Ok(_) => {}
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
            }
            ParsedOutputLine::TINFO((tid, ir)) => {
                match self.dc.update_title_attribute(tid, &ir) {
                    Ok(_) => {}
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
            }
            ParsedOutputLine::SINFO((tid, sid, ir)) => {
                // store this attribute in the stream attribute cache
                let tir = self.sac.entry(tid).or_default();
                let sar = tir.entry(sid).or_default();
                sar.insert(ir.attr_num, ir.value.clone());
            }
            // extract progress-related data
            ParsedOutputLine::PRGT((code, id, name)) => {
                // sample output
                // ----------------------------------------------------
                // PRGT:5018,0,"Scanning CD-ROM devices"
                // PRGT:5047,0,"Copying all files"
                // ----------------------------------------------------

                // ensure a 100% indication is printed for the previous
                // stage and task
                if self.prgc_last < self.prg_max {
                    let prgv = ProgressValueRecord::new(
                        &self.source,
                        self.prg_max,
                        self.prg_max,
                        self.prg_max,
                    );
                    let event = MakeMkvEvent::PRGV(prgv);
                    tx.send(event).await.unwrap();
                }

                // process PRGT event
                let prgt = ProgressTotalRecord::new(&self.source, code, id, name);
                let event = MakeMkvEvent::PRGT(prgt);
                tx.send(event).await.unwrap();

                // reset 'current' and 'total' progress
                self.prgc_last = 0;
                self.prgt_last = 0;
                // zero out prg_last to indicate the maximum is unknown
                self.prg_max = 0;
            }
            ParsedOutputLine::PRGC((code, id, name)) => {
                // The meaning of the second field is unclear. Usually
                // the value increases, but sometimes it skips ahead
                // and sometimes it decreases again.
                // ----------------------------------------------------
                // PRGC:5046,435,"Copying file"
                // PRGC:5046,466,"Copying file" <-- skips ahead
                // PRGC:5046,467,"Copying file"
                // PRGC:5046,203,"Copying file" <-- decreases
                // ----------------------------------------------------

                // ensure a 100% indication is printed for the previous task
                if self.prgc_last < self.prg_max {
                    let prgv = ProgressValueRecord::new(
                        &self.source,
                        self.prg_max,
                        self.prgt_last,
                        self.prg_max,
                    );
                    let event = MakeMkvEvent::PRGV(prgv);
                    tx.send(event).await.unwrap();
                }

                // process PRGC event
                let prgc = ProgressCurrentRecord::new(&self.source, code, id, name);
                let event = MakeMkvEvent::PRGC(prgc);
                tx.send(event).await.unwrap();

                // reset 'current' progress
                // (preserve 'total' and 'max' value)
                self.prgc_last = 0;
            }
            ParsedOutputLine::PRGV((current, total, maximum)) => {
                // ensure a 0% indication is printed for each stage
                if self.prgc_last == 0 && current > 0 {
                    let prgv = ProgressValueRecord::new(&self.source, 0, total, maximum);
                    let event = MakeMkvEvent::PRGV(prgv);
                    tx.send(event).await.unwrap();
                }

                // process PRGV event
                let prgv = ProgressValueRecord::new(&self.source, current, total, maximum);
                let event = MakeMkvEvent::PRGV(prgv.clone());
                tx.send(event).await.unwrap();

                // remember updated progress values
                self.prgc_last = current;
                self.prgt_last = total;
                self.prg_max = maximum;
            }
        }
    }

    fn get_dc(&self) -> DiscContent {
        return self.dc.clone();
    }

    fn get_prgv(&self) -> ProgressValueRecord {
        ProgressValueRecord::new(&self.source, self.prgc_last, self.prgt_last, self.prg_max)
    }

    fn get_sac(&self) -> SacType {
        return self.sac.clone();
    }

    fn get_title_count(&self) -> usize {
        return self.title_count;
    }
}
