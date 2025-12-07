use colored::Colorize;
use image::DynamicImage;
use log::{debug, warn};

pub fn decode_qr_codes(img: &DynamicImage) -> Result<(), Box<dyn std::error::Error>> {
    debug!("converting image to grayscale for QR detection");
    let gray_img = img.to_luma8();

    let mut prepared_img = rqrr::PreparedImage::prepare(gray_img);

    let grids = prepared_img.detect_grids();

    if grids.is_empty() {
        warn!("no QR codes found in image");
        return Ok(());
    }

    println!(
        "{} {}",
        "「found」".green().bold(),
        format!("{} qr code(s)", grids.len()).cyan()
    );
    println!();

    for (i, grid) in grids.iter().enumerate() {
        match grid.decode() {
            Ok((meta, content)) => {
                debug!(
                    "decoded QR code {}: {:?} version {:?}",
                    i + 1,
                    meta.ecc_level,
                    meta.version
                );

                println!(
                    "{} {}",
                    "「qr code」".cyan().bold(),
                    (i + 1).to_string().yellow()
                );
                println!("  {}: {:?}", "Version".green(), meta.version);
                println!("  {}: {:?}", "Error Correction".green(), meta.ecc_level);
                println!("  {}:", "Content".green());
                println!();
                println!("{}", content);
                println!();
            }
            Err(e) => {
                warn!("failed to decode QR code {}: {:?}", i + 1, e);
            }
        }
    }

    Ok(())
}
