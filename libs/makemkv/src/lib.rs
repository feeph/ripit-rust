/*
    interface library for MakeMKV's console application (makemkvcon)
    https://www.makemkv.com/

    It is strongly recommended to set 'noscan' to 'true' if the caller
    doesn't care what kinds of discs are inserted in the optical drives.
    This significantly speeds up the command call by removing unneeded disc
    access and making competing I/O calls to already busy drives.
     
    The recommended pattern for reading from physical media is:

      - call `makemkvcon info` to identify drives and media type (DVD,
        Blu-Ray)
      - call `makemkvcon --noscan backup` with the correct destination
        (ISO file  for DVDs, directory for Blu-Rays)

    On all other calls (e.g. `makemkvcon mkv`) the parameter `--noscan`
    should be used.
*/

// TODO consider adding special logic for "MSG:3309"+"MSG:3041"
/*
The Grand Budapest Hotel (2014) BluRay:
  MSG:3309 Title 00851.mpls is equal to title 00801.mpls and was skipped
  MSG:3041 Failed to add angle #7 for title #851
*/

#[macro_use]
extern crate enum_primitive;

mod apdefs_h;
mod error_types;
mod parser;
mod runner;
mod scan_mode;

// standard library imports
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

// third-party imports
use async_trait::async_trait;
#[allow(unused_imports)]
use log::{debug, error, info, warn};
use tokio::sync::mpsc::Sender;

// crate-provided imports
use crate::api::{ProgressCurrentRecord, ProgressTotalRecord, ProgressValueRecord};
use crate::runner::{DiscContent, UpdateError, run_makemkvcon, parse_stream_attributes};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

pub mod api;

pub use api::{ContentType, DrvRecord, DrvStatus, InfoRecord, MsgRecord};
pub use error_types::{BackupError, InfoError, MkvError, LicenseError, FirmwareError};
pub use runner::{DirectIO, MakeMkvError, MakeMkvEvent, OutputType, Source, StreamRecord, parse_source};
pub use scan_mode::ScanMode;

/// makemkvcon's CLI interface as documented by `makemkvcon --help`
#[async_trait]
pub trait MakeMkvCli {

    /// backs up disc to a hard drive
    ///
    /// calls `makemkvcon [--noscan] backup <source> <destination folder>`
    async fn backup(&self, source: String, target: PathBuf, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>, logfile: Option<PathBuf>) -> Result<(), BackupError>;

    /// prints info about disc
    /// 
    /// It is strongly recommended to set 'scan_drives' to 'false' if the
    /// caller doesn't care what kinds of discs are inserted in the optical
    /// drives. This significantly speeds up the command call by preventing
    /// competing I/O calls to already busy drives.
    /// 
    /// The recommended pattern is:
    /// 
    /// - call `info` to identify drives and media type (DVD, Blu-Ray)
    /// - call `backup` with the correct destination (iso-file / directory)
    ///
    /// calls `makemkvcon [--noscan] info <source>`
    async fn info(&self, source: String, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>) -> Result<(), InfoError>;

    /// saves one or more titles to mkv files
    ///
    /// calls `makemkvcon mkv <source> <title id> <destination folder>`
    /// - `<title id>` can be 
    ///   - a single title (e.g. '1')
    ///   - a range of titles (e.g.: '1-3')
    ///   - the keyword 'all'
    ///
    /// calls `makemkvcon [--noscan] mkv <source> <title> <target>`
    async fn mkv(&self, source: String, titles: String, target: PathBuf, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>, logfile: Option<PathBuf>) -> Result<(), MkvError>;

    /// run universal firmware tool
    /// 
    /// calls `makemkvcon f <args>`
    async fn f(&self, drive: String, filename: std::path::PathBuf, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>) -> Result<(), FirmwareError>;
    
    /// enter registration key into program
    /// (beta key is published at https://forum.makemkv.com/forum/viewtopic.php?f=5&t=1053)
    ///
    /// calls `makemkvcon [--noscan] reg <key string or file name>`
    async fn reg(&self, license_key: String, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>) -> Result<(), LicenseError>;
}

// ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Debug,
    Noise,
    Unknown,
}

// similar to MsgRecord but augmented with a severity level
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub code: u32,
    pub flags: u32,
    pub count: u32,
    pub message: String,
    pub format: String,
    pub params: Vec<String>,
    pub severity: Severity,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stage {
    Idle,
    DriveScan,
    BackupDisc,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BackupEvent {
    Message(Message),
    ProgressPercentage((u8, Stage, u32, f32)),
}

#[derive(Clone, Debug)]
pub struct MakeMkvConfig {
    // generic settings common to all modes
    pub enable_robot: bool,     // --robot
    pub debug: bool,            // --debug
    pub directio: DirectIO,     // --directio=...
    pub messages: OutputType,   // --messages=...
    pub progress: OutputType,   // --progress=...
    pub min_length: usize,      // --minlength=...
}

#[derive(Clone, Debug)]
pub struct MakeMkv {
    pub binary: PathBuf,
    pub config: MakeMkvConfig,
}

impl MakeMkv {

    pub fn defaults(binary: PathBuf) -> Self {
        MakeMkv {
            binary,
            config: MakeMkvConfig {
                enable_robot: false,
                debug: false,
                directio: DirectIO::Enabled,
                messages: OutputType::Silent,
                progress: OutputType::Silent, 
                min_length: 120,
            }
        }
    }

    // --------------------------------------------------------------------
    // generic command runner
    // --------------------------------------------------------------------

    pub async fn run(&self, mut args: Vec<String>, tx: Sender<MakeMkvEvent>, logfile: Option<PathBuf>) -> Result<DiscContent, MakeMkvError> {
        let args_mkv = create_args(&self.config, &mut args);
        run_makemkvcon(self.binary.clone(), args_mkv, tx, logfile).await
    }

}

#[async_trait]
impl MakeMkvCli for MakeMkv {

    async fn backup(&self, source: String, target: PathBuf, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>, logfile: Option<PathBuf>) -> Result<(), BackupError> {

        // e.g. `makemkvcon64.exe backup drv:0 filename.iso`
        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("backup".to_string());
        args.push(source);
        args.push(target.to_string_lossy().to_string());

        match self.run(args, tx, logfile).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(BackupError::DataError),
        }
    }

    async fn info(&self, source: String, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>) -> Result<(), InfoError> {

        // e.g. `makemkvcon64.exe [--noscan] info drv:0`
        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("info".to_string());
        args.push(source);

        match self.run(args, tx, None).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(InfoError::DataError),
        }
    }

    async fn mkv(&self, source: String, titles: String, target: PathBuf, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>, logfile: Option<PathBuf>) -> Result<(), MkvError> {

        // e.g. `makemkvcon64.exe --noscan mkv drv:0 1 title_1.mkv`
        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("mkv".to_string());
        args.push(source);
        args.push(titles);
        args.push(target.to_string_lossy().to_string());

        match self.run(args, tx, logfile).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(MkvError::DataError),
        }
    }

    async fn f(&self, drive: String, filename: std::path::PathBuf, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>) -> Result<(), FirmwareError> {

        // e.g. `makemkvcon64.exe f --all-yes -d E: rawflash -i Downgrade-Enabled-Firmware\Auto-Flash\LG-Desktop-NS60-sleep-fix\WH16NS60-1.02-MK.bin`
        // TODO add support for flashing encrypted firmware (enc)
        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("f".to_string());
        args.push("--all-yes".to_string());
        args.push("-d".to_string());
        args.push(drive);
        args.push("rawflash".to_string());
        args.push("-i".to_string());
        args.push(filename.to_string_lossy().to_string());

        match self.run(args, tx, None).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(FirmwareError::DataError),
        }
    }
    
    async fn reg(&self, license_key: String, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>) -> Result<(), LicenseError> {

        // e.g. `makemkvcon64.exe reg T-Wa...9e`
        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("--robot".to_string());
        args.push("reg".to_string());
        args.push(license_key);

        match self.run(args, tx, None).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(LicenseError::DataError),
        }
    }

}

