mod analyzer;
mod frame;
mod repair;

use anyhow::Result;
use clap::Parser;
use log::info;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "uwu-mp3c")]
#[command(about = "🌸 「mp3 corruption analyzer and repair tool」 🌸")]
struct Cli {
    #[arg(short = 'i', long, value_name = "FILE")]
    input: PathBuf,

    #[arg(
        short = 'e',
        long,
        value_name = "DIR",
        num_args = 0..=1,
        default_missing_value = "",
        help = "Extract and repair MP3. Optionally specify output directory."
    )]
    extract: Option<String>,
}

fn main() -> Result<()> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("symphonia", log::LevelFilter::Error)
        .parse_default_env()
        .init();

    let cli = Cli::parse();

    match cli.extract {
        None => {
            info!("analyzing mp3 file: {}", cli.input.display());
            analyzer::analyze(&cli.input)?;
        }
        Some(extract_path) => {
            info!("repairing mp3 file: {}", cli.input.display());
            repair::repair(&cli.input, &extract_path)?;
        }
    }

    Ok(())
}
