/*
    command line interface for ripit-cli
*/

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, warn};

mod cmd_drives;
mod cmd_extract;
// mod cmd_scan;
mod cmd_unshackle;
mod logging;
mod os_utils;
mod progress_tracker;

use clap::{Args, Parser, Subcommand};

#[derive(Subcommand, Debug)]
enum Cmd {
    #[command(about = "show optical drives")]
    Drives(cmd_drives::CmdArgs),

    // #[command(about = "scan the medium and show its content")]
    // Scan(cmd_scan::CmdArgs),
    #[command(
        about = "free your beloved movies and series from their physical constraints",
        after_help = UNSHACKLE_AFTER_HELP,
    )]
    Unshackle(cmd_unshackle::CmdArgs),

    #[command(
        about = "extract one or more titles from disc or disc image",
        after_help = EXTRACT_AFTER_HELP,
    )]
    Extract(cmd_extract::CmdArgs),
}

#[derive(Args, Debug)]
struct GlobalOpts {
    // 'RUST_LOG' would work without configuring it here, but let's be
    // user-friendly and explicitly spell out the environment variable
    #[arg(
        long = "log-level",
        env = "RUST_LOG",
        global = true,
        help = "Set logging level (debug, info, warn, error). Default is 'warn'"
    )]
    log_level: Option<log::Level>,
}

#[derive(Debug, Parser)]
#[command(name = "ripit-cli")]
#[command(about = "A small tool to scan, extract, and unshackle media content.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[tokio::main]
async fn main() {
    // TODO validate that 'makemkvcon' is present, otherwise give up and report to user
    // FIXME ripit breaks if makemkvcon can't be launched
    /*
        thread 'tokio-rt-worker' (425107) panicked at libs/makemkv/src/runner/mod.rs:286:10:
        Failed to spawn process!: Os { code: 2, kind: NotFound, message: "No such file or directory" }
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
        [2026-09-01T17:36:10Z DEBUG makemkv::runner] run_makemkvcon(): run_program() has returned.
    */
    let mm = makemkv::MakeMkv {
        binary: os_utils::find_makemkvcon(),
        config: makemkv::MakeMkvConfig {
            enable_robot: true,
            debug: false,
            directio: makemkv::DirectIO::Enabled,
            messages: makemkv::OutputType::StdOut,
            progress: makemkv::OutputType::StdOut,
            min_length: 0,
        },
    };

    // using 'wild::args_os()' to expand wildcard arguments on Windows
    // (on Linux this is a no-op since the shell already did it for us)
    // <https://docs.rs/wild/latest/wild/>
    let cli = Cli::parse_from(wild::args_os());
    let exit_code = match cli.cmd {
        // TODO consider moving all async-related logic to subcommands and make main() async-less again?
        Cmd::Drives(args) => cmd_drives::run(args, &mm).await,
        Cmd::Unshackle(args) => cmd_unshackle::run(args, &mm).await,
        // Cmd::Scan(args) => cmd_scan::run(args, &makemkvcon_bin),
        Cmd::Extract(args) => cmd_extract::run(args, &mm).await,
    };

    std::process::exit(exit_code);
}

// Using global static strings primarily because the content is sensitive
// to indentation (any leading whitespace is going to be visible in the
// command line interface's output)

static UNSHACKLE_AFTER_HELP: &str = r"common usage examples:

  create a disc image from a DVD, HD-DVD or Blu-Ray and eject it when done
  (creates a subdirectory matching the disc's name)
  -------------------------------------------------------------------------

  ripit-cli unshackle -e -d E:       -t D:\backup
  ripit-cli unshackle -e -d /dev/sr0 -t /mnt/backup
";

static EXTRACT_AFTER_HELP: &str = r"common usage examples:

  extract MKV files from an optical drive and eject the disc when done
  (creates a subdirectory matching the disc's name)
  -------------------------------------------------------------------------

  ripit-cli extract -e -d E:       -t D:\mkv
  ripit-cli extract -e -d /dev/sr0 -t /mnt/mkv

  extract MKV files from an image
  -------------------------------------------------------------------------

  ripit-cli extract -i image.iso -t D:\mkv
  ripit-cli extract -i image.iso -t /mnt/mkv
";
