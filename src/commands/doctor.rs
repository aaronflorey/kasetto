use std::io::IsTerminal;
use std::path::Path;

use crate::banner::print_banner;
use crate::error::Result;
use crate::fsops::{load_latest_failed_installs, load_state, manifest_db_path};
use crate::model::FailedInstall;
use crate::profile::{format_updated_ago, list_color_enabled};

#[derive(serde::Serialize)]
struct DoctorOutput {
    version: String,
    manifest_db: String,
    installation_path: String,
    last_sync: Option<String>,
    failed_skills: Vec<FailedInstall>,
}

pub fn run(as_json: bool) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let manifest_path = manifest_db_path()?;
    let state = load_state()?;
    let mut install_paths: Vec<String> = state
        .skills
        .values()
        .map(|entry| {
            let p = Path::new(&entry.destination);
            p.parent().unwrap_or(p).to_string_lossy().to_string()
        })
        .collect();
    install_paths.sort();
    install_paths.dedup();
    let installation_path = if install_paths.is_empty() {
        "none".to_string()
    } else if install_paths.len() == 1 {
        install_paths[0].clone()
    } else {
        install_paths.join(", ")
    };

    let failed_skills = load_latest_failed_installs()?;
    let last_sync = state.last_run.clone();
    let output = DoctorOutput {
        version,
        manifest_db: manifest_path.to_string_lossy().to_string(),
        installation_path,
        last_sync,
        failed_skills,
    };

    if as_json {
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let color = list_color_enabled();
    if std::io::stdout().is_terminal() {
        if color {
            print_banner();
        } else {
            println!("kasetto | カセット");
        }
        println!();
    }
    let last_sync_text = match &output.last_sync {
        Some(ts) => format!("{} ({})", format_updated_ago(ts), ts),
        None => "none".to_string(),
    };

    if color {
        println!("\x1b[1;35mVersion:\x1b[0m {}", output.version);
        println!("\x1b[1;35mManifest DB:\x1b[0m {}", output.manifest_db);
        println!(
            "\x1b[1;35mInstallation Path:\x1b[0m {}",
            output.installation_path
        );
        println!("\x1b[1;35mLast Sync:\x1b[0m {}", last_sync_text);
        println!("\x1b[1;35mFailed Skills:\x1b[0m");
        if output.failed_skills.is_empty() {
            println!("none");
        } else {
            for failed in &output.failed_skills {
                println!(
                    "\x1b[1;33m{}\x1b[0m {} \x1b[90m{}\x1b[0m",
                    failed.skill, failed.reason, failed.source
                );
            }
        }
    } else {
        println!("Version: {}", output.version);
        println!("Manifest DB: {}", output.manifest_db);
        println!("Installation Path: {}", output.installation_path);
        println!("Last Sync: {}", last_sync_text);
        println!("Failed Skills:");
        if output.failed_skills.is_empty() {
            println!("none");
        } else {
            for failed in &output.failed_skills {
                println!("{} {} {}", failed.skill, failed.reason, failed.source);
            }
        }
    }

    Ok(())
}
