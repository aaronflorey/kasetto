use std::collections::BTreeSet;
use std::io::{self, IsTerminal, Write};

use crate::banner::print_banner;
use crate::colors::{ACCENT, RESET, SUCCESS, WARNING};
use crate::commands::add::{
    config_path_for_edit, load_or_default_config, normalize_source, save_config,
    skill_names_from_field, source_matches,
};
use crate::error::{err, Result};
use crate::model::{Config, SkillTarget, SkillsField};
use crate::ui::SYM_OK;

pub(crate) fn run(
    repo: &str,
    skill_names: &[String],
    global: bool,
    unattended: bool,
) -> Result<()> {
    print_banner();
    println!();

    let config_path = config_path_for_edit(global)?;
    let source = normalize_source(repo)?;
    let mut cfg = load_or_default_config(&config_path)?;
    let plan = build_removal_plan(&cfg, &source, skill_names)?;

    if let RemovalPlan::Noop { summary } = &plan {
        println!("{WARNING}!{RESET} {summary}");
        return Ok(());
    }

    print_removal_preview(&config_path, &source, &plan);
    if !unattended && !confirm_removal()? {
        println!("{WARNING}!{RESET} remove cancelled");
        return Ok(());
    }

    let summary = apply_removal_plan(&mut cfg, plan);
    save_config(&config_path, &cfg)?;

    println!();
    println!(
        "{SUCCESS}{SYM_OK}{RESET} Updated {ACCENT}{}{RESET}",
        config_path.display()
    );
    println!("  {summary}");
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RemovalPlan {
    Noop {
        summary: String,
    },
    RemoveSource {
        index: usize,
    },
    RemoveSkills {
        index: usize,
        removed: Vec<String>,
        remaining: Vec<String>,
        missing: Vec<String>,
    },
}

fn build_removal_plan(cfg: &Config, source: &str, skill_names: &[String]) -> Result<RemovalPlan> {
    let Some((index, entry)) = cfg
        .skills
        .iter()
        .enumerate()
        .find(|(_, entry)| source_matches(&entry.source, source))
    else {
        return Ok(RemovalPlan::Noop {
            summary: format!("no source matched {source}"),
        });
    };

    if skill_names.is_empty() {
        return Ok(RemovalPlan::RemoveSource { index });
    }

    if matches!(entry.skills, SkillsField::Wildcard(ref wildcard) if wildcard == "*") {
        return Err(err(format!(
            "cannot remove specific skills from {source} because it is configured as skills: \"*\""
        )));
    }

    let existing = skill_names_from_field(&entry.skills);
    let requested = dedupe_skill_names(skill_names);
    let removed = requested
        .intersection(&existing)
        .cloned()
        .collect::<Vec<_>>();
    let missing = requested.difference(&existing).cloned().collect::<Vec<_>>();

    if removed.is_empty() {
        return Ok(RemovalPlan::Noop {
            summary: format!(
                "no changes for {source}; requested skills were not present{}",
                format_missing_suffix(&missing)
            ),
        });
    }

    let remaining = existing.difference(&requested).cloned().collect::<Vec<_>>();
    if remaining.is_empty() {
        return Ok(RemovalPlan::RemoveSource { index });
    }

    Ok(RemovalPlan::RemoveSkills {
        index,
        removed,
        remaining,
        missing,
    })
}

fn dedupe_skill_names(skill_names: &[String]) -> BTreeSet<String> {
    skill_names.iter().cloned().collect()
}

fn print_removal_preview(config_path: &std::path::Path, source: &str, plan: &RemovalPlan) {
    println!("Config: {}", config_path.display());
    println!("Source: {source}");

    match plan {
        RemovalPlan::Noop { summary } => println!("Result: {summary}"),
        RemovalPlan::RemoveSource { .. } => {
            println!("Change: remove source entry");
            println!("Result: source will be deleted");
        }
        RemovalPlan::RemoveSkills {
            removed,
            remaining,
            missing,
            ..
        } => {
            println!("Change: remove skills [{}]", removed.join(", "));
            if !missing.is_empty() {
                println!("Ignored: requested but absent [{}]", missing.join(", "));
            }
            println!("Result: source will keep [{}]", remaining.join(", "));
        }
    }
}

fn confirm_removal() -> Result<bool> {
    if !io::stdin().is_terminal() {
        return Err(err(
            "remove needs confirmation but stdin is not a terminal; pass -u to continue non-interactively",
        ));
    }

    print!("{ACCENT}Proceed?{RESET} [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes"))
}

