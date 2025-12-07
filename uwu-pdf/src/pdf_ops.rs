use log::{debug, error, info};
use std::fs;
use std::path::PathBuf;

use crate::analysis_helpers::{PdfStats, count_object_types, print_pdf_stats};
use crate::extraction_helpers::{
    extract_padding, extract_pdf_streams, print_extraction_header, print_extraction_summary,
};
use crate::pdf_post_parse_sec_checks;
use crate::pdf_pre_parse_sec_checks;
use crate::pdf_pre_parse_sec_checks::PreParseResults;

/// load pdf, check for issues and try repair
///
/// we'll look for things like appended and prepended data
/// and report these as possible security issues since
/// data could be hidden here
pub fn repair_and_load_pdf(
    file_path: &PathBuf,
) -> Result<(lopdf::Document, PreParseResults), lopdf::Error> {
    info!("loading and repairing PDF: {}", file_path.display());

    let mut repair_info = PreParseResults::default();

    let mut pdf_bytes = std::fs::read(file_path).map_err(|e| {
        error!("could not read file: {}", e);
        lopdf::Error::IO(e)
    })?;

    debug!("read {} bytes from PDF file", pdf_bytes.len());

    let pre_parse_results = pdf_pre_parse_sec_checks::pre_parse_sec_checks(&pdf_bytes);

    if let Some(prepend_bytes) = pre_parse_results.prepended_bytes {
        repair_info.prepended_bytes = Some(prepend_bytes);
        pdf_bytes = pdf_bytes[prepend_bytes..].to_vec();
    }

    if let Some(append_bytes) = pre_parse_results.appended_bytes {
        repair_info.appended_bytes = Some(append_bytes);

        if let Some(eof_position) = pdf_bytes.windows(5).position(|window| window == b"%%EOF") {
            pdf_bytes.truncate(eof_position + 5);
        }
    }

    match lopdf::Document::load_mem(&pdf_bytes) {
        Ok(doc) => {
            info!("pdf loaded successfully");
            Ok((doc, repair_info))
        }
        Err(e) => {
            error!("failed to load PDF: {:?}", e);
            Err(e)
        }
    }
}

/// load pdf from bytes with pre-parse results already computed
fn load_pdf_from_bytes(
    mut pdf_bytes: Vec<u8>,
    pre_parse_results: &PreParseResults,
) -> Result<lopdf::Document, lopdf::Error> {
    use log::error;

    if let Some(prepend_bytes) = pre_parse_results.prepended_bytes {
        pdf_bytes = pdf_bytes[prepend_bytes..].to_vec();
    }

    if pre_parse_results.appended_bytes.is_some() {
        if let Some(eof_position) = pdf_bytes.windows(5).position(|window| window == b"%%EOF") {
            pdf_bytes.truncate(eof_position + 5);
        }
    }

    match lopdf::Document::load_mem(&pdf_bytes) {
        Ok(doc) => {
            info!("pdf loaded successfully");
            Ok(doc)
        }
        Err(e) => {
            error!("failed to load PDF: {:?}", e);
            Err(e)
        }
    }
}

/// prints PDF object counts and does a simple security check
///
/// this just prints the counts of the various object types,
/// it's kinda intended to show you if something is worth
/// digging into before you invest the time
///
/// probably not super reliable but it's a decent start?
pub fn analyze_pdf(file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    info!("starting PDF analysis");
    let (doc, _) = repair_and_load_pdf(file_path)?;

    let mut stats = PdfStats::default();
    stats.object_count = doc.objects.len();
    stats.page_count = doc.get_pages().len();

    debug!(
        "analyzing {} objects across {} pages",
        stats.object_count, stats.page_count
    );

    for (_object_id, object) in doc.objects.iter() {
        count_object_types(object, &mut stats);
    }

    info!("running post-parse security checks");
    pdf_post_parse_sec_checks::post_parse_sec_checks(&doc);

    print_pdf_stats(stats);

    Ok(())
}

/// extracts objects from pdf
///
/// currently handles text fields, binary data, and images,
/// though implementation is likely incomplete.
pub fn extract_pdf(input_file: &PathBuf, output_dir: &PathBuf) {
    info!(
        "Starting PDF extraction: {} -> {}",
        input_file.display(),
        output_dir.display()
    );
    print_extraction_header(input_file, output_dir);

    let pdf_bytes = match fs::read(input_file) {
        Ok(bytes) => {
            debug!("Read {} bytes from input file", bytes.len());
            bytes
        }
        Err(e) => {
            error!("Failed to read input file: {}", e);
            return;
        }
    };

    let pre_parse_results = pdf_pre_parse_sec_checks::pre_parse_sec_checks(&pdf_bytes);

    if fs::create_dir_all(output_dir).is_err() {
        error!(
            "Could not create output directory: {}",
            output_dir.display()
        );
        return;
    }

    extract_padding(output_dir, &pre_parse_results);

    let doc = match load_pdf_from_bytes(pdf_bytes, &pre_parse_results) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Could not load PDF: {:?}", e);
            return;
        }
    };

    let counts = extract_pdf_streams(&doc, output_dir);
    info!(
        "Extraction complete: {} images, {} text files, {} binary files",
        counts.images, counts.text, counts.binary
    );
    print_extraction_summary(&counts, &pre_parse_results);
}
