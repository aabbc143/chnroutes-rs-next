use std::str::FromStr;

use clap::Parser;
use colored::Colorize;
use log::LevelFilter;

#[inline]
pub fn log_init() {
    #[cfg(not(debug_assertions))]
    log_init_with_default_level(LevelFilter::Info);

    #[cfg(debug_assertions)]
    log_init_with_default_level(LevelFilter::Debug);
}

#[inline]
pub fn log_init_with_default_level(level: LevelFilter) {
    _ = pretty_env_logger::formatted_builder()
        .filter_level(level)
        .format_timestamp_secs()
        .filter_module("reqwest", LevelFilter::Info)
        .parse_default_env()
        .try_init();
}

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Subcommand,

    /// Data source used to generate CN IP rules.
    #[arg(short, long, default_value = "apnic", global = true)]
    source: String,
}

#[derive(Debug, clap::Subcommand, Clone)]
pub enum Subcommand {
    /// Export route scripts for Windows, macOS, Linux, Android or OpenVPN.
    Export(ExportArgs),

    /// Write CN IP rules to the system route table.
    Up,

    /// Remove CN IP rules from the system route table.
    Down,
}

#[derive(Debug, clap::Args, Clone)]
pub struct ExportArgs {
    /// The platform of the script to export.
    #[arg(short, long)]
    platform: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log_init();

    let cli = Cli::parse();

    let source =
    chnroutes::Source::from_str(&cli.source).map_err(|err| format!("Invalid source: {err}"))?;

    log::info!("Using source: {}", source.as_str());

    match cli.subcommand {
        Subcommand::Export(ExportArgs { platform }) => {
            export(platform.as_deref(), &source)?;
        }

        Subcommand::Up => {
            chnroutes::up(&source).await?;
        }

        Subcommand::Down => {
            chnroutes::down(&source).await?;
        }
    }

    Ok(())
}

pub fn export(platform: Option<&str>, source: &chnroutes::Source) -> chnroutes::Result<()> {
    let target = chnroutes::Target::from_str(platform.unwrap_or_default());

    if let Ok(target) = target {
        target.export_file(source)?;
    } else {
        eprint!("Unknown platform. platform must be ");

        ["windows", "mac", "linux", "android", "openvpn"]
            .iter()
            .for_each(|x| eprint!("{}, ", x.green()));

        eprintln!();

        return Err(chnroutes::Error::InvalidTarget);
    }

    Ok(())
}
