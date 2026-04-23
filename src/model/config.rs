use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{err, Result};

use super::{Agent, AgentField};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Scope {
    #[default]
    #[serde(rename = "global")]
    Global,
    #[serde(rename = "project")]
    Project,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub destination: Option<String>,
    #[serde(default)]
    pub scope: Option<Scope>,
    #[serde(default)]
    pub agent: Option<AgentField>,
    #[serde(default)]
    pub presets: Vec<PresetDefinition>,
    #[serde(default)]
    pub include_presets: Vec<String>,
    #[serde(default)]
    pub skills: Vec<SourceSpec>,
    #[serde(default)]
    pub mcps: Vec<McpSourceSpec>,
}

impl Config {
    pub(crate) fn agents(&self) -> Vec<Agent> {
        match &self.agent {
            Some(AgentField::One(a)) => vec![*a],
            Some(AgentField::Many(v)) => v.clone(),
            None => vec![],
        }
    }

    pub(crate) fn resolved_scope(&self) -> Scope {
        self.scope.unwrap_or_default()
    }

    pub(crate) fn expand_included_presets(
        &mut self,
        global_presets: &[PresetDefinition],
    ) -> Result<()> {
        if self.include_presets.is_empty() {
            return Ok(());
        }

        let mut available = preset_map(global_presets)?;
        for preset in &self.presets {
            if available.insert(preset.name.clone(), preset).is_some() {
                return Err(err(format!("duplicate preset definition: {}", preset.name)));
            }
        }

        let mut expanded = Vec::new();
        for preset_name in &self.include_presets {
            let preset = available.get(preset_name).ok_or_else(|| {
                err(format!(
                    "preset not found: {preset_name} (define it in the repo config or global config)"
                ))
            })?;
            expanded.extend(preset.skills.clone());
        }

        expanded.extend(std::mem::take(&mut self.skills));
        self.skills = expanded;
        Ok(())
    }
}

fn preset_map<'a>(
    presets: &'a [PresetDefinition],
) -> Result<HashMap<String, &'a PresetDefinition>> {
    let mut available = HashMap::new();
    for preset in presets {
        if available.insert(preset.name.clone(), preset).is_some() {
            return Err(err(format!("duplicate preset definition: {}", preset.name)));
        }
    }
    Ok(available)
}

/// Resolve the effective scope: CLI override > config YAML `scope:` field > Global default.
///
/// When a `Config` is already loaded, pass it directly. Otherwise the function
/// reads the default config path (local `kasetto.yaml`, then global XDG config)
/// as a fallback.
pub(crate) fn resolve_scope(cli_override: Option<Scope>, cfg: Option<&Config>) -> Scope {
    if let Some(s) = cli_override {
        return s;
    }
    if let Some(cfg) = cfg {
        return cfg.resolved_scope();
    }
    if let Ok(text) = std::fs::read_to_string(crate::default_config_path()) {
        if let Ok(cfg) = serde_yaml::from_str::<Config>(&text) {
            return cfg.resolved_scope();
        }
    }
    Scope::Global
}

#[cfg(test)]
mod tests {
    use super::{resolve_scope, Config, Scope};

    #[test]
    fn resolve_scope_prefers_cli_override() {
        assert_eq!(resolve_scope(Some(Scope::Project), None), Scope::Project);
        assert_eq!(resolve_scope(Some(Scope::Global), None), Scope::Global);
    }

    #[test]
    fn expand_included_presets_prepends_preset_skills() {
        let global_yaml = r#"
presets:
  - name: team-core
    skills:
      - source: https://github.com/example/team
        skills: "*"
"#;
        let local_yaml = r#"
include_presets:
  - team-core
skills:
  - source: ~/repo-skills
    skills: "*"
"#;

        let global: Config = serde_yaml::from_str(global_yaml).expect("parse global config");
        let mut local: Config = serde_yaml::from_str(local_yaml).expect("parse local config");

        local
            .expand_included_presets(&global.presets)
            .expect("expand presets");

        assert_eq!(local.skills.len(), 2);
        assert_eq!(local.skills[0].source, "https://github.com/example/team");
        assert_eq!(local.skills[1].source, "~/repo-skills");
    }

    #[test]
    fn expand_included_presets_errors_for_missing_preset() {
        let mut cfg: Config = serde_yaml::from_str("include_presets:\n  - missing\nskills: []\n")
            .expect("parse config");

        let err = cfg
            .expand_included_presets(&[])
            .expect_err("missing preset should fail");

        assert!(err.to_string().contains("preset not found: missing"));
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct SourceSpec {
    pub source: String,
    pub branch: Option<String>,
    /// Pin to a git tag, commit SHA, or any ref. Takes priority over `branch`.
    /// When set, no main/master fallback is attempted.
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    /// Optional subdirectory inside the source repo/path to use as the skill root.
    /// Supports both `sub-dir` and `sub_dir` YAML keys.
    #[serde(default, rename = "sub-dir", alias = "sub_dir")]
    pub sub_dir: Option<String>,
    pub skills: SkillsField,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct PresetDefinition {
    pub name: String,
    pub skills: Vec<SourceSpec>,
}

/// What the user specified to identify a version of the source.
pub(crate) enum GitPin {
    /// Explicit ref (tag, SHA, etc.) -- no fallback.
    Ref(String),
    /// Explicit branch name -- no fallback.
    Branch(String),
    /// Nothing specified -- try "main", fall back to "master".
    Default,
}

impl SourceSpec {
    /// Resolve the effective git pin: `ref` > `branch` > default.
    pub(crate) fn git_pin(&self) -> GitPin {
        if let Some(r) = &self.git_ref {
            GitPin::Ref(r.clone())
        } else if let Some(b) = &self.branch {
            GitPin::Branch(b.clone())
        } else {
            GitPin::Default
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct McpSourceSpec {
    pub source: String,
    pub branch: Option<String>,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    /// Explicit path to an MCP JSON file within the repo (e.g. `.mcp.json`).
    /// When set, skips auto-discovery and uses this file directly.
    pub path: Option<String>,
}

impl McpSourceSpec {
    pub(crate) fn as_source_spec(&self) -> SourceSpec {
        SourceSpec {
            source: self.source.clone(),
            branch: self.branch.clone(),
            git_ref: self.git_ref.clone(),
            sub_dir: None,
            skills: SkillsField::Wildcard("*".to_string()),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum SkillsField {
    Wildcard(String),
    List(Vec<SkillTarget>),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum SkillTarget {
    Name(String),
    Obj { name: String, path: Option<String> },
}
