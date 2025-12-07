use colored::Colorize;
use log::{info, warn};
use lopdf::Object;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::pdf_post_parse_sec_checks;
use crate::pdf_pre_parse_sec_checks;
use crate::pdf_pre_parse_sec_checks::PreParseResults;

#[derive(Debug, Default)]
pub struct PdfStats {
    pub object_count: usize,
    pub page_count: usize,
    pub images: usize,
    pub fonts: usize,
    pub streams: usize,
    pub dictionaries: usize,
    pub arrays: usize,
    pub strings: usize,
    pub names: usize,
    pub integers: usize,
    pub reals: usize,
    pub booleans: usize,
    pub nulls: usize,
    pub references: usize,
    pub annotations: usize,
    pub form_xobjects: usize,
    pub filter_types: HashMap<String, usize>,
    pub color_spaces: HashMap<String, usize>,
}
/**
#[derive(Debug, Default)]
pub struct PdfRepairInfo {
    pub prepended_bytes: Option<usize>,
    pub appended_bytes: Option<usize>,
}
 */

/// load pdf, check for issues and try repair
///
/// we'll look for things like appended and prepended data
/// and report these as possible security issues since
/// data could be hidden here
pub fn repair_and_load_pdf(
    file_path: &PathBuf,
) -> Result<(lopdf::Document, PreParseResults), lopdf::Error> {
    use log::{error, info, warn};

    let mut repair_info = PreParseResults::default();

    let mut pdf_bytes = std::fs::read(file_path).map_err(|e| {
        error!("Could not read file: {}", e);
        lopdf::Error::IO(e)
    })?;

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

    // if pdf is valid at all it should load now
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

/// prints PDF object counts
///
/// this just prints the counts of the various object types,
/// it's kinda intended to show you if something is worth
/// digging into before you invest the time
///
/// probably not super reliable but it's a decent start?
pub fn analyze_pdf(file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let (doc, _) = repair_and_load_pdf(file_path)?;

    let mut stats = PdfStats::default();
    stats.object_count = doc.objects.len();
    stats.page_count = doc.get_pages().len();

    for (_object_id, object) in doc.objects.iter() {
        count_object_types(object, &mut stats);
    }

    pdf_post_parse_sec_checks::post_parse_sec_checks(&doc);

    print_pdf_stats(stats);

    Ok(())
}

pub fn extract_pdf(input_file: &PathBuf, output_dir: &PathBuf) {
    println!(
        "extracting {} to {}",
        input_file.display(),
        output_dir.display()
    )
}

fn count_object_types(object: &Object, stats: &mut PdfStats) {
    match object {
        Object::Boolean(_) => stats.booleans += 1,
        Object::Integer(_) => stats.integers += 1,
        Object::Real(_) => stats.reals += 1,
        Object::Name(_) => stats.names += 1,
        Object::String(_, _) => stats.strings += 1,
        Object::Array(_) => stats.arrays += 1,
        Object::Reference(_) => stats.references += 1,
        Object::Null => stats.nulls += 1,

        Object::Stream(stream) => {
            stats.streams += 1;

            let dict = &stream.dict;

            if let Ok(filter) = dict.get(b"Filter") {
                let filter_name = match filter {
                    Object::Name(name) => Some(String::from_utf8_lossy(name).to_string()),
                    Object::Array(arr) if !arr.is_empty() => {
                        if let Object::Name(name) = &arr[0] {
                            Some(String::from_utf8_lossy(name).to_string())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(name) = filter_name {
                    *stats.filter_types.entry(name).or_insert(0) += 1;
                }
            }

            if let Ok(Object::Name(subtype)) = dict.get(b"Subtype") {
                if subtype == b"Image" {
                    stats.images += 1;

                    if let Ok(Object::Name(cs)) = dict.get(b"ColorSpace") {
                        let cs_name = String::from_utf8_lossy(cs).to_string();
                        *stats.color_spaces.entry(cs_name).or_insert(0) += 1;
                    }
                }
            }

            if let Ok(Object::Name(type_name)) = dict.get(b"Type") {
                if type_name == b"XObject" {
                    if let Ok(Object::Name(subtype)) = dict.get(b"Subtype") {
                        if subtype == b"Form" {
                            stats.form_xobjects += 1;
                        }
                    }
                }
            }
        }

        Object::Dictionary(dict) => {
            stats.dictionaries += 1;
            if let Ok(Object::Name(type_name)) = dict.get(b"Type") {
                match type_name.as_slice() {
                    b"Font" => stats.fonts += 1,
                    b"Annot" => stats.annotations += 1,
                    _ => {}
                }
            }
        }
    }
}

/// prints pdf stats
///
/// this really doesn't need to be factored out but i think it
/// looks ugly in the main analysis function so i'm hiding it
/// down here where nobody will ever find it
fn print_pdf_stats(stats: PdfStats) {
    println!("{}", "「pdf stats」".cyan().bold());
    println!("  {}: {}", "Pages".green(), stats.page_count);
    println!("  {}: {}", "Total Objects".green(), stats.object_count);
    println!("  {}: {}", "Images".green(), stats.images);
    println!("  {}: {}", "Fonts".green(), stats.fonts);
    println!("  {}: {}", "Streams".green(), stats.streams);
    println!("  {}: {}", "Dictionaries".green(), stats.dictionaries);
    println!("  {}: {}", "Arrays".green(), stats.arrays);
    println!("  {}: {}", "Strings".green(), stats.strings);
    println!("  {}: {}", "Names".green(), stats.names);
    println!("  {}: {}", "Integers".green(), stats.integers);
    println!("  {}: {}", "Reals".green(), stats.reals);
    println!("  {}: {}", "Booleans".green(), stats.booleans);
    println!("  {}: {}", "Nulls".green(), stats.nulls);
    println!("  {}: {}", "References".green(), stats.references);
    println!("  {}: {}", "Annotations".green(), stats.annotations);
    println!("  {}: {}", "Form XObjects".green(), stats.form_xobjects);

    if !stats.filter_types.is_empty() {
        println!("  {}:", "Filter Types".green());
        for (filter, count) in &stats.filter_types {
            println!("    {}: {}", filter.cyan(), count);
        }
    }

    if !stats.color_spaces.is_empty() {
        println!("  {}:", "Color Spaces".green());
        for (cs, count) in &stats.color_spaces {
            println!("    {}: {}", cs.cyan(), count);
        }
    }
}
