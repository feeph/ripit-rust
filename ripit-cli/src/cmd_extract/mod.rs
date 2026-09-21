/*!
    ripit-cli extract [OPTIONS]

    (use `ripit-cli extract --help` for full usage instructions)

    usage:

    ```TEXT
    # read from an optical drive and write MKV files to target directory
    # (creates a subdirectory matching the disc's name)
    ripit-cli extract -d E:       -t C:\mkv
    ripit-cli extract -d /dev/sr0 -t /mnt/mkv

    # read from multiple optical drives in parallel
    ripit-cli extract -d D:       -d E:       -t C:\mkv
    ripit-cli extract -d /dev/sr0 -d /dev/sr1 -t /mnt/mkv

    # read from all available optical drives
    # (skips drives without a disc)
    ripit-cli extract -t C:\mkv
    ripit-cli extract -t /mnt/mkv

    # read from one or more disc images
    ripit-cli extract -i feature.iso -i bonus.iso -t C:\mkv
    ripit-cli extract -i *.iso                    -t /mnt/mkv
    ```

    output example:

    ```TEXT
    $ ripit-cli extract -O -d /dev/sr1 /dev/sr2 -t /media/backup
    I: Found 3 drives: /dev/sr0 /dev/sr1 /dev/sr2
    I: Deleted existing directory "/media/backup/DVDVolume". (0 MKV files, 0 other).
    I: Using optical drive "/dev/sr2" (DVDVolume).
    I: Using optical drive "/dev/sr1" (OTAKU_NO_VIDEO).
    ⠴ [OTAKU_NO_VIDEO] Opening DVD disc (12%)          [██░░░░░░░░░░░░░░░░░░] 00:00:09
    ⠙ [OTAKU_NO_VIDEO] `--> Scanning contents (0%)     [░░░░░░░░░░░░░░░░░░░░] 00:00:00
    ⠙ [DVDVolume] Opening DVD disc (2%)                [░░░░░░░░░░░░░░░░░░░░] 00:00:08
    ⠸ [DVDVolume] `--> Scanning contents (1%)          [░░░░░░░░░░░░░░░░░░░░] 00:00:00
    ```

    A logfile with makemkvcon's output is added to the target directory.

*/

// standard library imports
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

// third-party imports
use clap::{Parser, ValueHint};
use dialoguer::Confirm;
use indicatif_log_bridge::LogWrapper;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::Serialize;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

// crate-provided imports
use crate::progress_tracker::{ProgressTracker, Stage};
use ripit::{
    ExtractEvent, OpticalDrive, ScanMode, extract_from_drive, extract_from_image, find_drives,
    find_matching_drives,
};

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum OutputFormat {
    #[default]
    Text,
    Yaml,
}

#[derive(Parser, Debug)]
pub struct CmdArgs {
    // Please note:
    // Trailing dots in ///-comments are stripped by clap.
    /// Select a drive, e.g. 'D:' or '/dev/sr0' (repeatable)
    /// (implies usage of all drives if none are selected)
    #[arg(short = 'd', long = "drive", num_args = 1..)]
    drives_want: Vec<PathBuf>,

    /// Select an image, e.g. 'image.iso' (repeatable)
    #[arg(short = 'i', long = "disc-image", num_args = 1..)]
    disc_image: Vec<PathBuf>,

    /// Target directory
    #[arg(short = 't', long = "target-dir", default_value = ".", value_hint = ValueHint::DirPath)]
    target: std::path::PathBuf,

    // /// A path-like source: directory, filename, device name, or drive letter
    // #[arg()]
    // source: std::path::PathBuf,

    // /// Write MKV files to a disc-specific directory at this location
    // #[arg(short = 'r', long = "target-root", default_value = ".", value_hint = ValueHint::DirPath)]
    // target: std::path::PathBuf,
    /// Extract one or more specific title(s) identified by their index [defaults to 'all']
    #[arg(short = 'T', long = "title", value_name = "TITLE")]
    titles: Vec<usize>,

