use colored::Colorize;
use lopdf::Object;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct SuspiciousFeatures {
    pub has_javascript: bool,
    pub has_auto_action: bool,
    pub has_open_action: bool,
    pub unreferenced_objects_count: usize,
    pub large_unreferenced_streams: Vec<(u32, usize)>,
}

/// run post-parsing security checks
pub fn post_parse_sec_checks(doc: &lopdf::Document) {
    let results = detect_suspicious_features(doc);
    print_post_parse_warnings(results);
}

/// pdf security checks
///
/// probably a fairly naive list of checks to just look for obvious pdf smells
pub fn detect_suspicious_features(doc: &lopdf::Document) -> SuspiciousFeatures {
    let mut features = SuspiciousFeatures::default();

    let mut referenced_ids = HashSet::new();

    if let Ok(catalog_id) = doc.trailer.get(b"Root") {
        if let Object::Reference(id) = catalog_id {
            referenced_ids.insert(*id);
        }
    }

    if let Ok(info_id) = doc.trailer.get(b"Info") {
        if let Object::Reference(id) = info_id {
            referenced_ids.insert(*id);
        }
    }

    if let Ok(catalog_id) = doc.trailer.get(b"Root") {
        if let Object::Reference(id) = catalog_id {
            if let Ok(catalog) = doc.get_object(*id) {
                collect_references(catalog, &mut referenced_ids, doc);
            }
        }
    }

    for (object_id, object) in doc.objects.iter() {
        if !referenced_ids.contains(object_id) {
            features.unreferenced_objects_count += 1;
            if let Object::Stream(stream) = object {
                if let Ok(content) = stream.decompressed_content() {
                    if content.len() > 1024 {
                        features
                            .large_unreferenced_streams
                            .push((object_id.0, content.len()));
                    }
                }
            }
        }

        if let Object::Dictionary(dict) = object {
            if let Ok(Object::Name(name)) = dict.get(b"S") {
                if name == b"JavaScript" {
                    features.has_javascript = true;
                }
            }

            if dict.has(b"AA") {
                features.has_auto_action = true;
            }

            if dict.has(b"OpenAction") {
                features.has_open_action = true;
            }
        }
    }

    features
}

fn collect_references(
    object: &Object,
    referenced: &mut HashSet<(u32, u16)>,
    doc: &lopdf::Document,
) {
    match object {
        Object::Reference(id) => {
            if referenced.insert(*id) {
                if let Ok(obj) = doc.get_object(*id) {
                    collect_references(obj, referenced, doc);
                }
            }
        }
        Object::Array(arr) => {
            for item in arr {
                collect_references(item, referenced, doc);
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                collect_references(value, referenced, doc);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                collect_references(value, referenced, doc);
            }
        }
        _ => {}
    }
}

/// prints parsing results
///
/// again, doesn't need to be here but i don't like to see it so
fn print_post_parse_warnings(results: SuspiciousFeatures) {
    let mut warnings = Vec::new();

    if results.has_javascript {
        warnings.push(format!(
            "{}",
            "「javascript detected」\t pdf contains JavaScript"
                .red()
                .bold()
        ));
    }

    if results.has_auto_action {
        warnings.push(format!(
            "{}",
            "「auto actions」\t pdf executes code automatically"
                .red()
                .bold()
        ));
    }

    if results.has_open_action {
        warnings.push(format!(
            "{}",
            "「open actions」\t code runs on open".red().bold()
        ));
    }

    if results.unreferenced_objects_count > 0 {
        warnings.push(format!(
            "{} {} unreferenced objects found",
            "「unreferenced objects」\t".yellow().bold(),
            results.unreferenced_objects_count.to_string().yellow()
        ));
    }

    if !results.large_unreferenced_streams.is_empty() {
        warnings.push(format!(
            "{} {} large streams not referenced by any page",
            "「hidden streams」\t".red().bold(),
            results
                .large_unreferenced_streams
                .len()
                .to_string()
                .yellow()
        ));
        for (obj_id, size) in &results.large_unreferenced_streams {
            warnings.push(format!(
                "  Object {}: {} bytes",
                obj_id.to_string().cyan(),
                size.to_string().yellow()
            ));
        }
    }

    if !warnings.is_empty() {
        println!("{}", "「post-parse warnings」".red().bold());
        for warning in warnings {
            println!("  {}", warning);
        }
        println!();
    } else {
        println!(
            "{}",
            "「post-parse warnings」 no suspicious features detected".green()
        );
        println!();
    }
}
