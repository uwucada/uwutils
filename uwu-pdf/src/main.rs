use clap::{Parser, Subcommand};
use log::{debug, error, info, trace, warn};
use pretty_env_logger;
use std::path::PathBuf;

mod pdf_ops;
mod pdf_post_parse_sec_checks;
mod pdf_pre_parse_sec_checks;

#[derive(Parser)]
#[command(name = "uwu-pdf")]
#[command(about = "🌸 「simple and cute pdf utilities」 🌸")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Extract {
        #[arg(short = 'i', long, value_name = "FILE")]
        input_file: PathBuf,
        #[arg(short = 'o', long, value_name = "DIR")]
        output_dir: Option<PathBuf>,
    },
    Analyze {
        #[arg(short = 'i', long, value_name = "FILE")]
        input_file: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract {
            input_file,
            output_dir,
        } => {
            let output_path = output_dir.unwrap_or_else(|| {
                let mut path = input_file.clone();
                path.set_extension("");
                path
            });

            pdf_ops::extract_pdf(&input_file, &output_path);
        }
        Commands::Analyze { input_file } => {
            info!("Analyzing PDF: {}", input_file.display());
            pdf_ops::analyze_pdf(&input_file)?;
        }
    }

    Ok(())
}
