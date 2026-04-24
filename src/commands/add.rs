use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::banner::print_banner;
use crate::colors::{ACCENT, RESET, SUCCESS};
use crate::error::{err, Result};
use crate::fsops::dirs_kasetto_config;
use crate::model::{Config, SkillTarget, SkillsField, SourceSpec};
use crate::source::{materialize_source, normalize_repo_url};
use crate::ui::SYM_OK;
use crate::DEFAULT_CONFIG_FILENAME;

pub(crate) fn run(repo: &str, skill_names: &[String], global: bool) -> Result<()> {
    print_banner();
    println!();

    let config_path = config_path_for_edit(global)?;
    let repo = normalize_source(repo)?;
    let requested_skills = validate_and_collect_skills(&repo, skill_names)?;
    let mut cfg = load_or_default_config(&config_path)?;

    let summary = upsert_skill_source(&mut cfg, &repo, requested_skills, skill_names.is_empty());
    save_config(&config_path, &cfg)?;

    println!(
        "{SUCCESS}{SYM_OK}{RESET} Updated {ACCENT}{}{RESET}",
        config_path.display()
    );
    println!("  {}", summary);
    Ok(())
}

pub(crate) fn config_path_for_edit(global: bool) -> Result<PathBuf> {
    if global {
        return Ok(dirs_kasetto_config()?.join(DEFAULT_CONFIG_FILENAME));
    }
    Ok(PathBuf::from(DEFAULT_CONFIG_FILENAME))
}

pub(crate) fn normalize_source(source: &str) -> Result<String> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(err("repo must not be empty"));
    }
    if trimmed.contains("://") {
        return normalize_repo_url(trimmed);
    }
    if let Some(expanded) = expand_github_shorthand(trimmed) {
        if !Path::new(trimmed).exists() {
            return Ok(expanded);
        }
    }
    Ok(trimmed.to_string())
}

fn expand_github_shorthand(source: &str) -> Option<String> {
    if source.starts_with('.')
        || source.starts_with('/')
        || source.starts_with('~')
        || source.contains('\\')
    {
        return None;
    }

    let segments = source.split('/').collect::<Vec<_>>();
    if segments.len() != 2 || segments.iter().any(|segment| segment.is_empty()) {
        return None;
    }

    if !segments.iter().all(|segment| {
        segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    }) {
        return None;
    }

    normalize_repo_url(&format!("https://github.com/{source}")).ok()
}

fn validate_and_collect_skills(source: &str, skill_names: &[String]) -> Result<Vec<String>> {
    let available = discover_available_skills(source)?;

    if skill_names.is_empty() {
        if available.is_empty() {
            return Err(err(format!("no skills found in source: {source}")));
        }
        return Ok(Vec::new());
    }

    let mut requested = BTreeSet::new();
    for skill in skill_names {
        if !available
            .iter()
            .any(|available_skill| available_skill == skill)
        {
            return Err(err(format!("skill not found in source {source}: {skill}")));
        }
        requested.insert(skill.clone());
    }

    Ok(requested.into_iter().collect())
}

pub(crate) fn discover_available_skills(source: &str) -> Result<Vec<String>> {
    let src = SourceSpec {
        source: source.to_string(),
        branch: None,
        git_ref: None,
        skills: SkillsField::Wildcard("*".to_string()),
    };
    let stage = std::env::temp_dir().join(format!(
        "kasetto-add-{}-{}",
        std::process::id(),
        crate::fsops::now_unix()
    ));
    let materialized = materialize_source(&src, Path::new("."), &stage)?;
    let mut available = materialized.available.keys().cloned().collect::<Vec<_>>();
    available.sort();

    if let Some(cleanup_dir) = materialized.cleanup_dir {
        let _ = fs::remove_dir_all(cleanup_dir);
    }

    Ok(available)
}

pub(crate) fn load_or_default_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&text)?)
}

pub(crate) fn save_config(path: &Path, cfg: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let yaml = serde_yaml::to_string(cfg)?;
    fs::write(path, yaml)?;
    Ok(())
}

fn upsert_skill_source(
    cfg: &mut Config,
    source: &str,
    requested_skills: Vec<String>,
    use_wildcard: bool,
) -> String {
    if let Some(existing) = cfg
        .skills
        .iter_mut()
        .find(|entry| source_matches(&entry.source, source))
    {
        existing.source = source.to_string();
        if use_wildcard {
            existing.skills = SkillsField::Wildcard("*".to_string());
            return format!("set {source} to sync all skills");
        }

        if matches!(existing.skills, SkillsField::Wildcard(ref wildcard) if wildcard == "*") {
            return format!("kept {source} syncing all skills");
        }

        let mut merged = skill_names_from_field(&existing.skills);
        let before = merged.len();
        merged.extend(requested_skills);
        existing.skills = SkillsField::List(merged.into_iter().map(SkillTarget::Name).collect());

        let added = skill_names_from_field(&existing.skills)
            .len()
            .saturating_sub(before);
        return if added == 0 {
            format!("no changes for {source}; requested skills were already present")
        } else {
            format!(
                "added {added} skill{} to existing source {source}",
                pluralize(added)
            )
        };
    }

    cfg.skills.push(SourceSpec {
        source: source.to_string(),
        branch: None,
        git_ref: None,
        skills: if use_wildcard {
            SkillsField::Wildcard("*".to_string())
        } else {
            SkillsField::List(
                requested_skills
                    .into_iter()
                    .map(SkillTarget::Name)
                    .collect(),
            )
        },
    });

    if use_wildcard {
        format!("added new source {source} with all discovered skills")
    } else {
        let count = match cfg.skills.last() {
            Some(SourceSpec {
                skills: SkillsField::List(items),
                ..
            }) => items.len(),
            _ => 0,
        };
        format!(
            "added new source {source} with {count} selected skill{}",
            pluralize(count)
        )
    }
}

