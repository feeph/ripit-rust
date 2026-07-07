/*
    ripit-cli scan <SOURCE>
*/

use std::{fs, path::Path, path::PathBuf};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use clap::{Parser, ValueHint};
use itertools::Itertools;
use serde::Serialize;
use std::collections::BTreeSet;

use ripit::{ScanResult, scan_disc};

#[derive(clap::ValueEnum, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum OutputFormat {
    #[default]
    Summary,
    Yaml,
}

#[derive(Parser, Debug)]
pub struct CmdArgs {
    /// directory, filename, device name, or drive letter
    #[arg(value_hint = ValueHint::FilePath)]
    source: String,

    /// Select output format
    #[arg(short = 'f', long = "output-format", default_value_t, value_enum)]
    output_format: OutputFormat,

    /// Write output to a file
    #[arg(short = 'o', long = "output-file", value_hint = ValueHint::FilePath)]
    output_file: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

pub fn run(args: CmdArgs, makemkvcon_bin: &Path) -> i32 {
    if args.verbose {
        println!("[verbose] scan {}", args.source);
    }

    let scan_result = match scan_disc(makemkvcon_bin, &args.source) {
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

    // generate output
    let output = match args.output_format {
        OutputFormat::Summary => generate_text(&scan_result),
        OutputFormat::Yaml => generate_yaml(scan_result.parsed),
    };

    // present output
    if let Some(output_file) = args.output_file {
        match fs::write(&output_file, output) {
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
        println!("{}", output);
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

fn generate_text(scan_result: &ScanResult) -> String {
    let mut lines = Vec::<String>::new();
    let mut total_size = 0;

    // TODO is there a way to avoid the repeated clone() calls?
    match scan_result.parsed.clone() {
        Some(cr) => {
            lines.push(format!("name:   {}", cr.info.name.unwrap_or_default()));
            lines.push(format!(
                "volume: {}",
                cr.info.volume_name.unwrap_or_default()
            ));

            lines.push("-".repeat(80));
            for (tid, tr) in cr.titles {
                let filename = tr.info.output_file_name.clone().unwrap_or_default();
                let duration = tr.info.duration.clone().unwrap_or_default();
                lines.push(format!("{}: {} ({})", tid, filename, duration));

                let mut audio_tracks = BTreeSet::<String>::new();
                for (_, a_stream) in tr.streams.audio.clone() {
                    if a_stream.lang_name != "<unknown>" {
                        audio_tracks.insert(a_stream.lang_name);
                    }
                    if a_stream.metadata_language_name != "<unknown>" {
                        audio_tracks.insert(a_stream.metadata_language_name);
                    }
                }
                lines.push(format!("  audio:     {}", audio_tracks.iter().join(", ")));

                let mut subtitles = BTreeSet::<String>::new();
                for (_, s_stream) in tr.streams.subtitles.clone() {
                    if s_stream.lang_name != "<unknown>" {
                        subtitles.insert(s_stream.lang_name);
                    }
                    if s_stream.metadata_language_name != "<unknown>" {
                        subtitles.insert(s_stream.metadata_language_name);
                    }
                }
                lines.push(format!("  subtitles: {}", subtitles.iter().join(", ")));

                // aggregate all file sizes
                // (it is possible for total size to drastically exceed the medium's size)
                total_size += tr.info.disk_size_bytes;
            }
        }
        None => {
            lines.push("<None>".to_string());
        }
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
