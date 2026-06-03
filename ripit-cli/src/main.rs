/*
    command line interface for ripit-cli
*/

mod cmd_drives;
mod cmd_extract;
mod cmd_scan;
mod cmd_unshackle;
mod os_utils;
mod yaml_utils;

use clap::{Parser, Subcommand};
use log::LevelFilter;

#[derive(Subcommand, Debug)]
enum Cmd {
    #[command(about = "show optical drives")]
    Drives(cmd_drives::CmdArgs),

    #[command(about = "scan the medium and show its content")]
    Scan(cmd_scan::CmdArgs),

    #[command(about = "free your beloved movies and series from their physical constraints")]
    Unshackle(cmd_unshackle::CmdArgs),

    #[command(about = "extract one or more titles from disc or image")]
    Extract(cmd_extract::CmdArgs),
}

#[derive(Parser, Debug)]
#[command(name = "ripit-cli")]
#[command(about = "A small tool to scan, extract, and unshackle media content.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Info)
        .init();

    let makemkvcon_bin = os_utils::find_makemkvcon();
    log::info!("Using binary '{}'.", makemkvcon_bin.to_string_lossy());

    let cli = Cli::parse();
    let exit_code = match cli.cmd {
        Cmd::Drives(args) => cmd_drives::run(args, &makemkvcon_bin),
        Cmd::Unshackle(args) => cmd_unshackle::run(args, &makemkvcon_bin),
        Cmd::Scan(args) => cmd_scan::run(args, &makemkvcon_bin),
        Cmd::Extract(args) => cmd_extract::run(args, &makemkvcon_bin),
    };

    std::process::exit(exit_code);
}
