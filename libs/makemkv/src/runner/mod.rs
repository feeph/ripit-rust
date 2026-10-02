/*!
    abstraction layer for executing the 'makemkvcon' command and parsing
    its output while its running

    This code in this file is intended for exclusive use within this
    library and may change at any point. Consumers of this library are
    expected to use the public functions 'makemkv::backup()',
    'makemkv::info()' and 'makemkv::mkv()'.
*/

mod disc_content;
mod log_writer;
mod parser_context;
mod source;
mod streams;

// standard library imports
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
use crate::runner::log_writer::LogWriter;
use crate::runner::parser_context::ParserContext;

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub use disc_content::{DiscContent, UpdateError};
pub use parser_context::MakeMkvEvent;
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
    let mut pc = ParserContext::new(source, tx);
    while !th_mkv.is_finished() {
        rx_mkv.recv_many(&mut lines, batch_size).await;
        for line in lines.drain(..) {
            debug!("run_makemkvcon(): {}", line);

            // preserve MakeMkv's original output
            lw.write(&line).await;

            // process MakeMkv's output and generate events
            pc.process_output(&line).await;
        }
    }

    // makemkvcon does not return any particularly interesting result
    // TODO consider checking for Err()
    let _result = th_mkv.await;

    // makemkvcon has finished - ensure all remaining events are processed
    let remaining = rx_mkv.len();
    if remaining > 0 {
        rx_mkv.recv_many(&mut lines, remaining).await;
        for line in lines.drain(..) {
            pc.process_output(&line).await;
        }
    }

    // ensure the last PRGT/PRGC stops at 100%
    //
    // makemkvcon may stop without reaching 100%:
    // --------------------------------------------------------------------
    // PRGC:3104,0,"Decrypting data"
    // PRGV:0,36086,65536
    // <…>
    // PRGV:65536,43137,65536
    // PRGV:65536,43967,65536                 <-- PRGT stops at (43967) 67%
    // MSG:5011,0,0,"Operation successfully completed","Operation successfully completed"
    // --------------------------------------------------------------------

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
