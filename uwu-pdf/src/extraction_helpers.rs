use colored::Colorize;
use image::{GrayImage, ImageBuffer, RgbImage};
use log::{debug, info, trace, warn};
use lopdf::Object;
use std::fs;
use std::path::PathBuf;

use crate::pdf_pre_parse_sec_checks::PreParseResults;

pub struct ExtractionCounts {
    pub text: usize,
    pub images: usize,
    pub binary: usize,
}

pub fn print_extraction_header(input_file: &PathBuf, output_dir: &PathBuf) {
    println!(
        "{} {} {} {}",
        "「extracting」".cyan().bold(),
        input_file.display().to_string().green(),
        "to".cyan(),
        output_dir.display().to_string().green()
    );
    println!();
}

/// pull any padding added before or after boundary headers
pub fn extract_padding(output_dir: &PathBuf, pre_parse_results: &PreParseResults) {
    if let Some(ref prepended_data) = pre_parse_results.prepended_data {
        debug!(
            "Found {} bytes of prepended data before PDF header",
            prepended_data.len()
        );
        let prepend_path = output_dir.join("prepended.bin");
        if let Err(e) = fs::write(&prepend_path, prepended_data) {
            warn!("Failed to write prepended data: {}", e);
        } else {
            info!("Extracted prepended data ({} bytes)", prepended_data.len());
        }
    }

    if let Some(ref appended_data) = pre_parse_results.appended_data {
        debug!(
            "Found {} bytes of appended data after EOF marker",
            appended_data.len()
        );
        let append_path = output_dir.join("appended.bin");
        if let Err(e) = fs::write(&append_path, appended_data) {
            warn!("Failed to write appended data: {}", e);
        } else {
            info!("Extracted appended data ({} bytes)", appended_data.len());
        }
    }
}

pub fn extract_pdf_streams(doc: &lopdf::Document, output_dir: &PathBuf) -> ExtractionCounts {
    info!("Starting stream extraction from PDF");
    debug!("Total objects in PDF: {}", doc.objects.len());

    let text_dir = output_dir.join("text");
    let images_dir = output_dir.join("images");
    let binary_dir = output_dir.join("binary");

    let _ = fs::create_dir_all(&text_dir);
    let _ = fs::create_dir_all(&images_dir);
    let _ = fs::create_dir_all(&binary_dir);

    let mut counts = ExtractionCounts {
        text: 0,
        images: 0,
        binary: 0,
    };

    for (object_id, object) in doc.objects.iter() {
        if let Object::Stream(stream) = object {
            if let Ok(content) = stream.decompressed_content() {
                let dict = &stream.dict;

                if let Ok(Object::Name(subtype)) = dict.get(b"Subtype") {
                    if subtype == b"Image" {
                        extract_and_save_image(stream, object_id, &images_dir, &mut counts.images);
                        continue;
                    }
                }

                if is_text_content(&content) {
                    extract_and_save_text(&content, object_id, &text_dir, &mut counts.text);
                } else {
                    extract_and_save_binary(&content, object_id, &binary_dir, &mut counts.binary);
                }
            }
        }
    }

    counts
}

fn extract_and_save_image(
    stream: &lopdf::Stream,
    object_id: &(u32, u16),
    images_dir: &PathBuf,
    counter: &mut usize,
) {
    let (image_data, extension) = extract_image_data(stream);
    let filename = format!("image_{}_{}.{}", object_id.0, object_id.1, extension);
    let output_path = images_dir.join(&filename);

    debug!(
        "Extracting image object {}_{} as {}",
        object_id.0, object_id.1, extension
    );

    if let Err(e) = fs::write(&output_path, &image_data) {
        warn!("failed to write image {}: {}", filename, e);
    } else {
        println!(
            "  {} {} ({} bytes)",
            "「image」".green().bold(),
            filename.cyan(),
            image_data.len().to_string().yellow()
        );
        *counter += 1;
    }
}

fn extract_and_save_text(
    content: &[u8],
    object_id: &(u32, u16),
    text_dir: &PathBuf,
    counter: &mut usize,
) {
    let filename = format!("text_{}_{}.txt", object_id.0, object_id.1);
    let output_path = text_dir.join(&filename);

    debug!("Extracting text object {}_{}", object_id.0, object_id.1);

    if let Err(e) = fs::write(&output_path, content) {
        warn!("failed to write text {}: {}", filename, e);
    } else {
        println!(
            "  {} {} ({} bytes)",
            "「text」".green().bold(),
            filename.cyan(),
            content.len().to_string().yellow()
        );
        *counter += 1;
    }
}

fn extract_and_save_binary(
    content: &[u8],
    object_id: &(u32, u16),
    binary_dir: &PathBuf,
    counter: &mut usize,
) {
    let filename = format!("binary_{}_{}.bin", object_id.0, object_id.1);
    let output_path = binary_dir.join(&filename);

    debug!("Extracting binary object {}_{}", object_id.0, object_id.1);

    if let Err(e) = fs::write(&output_path, content) {
        warn!("failed to write binary {}: {}", filename, e);
    } else {
        println!(
            "  {} {} ({} bytes)",
            "「binary」".green().bold(),
            filename.cyan(),
            content.len().to_string().yellow()
        );
        *counter += 1;
    }
}