pub(crate) fn skill_names_from_field(skills: &SkillsField) -> BTreeSet<String> {
    match skills {
        SkillsField::Wildcard(_) => BTreeSet::new(),
        SkillsField::List(items) => items
            .iter()
            .map(|item| match item {
                SkillTarget::Name(name) => name.clone(),
                SkillTarget::Obj { name, .. } => name.clone(),
            })
            .collect(),
    }
}

fn pluralize(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

pub(crate) fn source_matches(existing: &str, target: &str) -> bool {
    normalize_source(existing)
        .map(|normalized| normalized == target)
        .unwrap_or_else(|_| existing.trim() == target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SkillsField;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn normalize_source_standardizes_repo_urls() {
        let normalized = normalize_source(" http://github.com/org/repo.git/ ").expect("norm");
        assert_eq!(normalized, "https://github.com/org/repo");
    }

    #[test]
    fn normalize_source_expands_github_shorthand() {
        let dir = temp_dir("kasetto-add-github-shorthand");
        fs::create_dir_all(&dir).expect("mkdir");

        let original = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&dir).expect("chdir");
        let normalized = normalize_source("org/repo").expect("norm");
        std::env::set_current_dir(original).expect("restore cwd");

        assert_eq!(normalized, "https://github.com/org/repo");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn normalize_source_keeps_existing_local_path() {
        let dir = temp_dir("kasetto-add-local-source");
        let local = dir.join("org");
        let nested = local.join("repo");
        fs::create_dir_all(&nested).expect("mkdirs");

        let original = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&dir).expect("chdir");
        let normalized = normalize_source("org/repo").expect("norm");
        std::env::set_current_dir(original).expect("restore cwd");

        assert_eq!(normalized, "org/repo");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_or_default_config_returns_empty_when_missing() {
        let path = temp_dir("kasetto-add-missing").join("kasetto.yaml");
        let cfg = load_or_default_config(&path).expect("config");
        assert!(cfg.skills.is_empty());
    }

    #[test]
    fn upsert_skill_source_appends_new_source() {
        let mut cfg = Config::default();

        let summary = upsert_skill_source(
            &mut cfg,
            "https://github.com/org/repo",
            vec!["alpha".into(), "beta".into()],
            false,
        );

        assert!(summary.contains("added new source"));
        assert_eq!(cfg.skills.len(), 1);
        assert!(matches!(&cfg.skills[0].skills, SkillsField::List(items) if items.len() == 2));
    }

    #[test]
    fn upsert_skill_source_merges_without_duplicates() {
        let mut cfg = Config {
            skills: vec![SourceSpec {
                source: "https://github.com/org/repo".into(),
                branch: None,
                git_ref: None,
                skills: SkillsField::List(vec![SkillTarget::Name("alpha".into())]),
            }],
            ..Config::default()
        };

        let summary = upsert_skill_source(
            &mut cfg,
            "https://github.com/org/repo",
            vec!["beta".into(), "alpha".into()],
            false,
        );

        assert!(summary.contains("added 1 skill"));
        assert!(matches!(&cfg.skills[0].skills, SkillsField::List(items) if items.len() == 2));
    }

    #[test]
    fn upsert_skill_source_promotes_existing_source_to_wildcard() {
        let mut cfg = Config {
            skills: vec![SourceSpec {
                source: "https://github.com/org/repo".into(),
                branch: None,
                git_ref: None,
                skills: SkillsField::List(vec![SkillTarget::Name("alpha".into())]),
            }],
            ..Config::default()
        };

        let summary = upsert_skill_source(&mut cfg, "https://github.com/org/repo", vec![], true);

        assert!(summary.contains("sync all skills"));
        assert!(
            matches!(cfg.skills[0].skills, SkillsField::Wildcard(ref wildcard) if wildcard == "*")
        );
    }

    #[test]
    fn upsert_skill_source_keeps_existing_wildcard() {
        let mut cfg = Config {
            skills: vec![SourceSpec {
                source: "https://github.com/org/repo".into(),
                branch: None,
                git_ref: None,
                skills: SkillsField::Wildcard("*".into()),
            }],
            ..Config::default()
        };

        let summary = upsert_skill_source(
            &mut cfg,
            "https://github.com/org/repo",
            vec!["alpha".into()],
            false,
        );

        assert!(summary.contains("syncing all skills"));
        assert!(
            matches!(cfg.skills[0].skills, SkillsField::Wildcard(ref wildcard) if wildcard == "*")
        );
    }

    #[test]
    fn save_config_creates_parent_directories() {
        let dir = temp_dir("kasetto-add-save");
        let path = dir.join("nested").join("kasetto.yaml");
        let cfg = Config::default();

        save_config(&path, &cfg).expect("save");

        assert!(path.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn upsert_skill_source_matches_existing_unnormalized_repo_url() {
        let mut cfg = Config {
            skills: vec![SourceSpec {
                source: "http://github.com/org/repo.git/".into(),
                branch: None,
                git_ref: None,
                skills: SkillsField::List(vec![SkillTarget::Name("alpha".into())]),
            }],
            ..Config::default()
        };

        let summary = upsert_skill_source(
            &mut cfg,
            "https://github.com/org/repo",
            vec!["beta".into()],
            false,
        );

        assert!(summary.contains("existing source"));
        assert_eq!(cfg.skills.len(), 1);
        assert_eq!(cfg.skills[0].source, "https://github.com/org/repo");
        assert!(matches!(&cfg.skills[0].skills, SkillsField::List(items) if items.len() == 2));
    }
}