fn apply_removal_plan(cfg: &mut Config, plan: RemovalPlan) -> String {
    match plan {
        RemovalPlan::Noop { summary } => summary,
        RemovalPlan::RemoveSource { index } => {
            let removed = cfg.skills.remove(index);
            format!("removed source {}", removed.source)
        }
        RemovalPlan::RemoveSkills {
            index,
            removed,
            remaining,
            missing,
        } => {
            let source = cfg.skills[index].source.clone();
            cfg.skills[index].skills =
                SkillsField::List(remaining.iter().cloned().map(SkillTarget::Name).collect());

            if missing.is_empty() {
                format!(
                    "removed {count} skill{suffix} from {source}",
                    count = removed.len(),
                    suffix = if removed.len() == 1 { "" } else { "s" }
                )
            } else {
                format!(
                    "removed {count} skill{suffix} from {source}; ignored absent [{missing}]",
                    count = removed.len(),
                    suffix = if removed.len() == 1 { "" } else { "s" },
                    missing = missing.join(", ")
                )
            }
        }
    }
}

fn format_missing_suffix(missing: &[String]) -> String {
    if missing.is_empty() {
        String::new()
    } else {
        format!(" [{}]", missing.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SourceSpec;

    fn config_with_skills(source: &str, skills: SkillsField) -> Config {
        Config {
            skills: vec![SourceSpec {
                source: source.into(),
                branch: None,
                git_ref: None,
                sub_dir: None,
                skills,
            }],
            ..Config::default()
        }
    }

    #[test]
    fn build_removal_plan_removes_entire_source_without_skill_filter() {
        let cfg = config_with_skills(
            "https://github.com/org/repo",
            SkillsField::List(vec![SkillTarget::Name("alpha".into())]),
        );

        let plan = build_removal_plan(&cfg, "https://github.com/org/repo", &[]).expect("plan");
        assert!(matches!(plan, RemovalPlan::RemoveSource { index: 0 }));
    }

    #[test]
    fn build_removal_plan_removes_selected_skills() {
        let cfg = config_with_skills(
            "https://github.com/org/repo",
            SkillsField::List(vec![
                SkillTarget::Name("alpha".into()),
                SkillTarget::Name("beta".into()),
            ]),
        );

        let plan = build_removal_plan(&cfg, "https://github.com/org/repo", &["alpha".into()])
            .expect("plan");

        assert!(matches!(
            plan,
            RemovalPlan::RemoveSkills {
                removed,
                remaining,
                missing,
                ..
            } if removed == vec!["alpha"] && remaining == vec!["beta"] && missing.is_empty()
        ));
    }

    #[test]
    fn build_removal_plan_removes_source_when_last_skill_is_removed() {
        let cfg = config_with_skills(
            "https://github.com/org/repo",
            SkillsField::List(vec![SkillTarget::Name("alpha".into())]),
        );

        let plan = build_removal_plan(&cfg, "https://github.com/org/repo", &["alpha".into()])
            .expect("plan");
        assert!(matches!(plan, RemovalPlan::RemoveSource { .. }));
    }

    #[test]
    fn build_removal_plan_rejects_selective_remove_from_wildcard() {
        let cfg = config_with_skills(
            "https://github.com/org/repo",
            SkillsField::Wildcard("*".into()),
        );

        let err = build_removal_plan(&cfg, "https://github.com/org/repo", &["alpha".into()])
            .expect_err("error");
        assert!(err.to_string().contains("skills: \"*\""));
    }

    #[test]
    fn build_removal_plan_matches_normalized_existing_source() {
        let cfg = config_with_skills(
            "http://github.com/org/repo.git/",
            SkillsField::List(vec![SkillTarget::Name("alpha".into())]),
        );

        let plan = build_removal_plan(&cfg, "https://github.com/org/repo", &[]).expect("plan");
        assert!(matches!(plan, RemovalPlan::RemoveSource { .. }));
    }

    #[test]
    fn build_removal_plan_noops_when_source_is_missing() {
        let cfg = Config::default();

        let plan = build_removal_plan(&cfg, "https://github.com/org/repo", &[]).expect("plan");
        assert!(matches!(plan, RemovalPlan::Noop { .. }));
    }

    #[test]
    fn build_removal_plan_noops_when_requested_skills_are_absent() {
        let cfg = config_with_skills(
            "https://github.com/org/repo",
            SkillsField::List(vec![SkillTarget::Name("alpha".into())]),
        );

        let plan = build_removal_plan(&cfg, "https://github.com/org/repo", &["beta".into()])
            .expect("plan");
        assert!(matches!(plan, RemovalPlan::Noop { summary } if summary.contains("not present")));
    }

    #[test]
    fn apply_removal_plan_updates_remaining_skills() {
        let mut cfg = config_with_skills(
            "https://github.com/org/repo",
            SkillsField::List(vec![
                SkillTarget::Name("alpha".into()),
                SkillTarget::Name("beta".into()),
            ]),
        );

        let summary = apply_removal_plan(
            &mut cfg,
            RemovalPlan::RemoveSkills {
                index: 0,
                removed: vec!["alpha".into()],
                remaining: vec!["beta".into()],
                missing: vec![],
            },
        );

        assert!(summary.contains("removed 1 skill"));
        assert!(matches!(&cfg.skills[0].skills, SkillsField::List(items) if items.len() == 1));
    }
}
