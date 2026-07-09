/*
    ripit-cli unshackle <SOURCE> <TARGET>
*/

use std::path::Path;

use clap::{Parser, ValueHint};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use ripit::{UnshackleError, eject_medium, unshackle_disc};

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
            Ok(_) => {
                if !args.continuous {
                    return exitcode::OK;
                }
            }
            Err(UnshackleError::ReadError) => {
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
            Err(UnshackleError::NoDrive) => {
                error!("Unable to find optical drive '{}'! Aborting.", source_str);
                return exitcode::OSFILE;
            }
            Err(UnshackleError::NoMedium) => {
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
            Err(UnshackleError::LogError) => {
                error!("Unable to create log file! Aborting.");
                return exitcode::OSFILE;
            }
        }
    }
}
