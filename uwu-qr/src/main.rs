mod clipboard;
mod image_io;
mod qr_decoder;

use clap::Parser;
use log::info;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "uwu-qr")]
#[command(about = "🌸 「simple and cute qr code reader」 🌸")]
struct Cli {
    // either a path to a file or not, if not we'll get from clipboard
    #[arg(short = 'i', long, value_name = "FILE")]
    input: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Warn)
        .parse_default_env()
        .init();

    let cli = Cli::parse();

    let img = match cli.input {
        Some(path) => {
            info!("reading QR code from file: {}", path.display());
            image_io::read_image_from_file(&path)?
        }
        None => {
            info!("reading QR code from clipboard");
            clipboard::read_image_from_clipboard()?
        }
    };

    qr_decoder::decode_qr_codes(&img)?;

    Ok(())
}