#[derive(Clone, Debug)]
pub struct MakeMkvMock {
    log_file: PathBuf,
}

impl MakeMkvMock {

    // --------------------------------------------------------------------
    // generic command runner
    // --------------------------------------------------------------------

    // duplicates 'libs/makemkv/src/runner/mod.rs::run_makemkvcon(<...>)'
    // TODO try to remove the duplication
    pub async fn run(&self, mut _args: Vec<String>, tx: Sender<MakeMkvEvent>, _logfile: Option<PathBuf>) -> Result<DiscContent, MakeMkvError> {
        let log = read_to_string(&self.log_file).expect("Unable to open log file.");

        // video, audio and subtitle streams needs special attention:
        // the attributes are provided as a sequential stream but we need
        // random access to be able to identify the stream's type and return
        // the correct type -> create an intermediate stream attribute cache
        let mut sac = HashMap::<usize, HashMap<usize, HashMap<u32, String>>>::new();

        let mut dc = DiscContent::new();
        let mut title_count = 0;
        let mut prgv_last = crate::api::ProgressValueRecord{ current: 0, total: 0, maximum: 0};
        let mut new_stage = false;
        for line in log.lines() {
            let parsed = crate::parser::parse_output_line(line.as_bytes());
            match &parsed {
                // extract messages
                crate::parser::ParsedOutputLine::MSG(msg) => {
                    let event = MakeMkvEvent::MSG(msg.to_owned());
                    tx.send(event).await.unwrap();
                }
                // extract drive-related data
                crate::parser::ParsedOutputLine::DRV(drv ) => {
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
                crate::parser::ParsedOutputLine::TCOUNT(value) => {
                    // update title count variable with actual value
                    title_count = *value;
                    // and report its value to the caller
                    let event = MakeMkvEvent::TCOUNT(*value);
                    tx.send(event).await.unwrap();
                },
                crate::parser::ParsedOutputLine::CINFO(ir) => {
                    match dc.update_disc_attribute(ir) {
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
                crate::parser::ParsedOutputLine::TINFO((tid, ir))  => {
                    match dc.update_title_attribute(*tid, ir) {
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
                crate::parser::ParsedOutputLine::SINFO((tid, sid, ir)) => {
                    // store this attribute in the stream attribute cache
                    let tir = sac.entry(*tid).or_default();
                    let sar = tir.entry(*sid).or_default();
                    sar.insert(ir.attr_num, ir.value.clone());
                },
                // extract progress-related data
                crate::parser::ParsedOutputLine::PRGT((code, id, name)) => {
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
                crate::parser::ParsedOutputLine::PRGC((code, id, name)) => {
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
                crate::parser::ParsedOutputLine::PRGV((current, total, maximum)) => {
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

        // ensure a 100% indication is printed for the last stage
        if prgv_last.current < prgv_last.maximum {
            prgv_last.current = prgv_last.maximum;
            let event = MakeMkvEvent::PRGV(prgv_last.clone());
            tx.send(event).await.unwrap();
        }

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

        if dc.titles.len() != title_count {
            let msg = "Found content but didn't see TCOUNT in output!".to_string();
            return Err(MakeMkvError::DataError(msg));
        }

        // close the communication channel to signal the receiver we're done
        drop(tx);

        Ok(dc)
    }

}

#[async_trait]
impl MakeMkvCli for MakeMkvMock {

    async fn backup(&self, source: String, target: PathBuf, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>, logfile: Option<PathBuf>) -> Result<(), BackupError> {

        // e.g. `makemkvcon64.exe backup drv:0 filename.iso`
        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("backup".to_string());
        args.push(source);
        args.push(target.to_string_lossy().to_string());

        match self.run(args, tx, logfile).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(BackupError::DataError),
        }
    }

    async fn info(&self, source: String, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>) -> Result<(), InfoError> {

        // e.g. `makemkvcon64.exe [--noscan] info drv:0`
        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("info".to_string());
        args.push(source);

        match self.run(args, tx, None).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(InfoError::DataError),
        }
    }

    async fn mkv(&self, source: String, titles: String, target: PathBuf, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>, logfile: Option<PathBuf>) -> Result<(), MkvError> {

        // e.g. `makemkvcon64.exe --noscan mkv drv:0 1 title_1.mkv`
        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("mkv".to_string());
        args.push(source);
        args.push(titles);
        args.push(target.to_string_lossy().to_string());

        match self.run(args, tx, logfile).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(MkvError::DataError),
        }
    }

    async fn f(&self, drive: String, filename: std::path::PathBuf, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>) -> Result<(), FirmwareError> {

        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("f".to_string());
        args.push("--all-yes".to_string());
        args.push("-d".to_string());
        args.push(drive);
        args.push("rawflash".to_string());
        args.push("-i".to_string());
        args.push(filename.to_string_lossy().to_string());

        match self.run(args, tx, None).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(FirmwareError::DataError),
        }
    }
    
    async fn reg(&self, license_key: String, scan_mode: ScanMode, tx: Sender<MakeMkvEvent>) -> Result<(), LicenseError> {

        // e.g. `makemkvcon64.exe reg T-Wa...9e`
        let mut args = Vec::new();
        match scan_mode {
            ScanMode::DriveOnly => {
                args.push("--noscan".to_string());
            },
            ScanMode::DriveAndDisc => {
                // default mode, do nothing
            }
        }
        args.push("--robot".to_string());
        args.push("reg".to_string());
        args.push(license_key);

        match self.run(args, tx, None).await {
            Ok(_) => Ok(()),
            Err(MakeMkvError::DataError(_)) => Err(LicenseError::DataError),
        }
    }
}

// ------------------------------------------------------------------------

fn create_args(cfg: &MakeMkvConfig, args: &mut Vec<String>) -> Vec::<String> {
    let mut args_mkv = Vec::new();

    // configure machine-readable output (CSV-like)
    //
    // This setting should be considered mandatory since without it
    // message codes unavailable and there's no way to identify a
    // message's severity without parsing the provided string.
    if cfg.enable_robot {
        args_mkv.push("--robot".to_string());
    }

    // configure visibility of status messages
    match &cfg.messages {
        OutputType::StdOut => {
            // nothing to do, it's the default
            // equivalent to '--messages=-stdout'
        },
        OutputType::StdErr => {
            args_mkv.push("--messages=-stderr".to_string());
        },
        OutputType::Silent => {
            args_mkv.push("--messages=-none".to_string());
        },
        OutputType::File(filename) => {
            args_mkv.push(format!("--messages={}", filename.to_string_lossy()));
        },
    };

    // configure visibility of progress messages
    match &cfg.progress {
        OutputType::StdOut => {
            args_mkv.push("--progress=-stdout".to_string());
        },
        OutputType::StdErr => {
            args_mkv.push("--progress=-stderr".to_string());
        },
        OutputType::Silent => {
            // nothing to do, it's the default
            // equivalent to '--progress=-none'
        },
        OutputType::File(filename) => {
            args_mkv.push(format!("--progress={}", filename.to_string_lossy()));
        },
    }

    // configure visibility of debug messages
    //
    // !! makemkvcon's implementation is bugged and always uses
    // !! ./MakeMKV_log.txt, ignoring the user-provided filename
    // !! -> Using "--debug[=FILE]" does NOT work.
    if cfg.debug {
        args_mkv.push("--debug".to_string());
    }

    // configure scanning the disc's media type (DVD, HD-DVD, ...)
    // if cfg.disable_scan {
    //     args_mkv.push("--noscan".to_string());
    // }

    // configure direct disc access
    match cfg.directio {
        DirectIO::Enabled => {
            // nothing to do, it's the default
            // equivalent to '--directio=true'
        },
        DirectIO::Disabled => {
            args_mkv.push("--directio=false".to_string());
        },
    }

    // configure minimum title length
    // (default value: 120 seconds)
    args_mkv.push(format!("--minlength={}", cfg.min_length));

    // ...and finally append the provided arguments
    args_mkv.append(args);

    args_mkv
}
