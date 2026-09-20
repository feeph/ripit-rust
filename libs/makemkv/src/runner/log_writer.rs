/*
    write line to text file

    (does nothing if filename is none)
*/

// standard library imports
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

// third-party imports
use chrono::Local;
#[allow(unused_imports)]
use log::{debug, error, info, warn};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub struct LogWriter {
    fh: Option<File>,
}

impl LogWriter {
    pub async fn new(filename: Option<PathBuf>) -> Self {
        let fh = match filename {
            Some(logfile) => {
                debug!(
                    "Writing MakeMkv's output to file '{}'.",
                    logfile.to_string_lossy()
                );
                // ensure the parent directory exists
                match logfile.parent() {
                    Some(parent_dir) => {
                        if !parent_dir.is_dir() {
                            debug!("Creating parent dir '{}'.", parent_dir.to_string_lossy());
                            std::fs::create_dir_all(parent_dir)
                                .expect("Unable to create parent dir!");
                        } else {
                            debug!(
                                "Required parent dir '{}' already exists. Good.",
                                parent_dir.to_string_lossy()
                            );
                        }
                    }
                    None => {
                        debug!("No need to create a parent directory.");
                    }
                }
                // open the file for writing
                // (replace an existing file)
                Some(
                    OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(logfile)
                        .expect("Failed to open provided file!"),
                )
            }
            None => None,
        };
        Self { fh }
    }

    pub async fn write(&mut self, line: &str) {
        if let Some(fh) = &mut self.fh {
            let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S] ").to_string();
            fh.write_all(timestamp.as_bytes())
                .expect("Failed to write timestamp!");
            fh.write_all(line.as_bytes())
                .expect("Failed to write log line!");
            fh.write_all(b"\n").expect("Failed to write log newline!");
            fh.flush().expect("Failed to flush log file!");
        }
    }
}