    /// Ignore all titles shorter than x seconds (env: MIN_LENGTH)
    #[arg(
        short = 'L',
        long = "min-length",
        default_value = "0",
        env = "MIN_LENGTH"
    )]
    min_length: usize,

    /// Allow overwriting existing an already existing directory
    #[arg(short = 'O', long = "allow-overwrite", default_value = "false")]
    allow_overwrite: bool,

    /// Eject medium from optical drive after extracting
    #[arg(short = 'e', long = "eject-when-done")]
    eject_when_done: bool,

    /// Write output  to a file
    #[arg(short = 'o', long = "output-file", value_hint = ValueHint::FilePath)]
    output_file: Option<PathBuf>,

    /// Select output format
    #[arg(short = 'f', long = "output-format", default_value_t, value_enum)]
    output_format: OutputFormat,

    #[clap(flatten)]
    global_opts: crate::GlobalOpts,
    // #[arg(long = "log-level", help = "Set logging level (debug, info, warn, error). Default is 'warn'")]
    // log_level: Option<log::Level>,
}

enum SourceType {
    Image(PathBuf),
    Drive(OpticalDrive),
}

/// example output:
///
/// ```TEXT
/// <...>
/// ```
pub async fn run(args: CmdArgs, mm: &makemkv::MakeMkv) -> i32 {
    // need to use indicatif_log_bridge otherwise logged messages would
    // messes up indicatif's output
    // FIXME restore ability to use 'args.global_opts.log_level'
    let logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).build();
    let level = logger.filter();

    debug!("drive(s):        {:#?}", args.drives_want);
    debug!("disc_image(s):   {:#?}", args.disc_image);
    debug!("target:          {:#?}", args.target);
    debug!("allow_overwrite: {:#?}", args.allow_overwrite);
    debug!("eject_when_done: {:#?}", args.eject_when_done);
    debug!("min_length:      {:#?}", args.min_length);
    debug!("output_file:     {:#?}", args.output_file);
    debug!("output_format:   {:#?}", args.output_format);

    let mut pt = ProgressTracker::new();

    // augment MultiProgress with indicatif-log-bridge to avoid
    // messages emitted by log-crate breaking indicatif's output
    // <https://crates.io/crates/indicatif-log-bridge>
    LogWrapper::new(pt.get_mp(), logger).try_init().unwrap();
    log::set_max_level(level);

    // if no titles are specified use the keyword "all"
    let titles: Vec<String> = if !args.titles.is_empty() {
        args.titles
            .clone()
            .iter()
            .map(|title| title.to_string())
            .collect()
    } else {
        vec!["all".to_string()]
    };
    debug!("titles:          {}", titles.join(", "));

    // the required order of operations is:
    // 1. identify the source's type (image or drive)
    // 2. identify the disc's name
    // 2.1. image: derive from filename
    // 2.2. drive: access drive and read disc
    // 3. create actual target dir value
    // 4. delete existing target (if it exists)

    // using drives and images at the same time is not supported because
    // that allows us to skip the drive scanning if one or more image are
    // being used. This may reduce startup time by up to a minute.
    // (Depending on how many drives are present and if they are currently
    // busy.)
    let mut sources = Vec::new();
    if !args.disc_image.is_empty() {
        for filename in args.disc_image {
            if filename.is_file() || filename.is_dir() {
                // file or directory: assume image
                // -> use the filename (without extension) as disc's name
                let disc_name: String = filename
                    .file_stem()
                    .expect("not a filename?")
                    .to_string_lossy()
                    .to_string();
                sources.push((SourceType::Image(filename), disc_name));
            } else {
                let message = format!(
                    "Image {:#?} is neither a file or directory. Ignoring.",
                    filename
                );
                pt.send_text_message(&message);
            }
        }
    } else {
        let scan_mode = ScanMode::DriveAndDisc;
        let drives_have = find_drives(mm.to_owned(), scan_mode)
            .await
            .expect("Reading drives should never fail.");
        if drives_have.is_empty() {
            pt.send_text_message("E: Found no available drives!");
            return exitcode::IOERR;
        } else {
            // convert 'Vec<OpticalDrive>' into a stringified list of sorted
            // device names, e.g. '/dev/sr0 /dev/sr1'
            let mut device_names = drives_have
                .clone()
                .into_iter()
                .map(|drv| drv.device.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            device_names.sort();
            let device_names_str = device_names.join(" ");
            pt.send_text_message(&format!(
                "I: Found {} drives: {}",
                drives_have.len(),
                device_names_str
            ));
        }

        let drives_want = args.drives_want;

        let drives = find_matching_drives(&drives_have, &drives_want);
        debug!("drives_have: {:#?}", drives_have);
        debug!("drives_want: {:#?}", drives_want);
        debug!("filtered:    {:#?}", drives);

        for drive in drives {
            match &drive.disc {
                Some(disc) => {
                    let disc_name = disc.get_name().to_string();
                    sources.push((SourceType::Drive(drive), disc_name));
                }
                None => {
                    let message = format!(
                        "I: Unable to detect disc in drive {:#?}. Ignoring.",
                        drive.device
                    );
                    pt.send_text_message(&message);
                }
            };
        }
    };

    if sources.is_empty() {
        let message = "Must provide at least one drive or disc image!";
        pt.send_text_message(message);
        return exitcode::CONFIG;
    }

    let (tx, mut rx) = mpsc::channel::<ExtractEvent>(256);

    let mut workers = HashMap::new();
    for (source_type, disc_name) in sources {
        // take user-provided target value and append the disc's name
        let target_mkv = normalize_path(&args.target).join(&disc_name);

        let allow_overwrite = args.allow_overwrite;
        let eject_when_done = args.eject_when_done;

        // can do this only AFTER knowing whether source is a drive or image
        if target_mkv.exists() {
            if allow_overwrite {
                match check_directory_and_delete(&target_mkv) {
                    Ok(msg) => {
                        pt.send_text_message(&msg);
                    }
                    Err(err) => {
                        pt.send_text_message(&err);
                        return exitcode::OSFILE;
                    }
                }
            } else {
                pt.send_text_message(&format!("E: Directory {:#?} exists and '--allow-overwrite' is not used. Unable to proceed.", target_mkv));
                return exitcode::OSFILE;
            }
        }
        debug!("disc_name:       {}", disc_name);
        let mm_mkv = mm.clone();

        // create copies of relevant resources for 'async move{}'
        let mm_cpy = mm_mkv.clone();
        let titles_cpy = titles.clone();
        let target_cpy = target_mkv.clone();
        let tx_cpy = tx.clone();

        // spawn the source-specific extraction process
        let th_mkv = match source_type {
            SourceType::Drive(drive) => {
                pt.send_text_message(&format!(
                    "I: Using optical drive {:#?} ({}).",
                    drive.device, disc_name
                ));

                spawn(async move {
                    extract_from_drive(
                        mm_cpy,
                        drive,
                        target_cpy,
                        titles_cpy,
                        eject_when_done,
                        tx_cpy,
                    )
                    .await
                })
            }
            SourceType::Image(image) => {
                pt.send_text_message(&format!(
                    "I: Using disc image {:#?} ({}).",
                    image, disc_name
                ));

                spawn(async move {
                    extract_from_image(mm_cpy, image, target_cpy, titles_cpy, tx_cpy).await
                })
            }
        };
        workers.insert(disc_name, th_mkv);
    }

    // monitor progress of worker tasks and terminate
    // parse incoming events until the spawned task finishes
    let mut events = Vec::new();
    let batch_size = 128;
    let mut ep = EventParser::new();
    loop {
        tokio::select! {
            // batched processing of generated events to reduce overhead
            //
            // It is important to create/clear the progress bars and avoid
            // reusing the same objects because the presented runtime value
            // is derived from the progress bar's creation time:
            //
            // ⠦ [/dev/sr1] Copying all files (0%)  [░░░░░░░░░░░░░░░░░░░░] 00:00:25
            //                                                             ^^^^^^^^
            _ = rx.recv_many(&mut events, batch_size) => {
                // at least one spawned process is still running
                // --> try to parse generated events
                ep.parse_events(&mut events, &mut pt);
            },
            _ = sleep(Duration::from_millis(500)) => {
                // timeout reached, do something else
            }
        }

        // test if the worker threads are still running or if a thread has
        // finished and provided a result
        debug!("workers (pre-cleanup):  {}", workers.len());
        for (disc_name, worker) in workers.extract_if(|_, worker| worker.is_finished()) {
            // TODO do we need to drain potentially remaining events?
            match worker.await.unwrap() {
                Ok(result) => {
                    let elapsed = result.elapsed_secs;
                    let size_mb = (result.fs_size as f32) / 1024u32.pow(2) as f32;
                    let size_gb = (result.fs_size as f32) / 1024u32.pow(3) as f32;
                    let write_rate = size_mb / (elapsed as f32);
                    let message = format!(
                        "Extraction of '{}' completed after {} seconds. ({:.1}GiB, {:.1}MiB/s)",
                        disc_name, elapsed, size_gb, write_rate
                    );
                    pt.send_text_message(&message);
                    ep.jobs_done += 1;

                    let msg_4004 = ep.get_msg_4004();
                    if msg_4004 > 0 {
                        // MSG:4004 - The source file '<...>' is corrupt or invalid at offset ###, attempting to work around
                        warn!(
                            "Disc '{}' had {} potential issues. Please verify.",
                            disc_name, msg_4004
                        );
                    }

                    // TODO count total bytes
                }
                Err(error) => {
                    let message = format!("[{}] Extraction failed: {:#?}", disc_name, error.reason);
                    pt.send_text_message(&message);
                    ep.jobs_failed += 1;
                }
            }
        }
        // after this for-loop has run:
        // - 'workers' contains active threads
        // - 'drives_done' contains drives with successful backup
        // - 'drives_failed' contains drives with failed backup
        debug!("workers (post-cleanup): {}", workers.len());

        // FIXME progress bars do not update timestamps without sending an explicit progress update

        // FIXME the loop-termination may fail to work
        // - ripit-cli hangs and does not return to shell prompt
        // - potentially related to disk space issues (filesystem full)?
        // ----------------------------------------------------------------
        // I: Found 3 drives: /dev/sr0 /dev/sr1 /dev/sr2
        // I: Deleted existing directory "/media/backup/dump/_dev6_multi/DVDVolume". (0 MKV files, 0 other).
        // I: Using optical drive "/dev/sr2" (DVDVolume).
        // I: Using optical drive "/dev/sr1" (OTAKU_NO_VIDEO).
        // [makemkv] E: MSG:2018 - Error 'Posix error - Resource temporarily unavailable' occurred while writing data to '/media/backup/dump/_dev6_multi/DVDVolume/B1_t00.mkv' at offset '1207959552'
        // [makemkv] E: MSG:2018 - Error 'Posix error - Resource temporarily unavailable' occurred while writing data to '/media/backup/dump/_dev6_multi/OTAKU_NO_VIDEO/C1_t04.mkv' at offset '2181038080'
        // [makemkv] E: MSG:5003 - Failed to save title 4 to file /media/backup/dump/_dev6_multi/OTAKU_NO_VIDEO/C1_t04.mkv
        // [makemkv] E: MSG:5003 - Failed to save title 0 to file /media/backup/dump/_dev6_multi/DVDVolume/B1_t00.mkv
        // [makemkv] E: MSG:5004 - 17 titles saved, 1 failed
        // [makemkv] E: MSG:5037 - Copy complete. 17 titles saved, 1 failed.
        // [OTAKU_NO_VIDEO] Extraction completed backup after 867 seconds. (3.7GiB written, 4.4MiB/s)
        // [makemkv] E: MSG:2019 - Error 'Posix error - No such file or directory' occurred while creating '/media/backup/dump/_dev6_multi/DVDVolume/B1_t16.mkv'
        // [makemkv] E: MSG:5003 - Failed to save title 16 to file /media/backup/dump/_dev6_multi/DVDVolume/B1_t16.mkv
        // [makemkv] E: MSG:5004 - 15 titles saved, 2 failed
        // [makemkv] E: MSG:5037 - Copy complete. 15 titles saved, 2 failed.
        // ⠸ [OTAKU_NO_VIDEO] Saving all titles to MKV files (100%)    [████████████████████] 00:13:03
        // ⠇ [DVDVolume] Saving all titles to MKV files (100%)    [████████████████████] 00:20:43
        // ^C
        // ----------------------------------------------------------------
        if workers.is_empty() {
            break;
        }
    }

    // process remaining events to ensure the queue is empty
    let remaining = rx.len();
    debug!(
        "run(): Threads have finished. Draining remaining {} events.",
        remaining
    );
    if remaining > 0 {
        let _ = rx.recv_many(&mut events, remaining).await;
        ep.parse_events(&mut events, &mut pt);
    }

    // sanity check: this must never trigger
    if !rx.is_empty() {
        panic!(
            "Internal error: Receiver queue still contains {} messages!",
            rx.len()
        );
    }

    if ep.jobs_failed == 0 {
        exitcode::OK
    } else {
        exitcode::IOERR
    }
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

/// strip trailing slash or backslash from provided value
fn normalize_path(value: &Path) -> PathBuf {
    std::path::PathBuf::from(value.to_string_lossy().trim_end_matches(['/', '\\']))
}

fn check_directory_and_delete(directory: &PathBuf) -> Result<String, String> {
    if !directory.is_dir() {
        let error = format!("E: {:#?} exists but it's not a directory!", directory);
        return Err(error);
    }

    // check directory contents
    let mut mkv_files = 0usize;
    let mut non_mkv = 0usize;
    let mut unknown = 0usize;
    let entries = std::fs::read_dir(directory).expect("Unable to read directory");
    for result in entries {
        match result {
            Ok(entry) => {
                let filename = entry.path();
                if filename.is_file() {
                    match filename.extension() {
                        Some(ext) if ext.to_str().unwrap_or("") == "mkv" => {
                            mkv_files += 1;
                        }
                        Some(_) | None => {
                            non_mkv += 1;
                        }
                    }
                } else {
                    non_mkv += 1;
                }
            }
            Err(_e) => {
                unknown += 1;
            }
        }
    }

    // deletion can be dangerous - ask user for permission
    if unknown > 0 {
        let error = format!(
            "E: Directory {:#?} contains unexpected files! Unable to proceed.",
            directory
        );
        return Err(error);
    } else if non_mkv > 0 {
        // directory contains non-MKV files
        // --> ask user for confirmation
        let prompt_msg = format!("Recursively delete existing directory {:#?}?", directory);
        let proceed = Confirm::new()
            .with_prompt(prompt_msg)
            .default(false)
            .interact()
            .expect("Failed to read confirmation");

        if proceed {
            // user has confirmed
            // --> safe to delete
        } else {
            let error = format!(
                "E: User denied permission to delete directory {:#?}! Unable to proceed.",
                directory
            );
            return Err(error);
        }
    } else if mkv_files > 0 {
        // directory contains mkv files and nothing else
        // --> relatively safe to delete
    } else {
        // directory is empty
        // --> safe to delete
    }

    match std::fs::remove_dir_all(directory) {
        Ok(_) => {
            let msg = format!(
                "I: Deleted existing directory {:#?}. ({} MKV files, {} other).",
                directory, mkv_files, non_mkv
            );
            Ok(msg)
        }
        Err(e) => {
            let error = format!("E: Unable to delete directory {:#?}: {:#?}", directory, e);
            Err(error)
        }
    }
}

struct EventParser {
    pub jobs_done: usize,
    pub jobs_failed: usize,
    stages: HashMap<String, HashMap<String, Stage>>,
    msg_4004: usize,
}

impl EventParser {
    fn new() -> Self {
        EventParser {
            jobs_done: 0,
            jobs_failed: 0,
            msg_4004: 0,
            stages: HashMap::new(),
        }
    }

    pub fn get_msg_4004(&self) -> usize {
        self.msg_4004
    }

    fn parse_events(&mut self, events: &mut Vec<ExtractEvent>, pt: &mut ProgressTracker) {
        // during an extraction we expect to see the following events:
        // ----------------------------------------------------------------
        // PRGT:5018,0,"Scanning CD-ROM devices"
        //   PRGC:5018,0,"Scanning CD-ROM devices"
        // PRGT:3100,0,"Opening DVD disc"
        //   PRGC:3102,0,"Processing title sets"
        //   PRGC:3120,1,"Scanning contents"
        //   PRGC:3103,0,"Processing titles"
        //   PRGC:3104,0,"Decrypting data"
        // PRGT:5024,0,"Saving all titles to MKV files"
        //   <repeating for each output file>
        //   PRGC:5057,0,"Analyzing seamless segments"
        //   PRGC:5017,0,"Saving to MKV file"
        //   PRGC:5057,1,"Analyzing seamless segments"
        //   PRGC:5017,1,"Saving to MKV file"
        //             ^-- index of generated output file
        //   </repeating for each output file>
        // ----------------------------------------------------------------
        // local convention:
        // - let's refer to PRGT records as 'stages'
        // - let's refer to PRGC records as 'tasks'
        for event in events.drain(..) {
            match event {
                ExtractEvent::MsgInfo(msg) => {
                    info!(
                        "[{}] MSG:{} - {}",
                        msg.source, msg.message.code, msg.message.message
                    );
                    // pt.send_text_message(&format!("[makemkv] I: MSG:{} - {}", msg.message.code, msg.message.message));
                }
                ExtractEvent::MsgWarn(msg) => {
                    if msg.message.code == 4004 {
                        self.msg_4004 += 1;
                    } else {
                        warn!(
                            "[{}] MSG:{} - {}",
                            msg.source, msg.message.code, msg.message.message
                        );
                    }
                }
                ExtractEvent::MsgFail(msg) => {
                    error!(
                        "[{}] MSG:{} - {}",
                        msg.source, msg.message.code, msg.message.message
                    );
                }
                ExtractEvent::ProgressT(pu) => {
                    // there should be exactly 3 stages per extraction:
                    // "Scanning…" / "Opening…" / "Saving…"
                    debug!(
                        "Create PRGT progress bar for '{}' ({}).",
                        pu.source, pu.prgt.code
                    );

                    // create a progress bar for the current stage
                    let pb_prgt_id = format!("{}_prgt", pu.source);
                    let stage_name = pu.get_stage_name();
                    let disc_name = pu.disc.get_name();
                    pt.create_progress_bar(&pb_prgt_id, disc_name, &stage_name);
                }
                ExtractEvent::ProgressC(pu) => {
                    // each stage has one or more tasks
                    debug!(
                        "Create PRGC progress bar for '{}' ({}:{}).",
                        pu.source, pu.prgt.code, pu.prgc.code
                    );

                    // create a progress bar for the current task
                    // (the progress bar is anchored to its stage)
                    let pb_prgt_id = format!("{}_prgt", pu.source);
                    let pb_prgc_id = format!("{}_prgc", pu.source);
                    let device_name = pu.source.clone();
                    let task_name = pu.get_task_name();
                    pt.create_progress_bar_after(
                        &pb_prgc_id,
                        &device_name,
                        &task_name,
                        &pb_prgt_id,
                    )
                    .unwrap();
                }
                ExtractEvent::ProgressValue(pu) => {
                    // "Processing title sets"
                    // "Scanning contents"
                    let device_name = pu.source.clone();
                    let stage_name = pu.get_stage_name(); // reported as 'total'
                    let task_name = pu.get_task_name(); // reported as 'current'
                    let device_stage = self
                        .stages
                        .entry(device_name.clone())
                        .or_insert(HashMap::from([(stage_name.clone(), Stage::new())]));
                    let stage = device_stage
                        .entry(stage_name.clone())
                        .or_insert(Stage::new());

                    let disc_name = pu.disc.get_name();
                    let disc_name_fmt = format!("[{}]", disc_name);

                    let prgt_old = stage.get_prgt();
                    let prgt_new = pu.prgt.percentage;
                    let prgc_old = stage.get_prgc();
                    let prgc_new = pu.prgc.percentage;

                    // update stored values
                    stage.update_progress(prgt_new, prgc_new);

                    // report to user

                    // throttle log output: notify only if stage or
                    // percentage has changed more than 5%
                    // (prevent log-flooding)
                    if prgc_old.is_nan() || prgt_new > prgt_old || prgc_new >= prgc_old + 5.0 {
                        info!(
                            "[{}] {}: {:3.0}% (done: {}, failed: {})",
                            device_name, stage_name, prgt_new, self.jobs_done, self.jobs_failed
                        );
                    }

                    let pb_prgt_id = format!("{}_prgt", pu.source);
                    let pb_prgc_id = format!("{}_prgc", pu.source);

                    // throttle progress bars: notify only if relevant
                    // progress was made
                    // (indicatif handles fine-grained throttling)
                    //
                    // It is possible for the 'total percentage' percentage
                    // to change while the 'current percentage' (PRGC)
                    // remains at the previous value. This doesn't really
                    // make sense but it's the way it is and must be
                    // handled appropriately.
                    if prgc_old.is_nan() || prgt_new > prgt_old || prgc_new >= prgc_old + 0.1 {
                        // update progress bars for current stage and task
                        pt.update_progress_bar(
                            &pb_prgt_id,
                            disc_name,
                            &stage_name,
                            pu.prgt.percentage,
                        )
                        .unwrap();

                        // modify the task name to make it more
                        // obvious how stage and task are related:
                        // ----------------------------------------
                        // ⠸ [/dev/sr0] Copying all files (6%)    [█░░░░░░░░░░░░░░░░░░░] 00:03:25
                        // ⠹ [/dev/sr0] `--> Copying file (25%)   [████░░░░░░░░░░░░░░░░] 00:00:12
                        // ----------------------------------------
                        let task_name_mod = format!("`--> {}", task_name);
                        pt.update_progress_bar(
                            &pb_prgc_id,
                            &disc_name_fmt,
                            &task_name_mod,
                            pu.prgc.percentage,
                        )
                        .unwrap();
                    } else {
                        debug!(
                            "task pct: old {:5.2}% new {:5.2}% (skip)",
                            prgc_old, prgc_new
                        );
                    }

                    if prgt_new == 100.0 {
                        // record completion
                        pt.send_text_message(&format!(
                            "🗸 {:40} '{}' finished after {} seconds.",
                            disc_name_fmt,
                            stage_name,
                            stage.get_elapsed()
                        ));
                        // remove progress bars
                        pt.clear_progress_bar(&pb_prgc_id).unwrap();
                        pt.clear_progress_bar(&pb_prgt_id).unwrap();
                    }
                }
            }
        }
    }
}
