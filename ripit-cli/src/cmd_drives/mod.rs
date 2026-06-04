/*
    ripit-cli scan <SOURCE>
*/

use std::{fs, path::Path, path::PathBuf};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use clap::{Parser, ValueHint};

#[derive(Parser, Debug)]
pub struct CmdArgs {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Write output YAML to a file
    #[arg(short = 'o', long = "output-file", value_hint = ValueHint::FilePath)]
    output_file: Option<PathBuf>,
}

pub fn run(args: CmdArgs, makemkvcon_bin: &Path) -> i32 {
    let (result, issues) = makemkvcon::drives(makemkvcon_bin);
    if !result.is_empty() {
        let yaml = serde_yaml::to_string(&result).unwrap();
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
    } else {
        info!("Unable to find any optical drives.")
    }

    if issues > 0 {
        warn!(
            "Detected {} potential issues during parsing! Please validate.",
            issues
        );
        exitcode::OK
    } else {
        exitcode::DATAERR
    }
}
