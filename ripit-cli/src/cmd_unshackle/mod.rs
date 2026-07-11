/*
    ripit-cli unshackle <SOURCE> <TARGET>
*/

use std::path::Path;

use clap::{Parser, ValueHint};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use ripit::unshackle_discs;

#[derive(Parser, Debug)]
pub struct CmdArgs {
    /// Select a drive, e.g. 'D:' or '/dev/sr0' (repeatable)
    /// (implies 'all drives' if none are selected)
    #[arg(short = 'd', long = "drive", value_name = "DRIVE")]
    drives: Vec<String>,

    /// Target directory
    #[arg(value_hint = ValueHint::DirPath)]
    target: std::path::PathBuf,

    /// Eject medium from the drive after backup
    #[arg(short = 'e', long = "eject-when-done")]
    eject: bool,

    /// Enable continuous mode (does not exit when done, implies eject)
    #[arg(short = 'c', long = "continuous")]
    continuous: bool,

    /// Allow overwriting existing destination files
    #[arg(short = 'O', long = "allow-overwrite")]
    allow_overwrite: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

// pub fn child(makemkvcon_bin: PathBuf, source: PathBuf, target: PathBuf, eject: bool, continuous: bool, allow_overwrite: bool) -> i32 {
//     let source_str = source.to_string_lossy().to_string();
//     let target_str = target.to_string_lossy().to_string();
//     info!(
//         "Running unshackle with source='{}' target='{}'",
//         source_str, target_str
//     );

//     let eject_when_done = eject || continuous;
//     let wait_time = 10;
//     let dur = std::time::Duration::from_secs(wait_time);
//     loop {
//         match unshackle_disc(
//             &makemkvcon_bin,
//             &source,
//             &target,
//             eject_when_done,
//             allow_overwrite,
//         ) {
//             Ok(_) => {
//                 if !continuous {
//                     return exitcode::OK;
//                 }
//             }
//             Err(UnshackleError::ReadError) => {
//                 if continuous {
//                     error!(
//                         "Unable to read disc in drive '{}'! Idling for {} seconds.",
//                         source_str, wait_time
//                     );
//                     eject_medium(&source);
//                     std::thread::sleep(dur);
//                 } else {
//                     return exitcode::OSFILE;
//                 }
//             }
//             Err(UnshackleError::NoDrive) => {
//                 error!("Unable to find optical drive '{}'! Aborting.", source_str);
//                 return exitcode::OSFILE;
//             }
//             Err(UnshackleError::NoMedium) => {
//                 if continuous {
//                     info!(
//                         "No disc found in drive '{}'. Idling for {} seconds.",
//                         source_str, wait_time
//                     );
//                     std::thread::sleep(dur);
//                 } else {
//                     info!("Unable to find medium in drive '{}'.", source_str);
//                     return exitcode::OK;
//                 }
//             }
//             Err(UnshackleError::LogError) => {
//                 error!("Unable to create log file! Aborting.");
//                 return exitcode::OSFILE;
//             }
//         }
//     }
// }

pub fn run(args: CmdArgs, makemkvcon_bin: &Path) -> i32 {
    let handle = unshackle_discs(
        makemkvcon_bin,
        &args.drives,
        &args.target,
        args.eject,
        args.allow_overwrite,
        args.continuous,
    );
    let _ = futures::executor::block_on(handle);

    exitcode::OK
}
