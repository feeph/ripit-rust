/*
    ripit-cli unshackle <SOURCE> <TARGET>
*/

//mod os_utils;

use std::path::Path;

use clap::{Parser, ValueHint};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use ripit::eject::eject_medium;
use ripit::unshackle::{UnshackleResult, unshackle_disc};

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
                    eject_medium(&args.source);
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
                    eject_medium(&args.source);
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
