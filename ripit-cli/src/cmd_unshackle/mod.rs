/*
    ripit-cli unshackle <SOURCE> <TARGET>
*/

mod os_utils;

use std::{fs, path::Path};

use clap::{Parser, ValueHint};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use makemkvcon::DriveRecord;

use crate::yaml_utils::generate_yaml;

#[derive(Parser, Debug)]
pub struct CmdArgs {
    /// A path-like source: device name or drive letter
    #[arg(value_hint = ValueHint::FilePath)]
    source: std::path::PathBuf,

    /// Target directory
    #[arg(value_hint = ValueHint::DirPath)]
    target: std::path::PathBuf,

    /// Enable continuous mode (does not exit when done, implies eject)
    #[arg(short, long)]
    continuous: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Allow overwriting existing destination files
    #[arg(short = 'O', long = "allow-overwrite")]
    allow_overwrite: bool,

    /// Eject medium from the drive after backup
    #[arg(short, long)]
    eject: bool,
}

fn find_drive_record(makemkvcon_bin: &Path, source: &str) -> Option<DriveRecord> {
    let (drive_records, _) = makemkvcon::drives(makemkvcon_bin);

    // find the drive matching the provided device name
    // - provides mapping from device name to disc id (required for backup)
    // - provides disc content type
    drive_records
        .into_iter()
        .find(|dr| dr.device_name == source)
}

enum UnshackleResult {
    ReadFailure,
    ScanFailure,
    ParseFailure,
    NoDrive,
    NoMedium,
    Success,
}

fn unshackle_disc(
    makemkvcon_bin: &Path,
    source: &Path,
    target: &Path,
    eject_when_done: bool,
    allow_overwrite: bool,
) -> UnshackleResult {
    let source_str = source.to_string_lossy();

    // the order of operations improves overall speed
    // 1. scan drives (fast, detect 'no drive' vs. 'no medium')
    // 2. identify medium (fast)
    // 3. extract from optical drive to disk (typically 10-20 minutes)
    // 4. scan the medium using the disk (faster than reading from drive)
    //
    // step 4 could be merged with step 2 but then we have to read the
    // medium two times from the optical drive; scanning from drive is
    // slower by a factor of 20 compared to scanning from disk (e.g. 6
    // seconds when reading from disk vs. 120 seconds reading from drive)

    // --------------------------------------------------------------------
    // step 1 - scan drives
    // --------------------------------------------------------------------

    debug!("step 2 - identify drive");
    // find the drive matching the provided device name or drive letter
    // - provides mapping from device name to disc id (required for backup)
    // - provides disc content type
    let drive = match find_drive_record(makemkvcon_bin, &source_str) {
        Some(dr) => {
            debug!(
                "Found optical drive: {} -> {}",
                dr.device_name, dr.disc_name
            );
            dr
        }
        None => {
            return UnshackleResult::NoDrive;
        }
    };

    // --------------------------------------------------------------------
    // step 2 - identify medium (volume label and block id)
    // --------------------------------------------------------------------

    debug!("step 1 - identify medium in drive '{}'.", source_str);
    let mut target_dump = std::path::PathBuf::from(target);
    let mut target_yaml = std::path::PathBuf::from(target);
    let result = os_utils::get_volume_id(&source_str);
    match result {
        Some(x) => {
            debug!(
                "Drive '{}' contains a medium with volume ID '{}'.",
                source_str, x
            );
            // append volume ID to target in order to create a unique
            // filesystem location for each disc (this is a precaution
            // since the volume name may be something stupid like
            // "DVDVolume", "LOGICAL_VOLUME_ID" or "UNDEFINED" and cause
            // filesystem conflicts if multiple discs share the same name)
            target_dump.push(&x);
            target_yaml.push(&x);
        }
        None => {
            return UnshackleResult::NoMedium;
        }
    }

    // ------------------------------------------------------------
    // step 3 - extract content
    // ------------------------------------------------------------

    debug!("step 3 - extract content");
    let disc_id = drive.index;
    if drive.content_type.has_dvd_files {
        // DVDs are extracted as images
        target_dump.add_extension("iso");
    }

    let result = makemkvcon::backup(makemkvcon_bin, disc_id, &target_dump, allow_overwrite);

    // --------------------------------------------------------------------
    // step 4 - scan extracted medium and save as YAML file
    // --------------------------------------------------------------------

    if result {
        debug!("step 4 - scan extracted content");
        let min_length = 0;
        target_yaml.add_extension("yaml");
        info!("target_yaml: {}", target_yaml.to_string_lossy());
        let scan_result =
            match makemkvcon::info(makemkvcon_bin, &target_dump.to_string_lossy(), min_length) {
                Some(x) => x,
                None => {
                    error!("Unable to scan '{}'!", source_str);
                    return UnshackleResult::ScanFailure;
                }
            };

        let yaml = generate_yaml(scan_result.parsed, true);
        match fs::write(&target_yaml, yaml) {
            Ok(_) => info!("Wrote scan output to '{}'.", target_yaml.display()),
            Err(err) => {
                error!(
                    "Failed to write output file '{}': {}",
                    target_yaml.display(),
                    err
                );
                return UnshackleResult::ParseFailure;
            }
        }

        if eject_when_done {
            info!("Ejecting medium from '{}'.", source_str);
            os_utils::eject_medium(source);
        }
        UnshackleResult::Success
    } else {
        UnshackleResult::ReadFailure
    }
}

pub fn run(args: CmdArgs, makemkvcon_bin: &Path) -> i32 {
    let source_str = args.source.to_string_lossy().to_string();
    let target_str = args.target.to_string_lossy().to_string();
    if args.verbose {
        info!("[verbose] unshackle {} -> {}", source_str, target_str);
    }
    info!(
        "Running unshackle with source='{}' target='{}'",
        source_str, target_str
    );

    let eject_when_done = args.eject || args.continuous;
    let wait_time = 10;
    let dur = std::time::Duration::from_secs(wait_time);
    loop {
        match unshackle_disc(
            makemkvcon_bin,
            &args.source,
            &args.target,
            eject_when_done,
            args.allow_overwrite,
        ) {
            UnshackleResult::Success => {
                if !args.continuous {
                    return exitcode::OK;
                }
            }
            UnshackleResult::ReadFailure => {
                // unable to read the disc at all
                if args.continuous {
                    error!(
                        "Unable to read disc in drive '{}'! Idling for {} seconds.",
                        source_str, wait_time
                    );
                    os_utils::eject_medium(&args.source);
                    std::thread::sleep(dur);
                } else {
                    return exitcode::OSFILE;
                }
            }
            UnshackleResult::ScanFailure | UnshackleResult::ParseFailure => {
                // able to read the disc but unable to parse its content
                if args.continuous {
                    error!(
                        "Unable to process disc in drive '{}'! Idling for {} seconds.",
                        source_str, wait_time
                    );
                    os_utils::eject_medium(&args.source);
                    std::thread::sleep(dur);
                } else {
                    return exitcode::OSFILE;
                }
            }
            UnshackleResult::NoDrive => {
                error!("Unable to find optical drive '{}'! Aborting.", source_str);
                return exitcode::OSFILE;
            }
            UnshackleResult::NoMedium => {
                if args.continuous {
                    info!(
                        "No disc found in drive '{}'. Idling for {} seconds.",
                        source_str, wait_time
                    );
                    std::thread::sleep(dur);
                } else {
                    info!("Unable to find medium in drive '{}'.", source_str);
                    return exitcode::OK;
                }
            }
        }
    }
}
