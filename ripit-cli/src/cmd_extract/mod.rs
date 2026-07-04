/*
    ripit-cli extract <SOURCE> <TARGET>
*/

use std::path::Path;

use clap::{Parser, ValueHint};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

#[derive(Parser, Debug)]
pub struct CmdArgs {
    /// A path-like source: directory, filename, device name, or drive letter
    #[arg(value_hint = ValueHint::FilePath)]
    source: String,

    /// Target directory
    #[arg(value_hint = ValueHint::DirPath)]
    target: std::path::PathBuf,

    /// Extract specific title(s)
    #[arg(short = 't', long = "title", value_name = "TITLE")]
    title: Vec<usize>,

    /// Ignore all titles shorter than x seconds (env: MIN_LENGTH)
    #[arg(short = 'l', long = "min-length", env = "MIN_LENGTH")]
    min_length: Option<usize>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

pub fn run(args: CmdArgs, makemkvcon_bin: &Path) -> i32 {
    // use provided value or default to 0
    let min_length = args.min_length.unwrap_or_default();

    let target_str = args.target.to_string_lossy();
    if args.verbose {
        info!("[verbose] extract {} -> {}", &args.source, &target_str);
    }
    info!(
        "Running extract with source='{}' target='{}'",
        &args.source, &target_str
    );

    // scan the medium and find all titles
    let scan_result = match makemkv::info(makemkvcon_bin, &args.source, min_length) {
        Some(x) => x,
        None => {
            return exitcode::DATAERR;
        }
    };

    let parsed = scan_result.parsed.unwrap();
    let titles_have = parsed.titles.clone();

    let titles_want = if !args.title.is_empty() {
        info!(
            "Selected {} titles for extraction: {:?}",
            args.title.len(),
            args.title
        );
        args.title
    } else {
        info!("Selected {} titles for extraction.", titles_have.len());
        let mut tmp: Vec<usize> = titles_have.clone().into_keys().collect();
        tmp.sort_unstable();
        tmp
    };

    let t_total = titles_want.len();

    // titles_want is a non-consecutive list of numbers if
    // specific titles have been requested for extraction
    // -> create a new collection which interleaves the
    //    current title's position with the title's id
    let title_idx = range::Range::new(1, t_total);
    let titles = title_idx.iter().zip(titles_want.iter());

    let mut exit_code = exitcode::OK;
    for (t_pos, id) in titles {
        match titles_have.get(id) {
            Some(tr) => {
                let filename = tr.info.output_file_name.clone().unwrap();
                debug!(
                    "Processing '{}' (idx: {}, {}/{})",
                    &filename, id, t_pos, t_total
                );

                if makemkv::mkv(makemkvcon_bin, &args.source, *id, &args.target, min_length) {
                    info!(
                        "Successfully extracted '{}'. ({}/{})",
                        &filename, t_pos, t_total
                    );
                } else {
                    warn!("Failed to extract '{}'! ({}/{})", &filename, t_pos, t_total);
                    exit_code = exitcode::DATAERR
                }
            }
            None => {
                warn!("Requested title '{}' does not exist.", id);
                exit_code = exitcode::DATAERR
            }
        }
    }

    exit_code
}
