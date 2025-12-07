use colored::Colorize;
use lofty::prelude::*;
use log::{debug, error, warn};
use std::path::PathBuf;

pub fn read_and_display_tags(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    debug!("loading tags from file: {}", path.display());

    let tagged_file = lofty::read_from_path(path).map_err(|e| {
        error!("failed to read audio file: {}", e);
        e
    })?;

    let tag = match tagged_file.primary_tag() {
        Some(t) => t,
        None => match tagged_file.first_tag() {
            Some(t) => t,
            None => {
                warn!("no tags found in file");
                return Ok(());
            }
        },
    };

    let items: Vec<_> = tag.items().collect();

    if items.is_empty() {
        warn!("no tag items found in file");
        return Ok(());
    }

    println!(
        "{} {}",
        "「found」".green().bold(),
        format!("{} tag(s)", items.len()).cyan()
    );
    println!();

    for item in items {
        let key = item.key();
        debug!("processing tag: {:?}", key);

        println!(
            "{} {}",
            "「tag」".cyan().bold(),
            format!("{:?}", key).yellow()
        );
        println!("  {}: {:?}", "Content".green(), item.value());
    }

    Ok(())
}
