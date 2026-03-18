use std::io::IsTerminal;

use crate::error::Result;
use crate::fsops::load_state;
use crate::list::browse as browse_list;
use crate::model::InstalledSkill;
use crate::profile::{format_updated_ago, list_color_enabled, read_skill_profile};

pub fn run(as_json: bool) -> Result<()> {
    let state = load_state()?;
    if state.skills.is_empty() {
        if as_json {
            println!("[]");
            return Ok(());
        }
        println!("No installed skills.");
        return Ok(());
    }

    let mut items = Vec::new();
    for (id, entry) in &state.skills {
        let (name, fallback_description) = read_skill_profile(&entry.destination, &entry.skill);
        let description = if entry.description.trim().is_empty() {
            fallback_description
        } else {
            entry.description.clone()
        };
        let updated_ago = format_updated_ago(&entry.updated_at);
        items.push(InstalledSkill {
            id: id.clone(),
            name,
            description,
            source: entry.source.clone(),
            skill: entry.skill.clone(),
            destination: entry.destination.clone(),
            hash: entry.hash.clone(),
            source_revision: entry.source_revision.clone(),
            updated_at: entry.updated_at.clone(),
            updated_ago,
        });
    }

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    if as_json {
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }

    if std::io::stdout().is_terminal() && std::env::var_os("NO_TUI").is_none() {
        browse_list(&items)?;
    } else {
        print_list_text(&items);
    }
    Ok(())
}

fn print_list_text(items: &[InstalledSkill]) {
    let color = list_color_enabled();
    println!("Installed skills: {}", items.len());
    println!();
    for item in items {
        if color {
            println!(
                "\x1b[1;33m{}\x1b[0m  \x1b[90mupdated {} ({})\x1b[0m",
                item.name, item.updated_ago, item.updated_at
            );
        } else {
            println!(
                "{}  updated {} ({})",
                item.name, item.updated_ago, item.updated_at
            );
        }
        println!("  {}", item.description);
        println!("  source: {}", item.source);
        println!("  path: {}", item.destination);
        println!();
    }
}
