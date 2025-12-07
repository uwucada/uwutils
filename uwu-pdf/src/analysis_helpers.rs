use colored::Colorize;
use log::{debug, trace};
use lopdf::Object;
use std::collections::HashMap;

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

pub fn count_object_types(object: &Object, stats: &mut PdfStats) {
    match object {
        Object::Boolean(_) => {
            stats.booleans += 1;
            trace!("found boolean object");
        }
        Object::Integer(_) => {
            stats.integers += 1;
            trace!("found integer object");
        }
        Object::Real(_) => {
            stats.reals += 1;
            trace!("found real object");
        }
        Object::Name(_) => {
            stats.names += 1;
            trace!("found name object");
        }
        Object::String(_, _) => {
            stats.strings += 1;
            trace!("found string object");
        }
        Object::Array(_) => {
            stats.arrays += 1;
            trace!("found array object");
        }
        Object::Reference(_) => {
            stats.references += 1;
            trace!("found reference object");
        }
        Object::Null => {
            stats.nulls += 1;
            trace!("found null object");
        }

        Object::Stream(stream) => {
            stats.streams += 1;
            trace!("found stream object");

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
                    debug!("found image stream");

                    if let Ok(Object::Name(cs)) = dict.get(b"ColorSpace") {
                        let cs_name = String::from_utf8_lossy(cs).to_string();
                        trace!("Image color space: {}", cs_name);
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
            trace!("found dictionary object");
            if let Ok(Object::Name(type_name)) = dict.get(b"Type") {
                match type_name.as_slice() {
                    b"Font" => {
                        stats.fonts += 1;
                        debug!("found font dictionary");
                    }
                    b"Annot" => {
                        stats.annotations += 1;
                        debug!("found annotation dictionary");
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn print_pdf_stats(stats: PdfStats) {
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
