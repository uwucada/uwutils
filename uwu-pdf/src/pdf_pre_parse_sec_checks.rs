use colored::Colorize;
use log::debug;

#[derive(Debug, Default)]
pub struct PreParseResults {
    pub prepended_bytes: Option<usize>,
    pub prepended_data: Option<Vec<u8>>,
    pub appended_bytes: Option<usize>,
    pub appended_data: Option<Vec<u8>>,
}

/// wrapper to run pre-parse sec checks
pub fn pre_parse_sec_checks(bytes: &[u8]) -> PreParseResults {
    debug!("running pre-parse security checks on {} bytes", bytes.len());

    let prepend_result = check_prepended_data_bytes(bytes);
    let append_result = check_appended_data_bytes(bytes);

    let results = PreParseResults {
        prepended_bytes: prepend_result.as_ref().map(|(size, _)| *size),
        prepended_data: prepend_result.map(|(_, data)| data),
        appended_bytes: append_result.as_ref().map(|(size, _)| *size),
        appended_data: append_result.map(|(_, data)| data),
    };

    print_pre_parse_warnings(&results);

    results
}

/// check for data appended after EOF header
fn check_appended_data_bytes(bytes: &[u8]) -> Option<(usize, Vec<u8>)> {
    if let Some(eof_pos) = bytes.windows(5).position(|window| window == b"%%EOF") {
        let total_len = bytes.len();
        let truncated_len = eof_pos + 5;
        if truncated_len < total_len {
            let extra_bytes = total_len - truncated_len;
            let appended_data = bytes[truncated_len..].to_vec();
            return Some((extra_bytes, appended_data));
        }
    }
    None
}

/// check for data prepended before PDF header
fn check_prepended_data_bytes(bytes: &[u8]) -> Option<(usize, Vec<u8>)> {
    if let Some(pdf_pos) = bytes.windows(5).position(|window| window == b"%PDF-") {
        if pdf_pos > 0 {
            let prepended_data = bytes[..pdf_pos].to_vec();
            return Some((pdf_pos, prepended_data));
        }
    }
    None
}

/// print pre-parse warnings, if any
fn print_pre_parse_warnings(results: &PreParseResults) {
    let mut warnings = Vec::new();

    if let Some(prepend_bytes) = results.prepended_bytes {
        warnings.push(format!(
            "{} {} bytes before PDF header",
            "「prepended data」\t".yellow().bold(),
            prepend_bytes.to_string().yellow()
        ));
    }

    if let Some(append_bytes) = results.appended_bytes {
        warnings.push(format!(
            "{} {} bytes after EOF header",
            "「appended data」\t".yellow().bold(),
            append_bytes.to_string().yellow()
        ));
    }

    if !warnings.is_empty() {
        println!("{}", "「pre-parse warnings」".yellow().bold());
        for warning in warnings {
            println!("  {}", warning);
        }
    } else {
        println!(
            "{}",
            "「pre-parse warnings」 no prepending or appending detected".green()
        )
    }
    println!();
}
