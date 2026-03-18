use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::error::Result;

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Sync {
        config: "skills.config.yaml".into(),
        dry_run: false,
        quiet: false,
        json: false,
        plain: false,
        verbose: false,
    }) {
        Commands::Sync {
            config,
            dry_run,
            quiet,
            json,
            plain,
            verbose,
        } => crate::commands::sync::run(&config, dry_run, quiet, json, plain, verbose),
        Commands::List { json } => crate::commands::list::run(json),
        Commands::Doctor { json } => crate::commands::doctor::run(json),
    }
}
