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

    /// Refresh routes using source data and apply only route differences.
    Update,

    /// Restore routes from saved state.
    Restore,

    /// Automatically restore routes during startup.
    AutoRestore,

    /// Install Windows service for startup route restoration.
    InstallService,

    /// Remove installed Windows service.
    RemoveService,

    /// Internal entry point for Windows Service Manager.
    #[command(hide = true)]
    Service,
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

    match cli.subcommand {
        Subcommand::Export(ExportArgs { platform }) => {
            let source = chnroutes::Source::from_str(&cli.source)
                .map_err(|err| format!("Invalid source: {err}"))?;

            log::info!("Using source: {}", source.as_str());

            export(platform.as_deref(), &source)?;
        }

        Subcommand::Up => {
            let source = chnroutes::Source::from_str(&cli.source)
                .map_err(|err| format!("Invalid source: {err}"))?;

            log::info!("Using source: {}", source.as_str());

            chnroutes::up(&source).await?;
        }

        Subcommand::Down => {
            let source = chnroutes::Source::from_str(&cli.source)
                .map_err(|err| format!("Invalid source: {err}"))?;

            log::info!("Using source: {}", source.as_str());

            chnroutes::down(&source).await?;
        }

        Subcommand::Update => {
            let source = chnroutes::Source::from_str(&cli.source)
                .map_err(|err| format!("Invalid source: {err}"))?;

            log::info!("Using source: {}", source.as_str());

            chnroutes::update(&source).await?;
        }

        Subcommand::Restore => {
            chnroutes::restore().await?;
        }

        Subcommand::AutoRestore => {
            chnroutes::auto_restore().await?;
        }

        #[cfg(windows)]
        Subcommand::InstallService => {
            chnroutes::service::win::install()?;
        }

        #[cfg(windows)]
        Subcommand::RemoveService => {
            chnroutes::service::win::remove()?;
        }

        #[cfg(windows)]
        Subcommand::Service => {
            chnroutes::service::win::run()?;
        }

        #[cfg(not(windows))]
        Subcommand::InstallService | Subcommand::RemoveService | Subcommand::Service => {
            return Err("Windows service management is only supported on Windows.".into());
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
