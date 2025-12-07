use image::DynamicImage;
use log::{debug, error, info};
use std::io::Read;

pub fn read_image_from_clipboard() -> Result<DynamicImage, Box<dyn std::error::Error>> {
    debug!("attempting to read image from clipboard using arboard");

    match try_arboard_clipboard() {
        Ok(img) => {
            info!("successfully read image from clipboard using arboard");
            return Ok(img);
        }
        Err(e) => {
            debug!("arboard failed: {}", e);

            // fallback to wl-clipboard-rs for wayland compat
            #[cfg(target_os = "linux")]
            {
                debug!("falling back to wl-clipboard-rs for wayland support");
                match try_wl_clipboard() {
                    Ok(img) => {
                        info!("successfully read image from clipboard using wl-clipboard-rs");
                        return Ok(img);
                    }
                    Err(wl_err) => {
                        error!("both arboard and wl-clipboard-rs failed");
                        debug!("arboard error: {}", e);
                        debug!("wl-clipboard error: {}", wl_err);
                    }
                }
            }

            error!("failed to read image from clipboard");
            return Err("no image found in clipboard".into());
        }
    }
}

fn try_arboard_clipboard() -> Result<DynamicImage, Box<dyn std::error::Error>> {
    use arboard::Clipboard;

    debug!("initializing arboard clipboard");
    let mut clipboard = Clipboard::new()?;
    let img_data = clipboard.get_image()?;

    debug!(
        "got image data: {}x{} pixels",
        img_data.width, img_data.height
    );

    let rgba_data = img_data.bytes.into_owned();
    let img = image::RgbaImage::from_raw(img_data.width as u32, img_data.height as u32, rgba_data)
        .ok_or("failed to create image from clipboard")?;

    info!(
        "successfully loaded image from clipboard via arboard ({}x{})",
        img.width(),
        img.height()
    );

    Ok(DynamicImage::ImageRgba8(img))
}

#[cfg(target_os = "linux")]
fn try_wl_clipboard() -> Result<DynamicImage, Box<dyn std::error::Error>> {
    use wl_clipboard_rs::paste::{ClipboardType, MimeType, Seat, get_contents};

    debug!("attempting to read image from Wayland clipboard");

    let mime_types = vec![
        MimeType::Specific("image/png"),
        MimeType::Specific("image/jpeg"),
        MimeType::Specific("image/jpg"),
        MimeType::Specific("image/bmp"),
    ];

    for mime_type in mime_types {
        debug!("trying mime type: {:?}", mime_type);
        match get_contents(ClipboardType::Regular, Seat::Unspecified, mime_type) {
            Ok((mut pipe, _)) => {
                debug!("successfully retrieved clipboard content");
                let mut buffer = Vec::new();
                pipe.read_to_end(&mut buffer)?;

                let img = image::load_from_memory(&buffer)?;

                info!(
                    "successfully loaded image from clipboard via wl-clipboard ({}x{})",
                    img.width(),
                    img.height()
                );
                return Ok(img);
            }
            Err(e) => {
                debug!("mime type {:?} not available: {}", mime_type, e);
                continue;
            }
        }
    }

    Err("no image found in Wayland clipboard".into())
}
