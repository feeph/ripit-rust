/*!
    ripit-cli scan [OPTIONS]

    (use `ripit-cli scan --help` for full usage instructions)

    usage:

    ```TEXT
    # read from an optical drive and show disc's content
    ripit-cli scan -d E:
    ripit-cli scan -d /dev/sr0

    # read from a disc image
    ripit-cli scan -i feature.iso

    # write disc's content to YAML file
    ripit-cli scan -d E:        -f YAML -o content.yaml
    ripit-cli scan -i image.iso -f YAML -o content.yaml
    ```

    output example:

    ```TEXT
    $ ripit-cli scan -d /dev/sr0
    <...>
    ```
*/

mod event_parser;

// standard library imports
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

// third-party imports
use clap::{Parser, ValueHint};
use indicatif_log_bridge::LogWrapper;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use makemkv::{DiscContent, StreamRecord};
use serde::Serialize;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

// crate-provided imports
use crate::progress_tracker::ProgressTracker;
use event_parser::EventParser;
use ripit::{
    OpticalDrive, ScanEvent, ScanMode, find_drives, find_matching_drive, scan_drive, scan_image,
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

// Please note:
// Trailing dots in ///-comments are stripped by clap.
#[derive(Parser, Debug)]
pub struct CmdArgs {
    /// Select a drive, e.g. 'D:' or '/dev/sr0' (repeatable)
    /// (implies usage of all drives if none are selected)
    #[arg(short = 'd', long = "drive")]
    drive_want: Option<PathBuf>,

    /// Select an image, e.g. 'image.iso' (repeatable)
    #[arg(short = 'i', long = "disc-image")]
    disc_image: Option<PathBuf>,

    /// Ignore all titles shorter than x seconds (env: MIN_LENGTH)
    #[arg(
        short = 'L',
        long = "min-length",
        default_value = "0",
        env = "MIN_LENGTH"
    )]
    min_length: usize,

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

    debug!("drive:           {:#?}", args.drive_want);
    debug!("disc_image:      {:#?}", args.disc_image);
    debug!("min_length:      {:#?}", args.min_length);
    debug!("eject_when_done: {:#?}", args.eject_when_done);
    debug!("output_file:     {:#?}", args.output_file);
    debug!("output_format:   {:#?}", args.output_format);

    let mut pt = ProgressTracker::new();

    // augment MultiProgress with indicatif-log-bridge to avoid
    // messages emitted by log-crate breaking indicatif's output
    // <https://crates.io/crates/indicatif-log-bridge>
    LogWrapper::new(pt.get_mp(), logger).try_init().unwrap();
    log::set_max_level(level);

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
    // (depending on how many drives are present and if they are busy)
    let (source_type, disc_name) = if let Some(filename) = args.disc_image {
        if filename.is_file() || filename.is_dir() {
            // file or directory: assume image
            // -> use the filename (without extension) as disc's name
            let disc_name: String = filename
                .file_stem()
                .expect("not a filename?")
                .to_string_lossy()
                .to_string();
            (SourceType::Image(filename), disc_name)
        } else {
            error!(
                "Image {:#?} is neither a file or directory. Ignoring.",
                filename
            );
            return exitcode::DATAERR;
        }
    } else if let Some(drive) = args.drive_want {
        let scan_mode = ScanMode::DriveAndDisc;
        let drives_have = find_drives(mm.to_owned(), scan_mode)
            .await
            .expect("Reading drives should never fail.");
        debug!("drives_have: {:#?}", drives_have);
        debug!("drive_want:  {:#?}", drive);

        if drives_have.is_empty() {
            error!("Found no available drives!");
            return exitcode::IOERR;
        } else {
            let result = find_matching_drive(&drives_have, &drive);
            match &result {
                Some(drive) => match drive.get_disc_name() {
                    Some(disc_name) => (SourceType::Drive(drive.clone()), disc_name.to_string()),
                    None => {
                        error!("E: Drive {:#?} has no disc!", drive.device);
                        return exitcode::IOERR;
                    }
                },
                None => {
                    error!("Found no matching drive!");
                    return exitcode::IOERR;
                }
            }
        }
    } else {
        error!("Unable to continue: Must provide a drive or image!");
        return exitcode::CONFIG;
    };

    let (tx, mut rx) = mpsc::channel::<ScanEvent>(256);

    let eject_when_done = args.eject_when_done;

    debug!("disc_name:       {}", disc_name);
    let mm_mkv = mm.clone();

    // create copies of relevant resources for 'async move{}'
    let mm_cpy = mm_mkv.clone();
    let tx_cpy = tx.clone();

    // spawn the source-specific extraction process
    let th_mkv = match source_type {
        SourceType::Drive(drive) => {
            pt.send_text_message(&format!(
                "I: Using optical drive {:#?} ({}).",
                drive.device, disc_name
            ));

            spawn(async move { scan_drive(mm_cpy, drive, eject_when_done, tx_cpy).await })
        }
        SourceType::Image(image) => {
            pt.send_text_message(&format!(
                "I: Using disc image {:#?} ({}).",
                image, disc_name
            ));

            spawn(async move { scan_image(mm_cpy, image, tx_cpy).await })
        }
    };

    // monitor progress of worker tasks and terminate
    // parse incoming events until the spawned task finishes
    let mut events = Vec::new();
    let batch_size = 128;
    let mut ep = EventParser::new();
    while !th_mkv.is_finished() {
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
    }

    let disc_content = match th_mkv.await.unwrap() {
        Ok(result) => {
            let name = result.disc_content.info.name.clone().unwrap_or_default();
            let elapsed = result.elapsed_secs;
            let message = format!("Scanning '{}' completed after {} seconds.", name, elapsed);
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
            result.disc_content
        }
        Err(error) => {
            error!("Scanning of '{}' failed: {:#?}", disc_name, error.reason);
            return exitcode::IOERR;
        }
    };

    // FIXME progress bars do not update timestamps without sending an explicit progress update

    // FIXME the loop-termination may fail to work
    // - ripit-cli hangs and does not return to shell prompt
    // - potentially related to disk space issues (filesystem full)?
    // ----------------------------------------------------------------
    // I: Found 3 drives: /dev/sr0 /dev/sr1 /dev/sr2
    // I: Deleted existing directory "<…>/DVDVolume". (0 MKV files, 0 other).
    // I: Using optical drive "/dev/sr2" (DVDVolume).
    // I: Using optical drive "/dev/sr1" (OTAKU_NO_VIDEO).
    // [makemkv] E: MSG:2018 - Error 'Posix error - Resource temporarily unavailable' occurred while writing data to '<…>/DVDVolume/B1_t00.mkv' at offset '1207959552'
    // [makemkv] E: MSG:2018 - Error 'Posix error - Resource temporarily unavailable' occurred while writing data to '<…>/OTAKU_NO_VIDEO/C1_t04.mkv' at offset '2181038080'
    // [makemkv] E: MSG:5003 - Failed to save title 4 to file <…>/OTAKU_NO_VIDEO/C1_t04.mkv
    // [makemkv] E: MSG:5003 - Failed to save title 0 to file <…>/DVDVolume/B1_t00.mkv
    // [makemkv] E: MSG:5004 - 17 titles saved, 1 failed
    // [makemkv] E: MSG:5037 - Copy complete. 17 titles saved, 1 failed.
    // [OTAKU_NO_VIDEO] Extraction completed backup after 867 seconds. (3.7GiB written, 4.4MiB/s)
    // [makemkv] E: MSG:2019 - Error 'Posix error - No such file or directory' occurred while creating '<…>/DVDVolume/B1_t16.mkv'
    // [makemkv] E: MSG:5003 - Failed to save title 16 to file <…>/DVDVolume/B1_t16.mkv
    // [makemkv] E: MSG:5004 - 15 titles saved, 2 failed
    // [makemkv] E: MSG:5037 - Copy complete. 15 titles saved, 2 failed.
    // ⠸ [OTAKU_NO_VIDEO] Saving all titles to MKV files (100%)    [████████████████████] 00:13:03
    // ⠇ [DVDVolume] Saving all titles to MKV files (100%)    [████████████████████] 00:20:43
    // ^C
    // ----------------------------------------------------------------

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

    let output_format = args.output_format;
    let (extension, output, output_type) = match output_format {
        OutputFormat::Text => {
            let output = generate_text(&disc_content);
            ("txt", output, "TEXT")
        }
        OutputFormat::Yaml => {
            let output = generate_yaml(disc_content);
            ("yaml", output, "YAML")
        }
    };

    // write content to console or output file
    //
    // filename is provided by the user:
    // - can't use a disc's name or volume name since the metadata might
    //   not provide any
    // - if reading from a drive, we might use the disc label
    // - if reading from an image, we might use the filename
    match args.output_file {
        Some(mut filename) => {
            filename.set_extension(extension);
            let message = format!("Writing {} output to {:#?}.", output_type, filename);
            pt.send_text_message(&message);
            if write_file(&filename, &output) {
                exitcode::OK
            } else {
                exitcode::IOERR
            }
        }
        None => {
            info!("Finished, need to write output to console.");
            println!("{}", output);
            exitcode::OK
        }
    }
}

