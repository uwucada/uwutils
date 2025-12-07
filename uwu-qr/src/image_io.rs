use image::DynamicImage;
use log::{debug, error, info};
use std::path::PathBuf;

pub fn read_image_from_file(path: &PathBuf) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    debug!("loading image from file: {}", path.display());
    let img = image::open(path).map_err(|e| {
        error!("failed to open image: {}", e);
        e
    })?;
    info!("loaded image ({}x{})", img.width(), img.height());
    Ok(img)
}
