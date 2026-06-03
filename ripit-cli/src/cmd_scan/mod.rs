/*
    ripit-cli scan <SOURCE>
*/

use std::{fs, path::PathBuf};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use clap::{Parser, ValueHint};

use crate::yaml_utils::generate_yaml;

#[derive(Parser, Debug)]
pub struct CmdArgs {
    /// directory, filename, device name, or drive letter
    #[arg(value_hint = ValueHint::FilePath)]
    source: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Write output YAML to a file
    #[arg(short = 'o', long = "output-file", value_hint = ValueHint::FilePath)]
    output_file: Option<PathBuf>,
}

pub fn run(args: CmdArgs, makemkvcon_bin: &PathBuf) -> i32 {
    if args.verbose {
        println!("[verbose] scan {}", args.source);
    }

    // makemkvcon's default: 120 seconds
    let min_length = 0;

    let scan_result = match makemkvcon::info(makemkvcon_bin, &args.source, min_length) {
        Some(x) => x,
        None => {
            return exitcode::DATAERR;
        }
    };

    if scan_result.errors > 0 {
        error!(
            "Detected {} errors while scanning '{}'!",
            scan_result.errors, &args.source
        );
        return exitcode::DATAERR;
    }

    let yaml = generate_yaml(scan_result.parsed, true);
    if let Some(output_file) = args.output_file {
        match fs::write(&output_file, yaml) {
            Ok(_) => info!("Wrote scan output to '{}'.", output_file.display()),
            Err(err) => {
                error!(
                    "Failed to write output file '{}': {}",
                    output_file.display(),
                    err
                );
                return exitcode::OSFILE;
            }
        }
    } else {
        println!("{}", yaml);
    }

    if scan_result.issues == 0 {
        info!("Successfully scanned '{}'.", &args.source);
    } else {
        warn!(
            "Detected {} potential issues while scanning '{}'! Please validate.",
            scan_result.issues, &args.source
        );
    }
    exitcode::OK
}