// ------------------------------------------------------------------------
// private helper functions
// ------------------------------------------------------------------------

fn generate_text(dc: &DiscContent) -> String {
    let mut lines = Vec::<String>::new();
    let mut total_size = 0;

    lines.push(format!(
        "name:   {}",
        dc.info.name.clone().unwrap_or_default()
    ));
    lines.push(format!(
        "volume: {}",
        dc.info.volume_name.clone().unwrap_or_default()
    ));

    lines.push("-".repeat(80));
    for (tid, tr) in dc.titles.iter() {
        let filename = tr.info.output_file_name.clone().unwrap_or_default();
        let duration = tr.info.duration.clone().unwrap_or_default();
        lines.push(format!("{}: {} ({})", tid, filename, duration));

        // use a set to automatically deduplicate values
        let mut audio_tracks = BTreeSet::<String>::new();
        let mut subtitles = BTreeSet::<String>::new();
        for sr in tr.streams.values() {
            match &sr {
                StreamRecord::Video(v_stream) => {
                    info!("{}", v_stream.metadata_language_name);
                }
                StreamRecord::Audio(a_stream) => {
                    if a_stream.lang_name != "<unknown>" {
                        audio_tracks.insert(a_stream.lang_name.clone());
                    }
                    if a_stream.metadata_language_name != "<unknown>" {
                        audio_tracks.insert(a_stream.metadata_language_name.clone());
                    }
                }
                StreamRecord::Subtitle(s_stream) => {
                    if s_stream.lang_name != "<unknown>" {
                        subtitles.insert(s_stream.lang_name.clone());
                    }
                    if s_stream.metadata_language_name != "<unknown>" {
                        subtitles.insert(s_stream.metadata_language_name.clone());
                    }
                }
            }
        }
        let audio_tracks_str = audio_tracks
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        let subtitles_str = subtitles
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  audio:     {}", audio_tracks_str));
        lines.push(format!("  subtitles: {}", subtitles_str));

        // aggregate all file sizes
        // (it is possible for total size to drastically exceed the medium's size)
        total_size += tr.info.disk_size_bytes;
    }

    lines.push("-".repeat(80));

    let total_size_gb = total_size as f32 / 1024.0 / 1024.0 / 1024.0;
    lines.push(format!("total size: {:.1} GiB", total_size_gb));

    lines.join("\n")
}

fn generate_yaml<T: Serialize>(data: T) -> String {
    let yaml_value = serde_yaml::to_value(&data).unwrap();
    serde_yaml::to_string(&yaml_value).unwrap()
}

fn write_file(filename: &PathBuf, content: &str) -> bool {
    let mut fh = File::create(filename).ok().unwrap();
    let _ = write!(fh, "{}", content);

    true
}
