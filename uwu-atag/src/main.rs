mod tag_reader;

use clap::Parser;
use log::info;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "uwu-atag")]
#[command(about = "🌸 「simple and cute audio tag dumper」 🌸")]
struct Cli {
    #[arg(short = 'i', long, value_name = "FILE")]
    input: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Warn)
        .parse_default_env()
        .init();

    let cli = Cli::parse();

    info!("reading tags from file: {}", cli.input.display());
    tag_reader::read_and_display_tags(&cli.input)?;

    Ok(())
}
