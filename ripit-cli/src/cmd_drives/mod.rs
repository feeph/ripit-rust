/*
    ripit-cli drives

    example output:

    ```TEXT
    $ ripit-cli drives
    DRV:0 /dev/sr1 DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ DiscInserted
        OTAKU_NO_VIDEO [DVD]
    DRV:1 /dev/sr2 BD-RE ASUS SBW-06D5H-U E101 AFDL222859WL DiscInserted
        DVDVolume [DVD]
    DRV:2 /dev/sr0 BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635 DiscInserted
        COLLATERAL [Blu-ray] [AACS]
    ```
*/

// standard library imports
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// third-party imports
use clap::{Parser, ValueHint};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use serde::Serialize;

// crate-provided imports
use makemkv::ScanMode;
use ripit::{find_drives, find_matching_drives};

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
    /// Select a drive, e.g. 'D:' or '/dev/sr0' (repeatable)
    /// (implies 'all drives' if none are selected)
    #[arg(short = 'd', long = "drive")]
    drives_want: Vec<PathBuf>,

    /// Disable scanning of inserted media. (improves speed)
    #[arg(short = 's', long = "disable-scan", default_value = "false")]
    disable_scan: bool,

    /// Write output YAML to a file.
    #[arg(short = 'o', long = "output-file", value_hint = ValueHint::FilePath)]
    output_file: Option<PathBuf>,

    /// Select output format.
    #[arg(short = 'f', long = "output-format", default_value_t, value_enum)]
    output_format: OutputFormat,

    #[clap(flatten)]
    global_opts: crate::GlobalOpts,
}

/// example output:
///
/// ```TEXT
/// [DRV:1] /dev/sr2 'BD-RE ASUS SBW-06D5H-U E101 AFDL222859WL'
///   'MASTERS OF THE UNIVERSE' (id: 0335ff7020202020) [Blu-Ray] [AACS]
/// [DRV:0] /dev/sr1 'DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ'
///   'OTAKU_NO_VIDEO' (3ed3dd1f5f5f5f4d) [DVD]
/// [DRV:2] /dev/sr0 'BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635'
///   'WARGAMES' (id: 5a6b874e20574152) [Blu-Ray] [AACS]
/// ```
pub async fn run(args: CmdArgs, mm: &makemkv::MakeMkv) -> i32 {
    crate::logging::init_logger(args.global_opts.log_level);

    let scan_mode = if args.disable_scan {
        ScanMode::DriveOnly
    } else {
        ScanMode::DriveAndDisc
    };
    let drives_have = find_drives(mm.to_owned(), scan_mode)
        .await
        .expect("Reading drives should never fail.");
    let drives_want = args.drives_want;

    let drives = find_matching_drives(&drives_have, &drives_want);
    debug!("drives_have: {:#?}", drives_have);
    debug!("drives_want: {:#?}", drives_want);
    debug!("drives:      {:#?}", drives);

    let output_str = match args.output_format {
        OutputFormat::Text => {
            info!("Generating TEXT output.");
            let mut lines = Vec::new();
            for drive in &drives {
                lines.push(format!(
                    "[DRV:{}] {} '{}'",
                    drive.index,
                    drive.device.to_string_lossy(),
                    drive.model
                ));
                // TODO restore drive detection functionality
                // match &drv.disc_ {
                //     Disc::NoDisc => {},
                //     Disc::Dvd(disc) => {
                //         lines.push(format!("  '{}' ({}) [DVD]", disc.name, disc.uid));
                //     },
                //     Disc::HdDvd(disc) => {
                //         lines.push(format!("  '{}' ({}) [HD-DVD]", disc.name, disc.uid));
                //     },
                //     Disc::BluRay(disc) => {
                //         let mut tags = Vec::new();
                //         if disc.has_aacs {
                //             tags.push("AACS");
                //         }
                //         if disc.has_bdsvm {
                //             tags.push("BD+");
                //         }
                //         lines.push(format!("  '{}' (id: {}) [Blu-Ray] [{}]", disc.name, disc.uid, tags.join(", ")));
                //     },
                // }
            }
            lines.join("\n")
        }
        OutputFormat::Yaml => {
            info!("Generating YAML output.");
            let wrapped = HashMap::from([("drives", drives)]);
            let yaml_value = serde_yaml::to_value(&wrapped).unwrap();
            serde_yaml::to_string(&yaml_value).unwrap()
        }
    };

    match args.output_file {
        Some(filename) => {
            info!("Writing output to file '{}'.", filename.to_string_lossy());
            fs::write(filename, output_str).expect("Writing output file should succeed.");
        }
        None => {
            debug!("{}", output_str);
        }
    }

    exitcode::OK
}