pub fn print_extraction_summary(counts: &ExtractionCounts, pre_parse_results: &PreParseResults) {
    println!();
    println!("{}", "「extraction summary」".cyan().bold());
    println!("  {} {}", "Text files:".green(), counts.text);
    println!("  {} {}", "Image files:".green(), counts.images);
    println!("  {} {}", "Binary files:".green(), counts.binary);
    if pre_parse_results.prepended_data.is_some() {
        println!("  {} {}", "Prepended data:".yellow(), "1");
    }
    if pre_parse_results.appended_data.is_some() {
        println!("  {} {}", "Appended data:".yellow(), "1");
    }
    println!();
}

fn extract_image_data(stream: &lopdf::Stream) -> (Vec<u8>, &'static str) {
    let dict = &stream.dict;

    if let Ok(filter) = dict.get(b"Filter") {
        match filter {
            Object::Name(name) => match name.as_slice() {
                b"DCTDecode" => {
                    return (stream.content.clone(), "jpg");
                }
                b"JPXDecode" => {
                    return (stream.content.clone(), "jp2");
                }
                b"JBIG2Decode" => {
                    return (stream.content.clone(), "jbig2");
                }
                _ => {}
            },
            Object::Array(arr) => {
                for item in arr {
                    if let Object::Name(name) = item {
                        match name.as_slice() {
                            b"DCTDecode" => {
                                return (stream.content.clone(), "jpg");
                            }
                            b"JPXDecode" => {
                                return (stream.content.clone(), "jp2");
                            }
                            b"JBIG2Decode" => {
                                return (stream.content.clone(), "jbig2");
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if let Ok(content) = stream.decompressed_content() {
        if let Some(png_data) = encode_raw_to_png(&content, dict) {
            return (png_data, "png");
        }
        let extension = detect_image_format(&content, dict);
        (content, extension)
    } else {
        (stream.content.clone(), "dat")
    }
}

/// re-encode raw pixels into png
fn encode_raw_to_png(raw_data: &[u8], dict: &lopdf::Dictionary) -> Option<Vec<u8>> {
    let width = dict.get(b"Width").ok()?.as_i64().ok()? as u32;
    let height = dict.get(b"Height").ok()?.as_i64().ok()? as u32;
    let bpc = dict.get(b"BitsPerComponent").ok()?.as_i64().ok()? as u8;

    trace!("Attempting PNG encoding: {}x{}, {} bpc", width, height, bpc);

    if bpc != 8 {
        debug!(
            "Skipping PNG encoding: unsupported bits per component ({})",
            bpc
        );
        return None;
    }

    let colorspace = dict.get(b"ColorSpace").ok()?;
    let colorspace_name = match colorspace {
        Object::Name(name) => name.as_slice(),
        _ => return None,
    };

    let mut png_buffer = Vec::new();

    match colorspace_name {
        b"DeviceRGB" => {
            let expected_size = (width * height * 3) as usize;
            if raw_data.len() < expected_size {
                return None;
            }

            let img: RgbImage =
                ImageBuffer::from_raw(width, height, raw_data[..expected_size].to_vec())?;
            img.write_to(
                &mut std::io::Cursor::new(&mut png_buffer),
                image::ImageFormat::Png,
            )
            .ok()?;
        }
        b"DeviceGray" => {
            let expected_size = (width * height) as usize;
            if raw_data.len() < expected_size {
                return None;
            }

            let img: GrayImage =
                ImageBuffer::from_raw(width, height, raw_data[..expected_size].to_vec())?;
            img.write_to(
                &mut std::io::Cursor::new(&mut png_buffer),
                image::ImageFormat::Png,
            )
            .ok()?;
        }
        _ => return None,
    }

    Some(png_buffer)
}

/// check common image formats
fn detect_image_format(content: &[u8], dict: &lopdf::Dictionary) -> &'static str {
    if content.len() > 8 && &content[0..8] == b"\x89PNG\r\n\x1a\n" {
        return "png";
    }

    if content.len() > 2 && &content[0..2] == b"\xff\xd8" {
        return "jpg";
    }

    if content.len() > 4 && &content[0..4] == b"GIF8" {
        return "gif";
    }

    if content.len() > 4 && ((&content[0..4] == b"II\x2a\x00") || (&content[0..4] == b"MM\x00\x2a"))
    {
        return "tiff";
    }

    if let Ok(filter) = dict.get(b"Filter") {
        match filter {
            Object::Name(name) => match name.as_slice() {
                b"DCTDecode" => return "jpg",
                b"JPXDecode" => return "jp2",
                b"JBIG2Decode" => return "jbig2",
                b"FlateDecode" | b"LZWDecode" => {
                    return "raw";
                }
                _ => {}
            },
            Object::Array(arr) => {
                if !arr.is_empty() {
                    if let Object::Name(name) = &arr[arr.len() - 1] {
                        match name.as_slice() {
                            b"DCTDecode" => return "jpg",
                            b"JPXDecode" => return "jp2",
                            b"JBIG2Decode" => return "jbig2",
                            b"FlateDecode" | b"LZWDecode" => {
                                return "raw";
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    "dat"
}

// check if extracted content is valid text
fn is_text_content(content: &[u8]) -> bool {
    if content.is_empty() {
        return false;
    }

    let sample_size = content.len().min(512);
    let sample = &content[..sample_size];

    let mut text_chars = 0;
    let mut total_chars = 0;

    for &byte in sample {
        total_chars += 1;
        if byte.is_ascii_graphic() || byte.is_ascii_whitespace() {
            text_chars += 1;
        } else if byte == 0x00 {
            return false;
        }
    }

    if total_chars == 0 {
        return false;
    }

    let text_ratio = (text_chars as f64) / (total_chars as f64);
    text_ratio > 0.75
}
