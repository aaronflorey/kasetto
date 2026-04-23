# Configuration

When `--config` is omitted, Kasetto looks for config in this order:

1. `$KASETTO_CONFIG` env var
2. `source:` key in `$XDG_CONFIG_HOME/kasetto/config.yaml`
3. `./kasetto.yaml`
4. `$XDG_CONFIG_HOME/kasetto/kasetto.yaml` (or `~/.config/kasetto/kasetto.yaml`)

Point it at a specific file or URL with `--config`, or run `kst init` for local `./kasetto.yaml` (`kst init --global` writes the global config file).
To persist a remote URL as your default, add a `source:` key to `~/.config/kasetto/config.yaml`.

## Example

```yaml
# Choose an agent preset (single or multiple)...
agent: codex
# agent:
#   - claude-code
#   - cursor

# ...or set an explicit path (overrides agent)
# destination: ./my-skills

# Install scope: "global" (default) or "project"
# scope: project

# Reusable preset definitions, typically in ~/.config/kasetto/kasetto.yaml
# presets:
#   - name: team-core
#     skills:
#       - source: https://github.com/org/shared-skills
#         skills: "*"

# Include preset definitions from this file or your global config
# include_presets:
#   - team-core

skills:
  # Pull specific skills from a GitHub repo
  - source: https://github.com/org/skill-pack
    branch: main
    skills:
      - code-reviewer
      - name: design-system

  # Sync everything from a local folder
  - source: ~/Development/my-skills
    skills: "*"

  # Pin to a specific git tag
  - source: https://github.com/acme/monorepo
    ref: v1.2.0
    skills:
      - name: custom-skill
        path: tools/skills

  # Limit discovery to a nested directory inside the source
  - source: https://github.com/acme/agents
    sub-dir: plugins/swift-apple-expert
    skills: "*"

# MCP servers (optional)
mcps:
  - source: https://github.com/org/mcp-pack
  - source: https://github.com/org/monorepo
    path: mcps/my-server/pack.json
```

## Reference

### Top-Level Fields

| Key               | Required | Description                                                         |
| ----------------- | -------- | ------------------------------------------------------------------- |
| `agent`           | no       | One or more [supported agent presets](./agents.md) - string or list |
| `destination`     | no       | Explicit install path - overrides `agent` if both are set           |
| `scope`           | no       | `"global"` (default) or `"project"` - where to install              |
| `presets`         | no       | Named reusable skill-source groups, usually defined in the global config |
| `include_presets` | no       | Preset names to prepend from the current config and/or global config |
| `skills`          | **yes**  | List of skill sources                                               |
| `mcps`            | no       | List of MCP server sources                                          |

### Preset Fields

Each entry in the `presets` list defines a named group of skill sources:

| Key                | Required | Description                                           |
| ------------------ | -------- | ----------------------------------------------------- |
| `presets[].name`   | **yes**  | Preset name referenced from `include_presets`         |
| `presets[].skills` | **yes**  | Skill source list using the same shape as top-level `skills` |

### Skill Source Fields

| Key       | Required | Description                                                            |
| --------- | -------- | ---------------------------------------------------------------------- |
| `source`  | **yes**  | Git host URL or local path (GitHub, GitLab, Bitbucket, Codeberg/Gitea) |
| `branch`  | no       | Branch for remote sources (default: `main`, falls back to `master`)    |
| `ref`     | no       | Git tag, commit SHA, or ref - takes priority over `branch`             |
| `sub-dir` | no       | Relative subdirectory within the source used as the discovery root (`sub_dir` alias supported) |
| `skills`  | **yes**  | `"*"` for all, or a list of names / `{ name, path }` objects           |

### Skill Entry Fields

Each entry in the `skills` list can be a string (the skill name) or an object:

| Key    | Required | Description                                                 |
| ------ | -------- | ----------------------------------------------------------- |
| `name` | **yes**  | Name of the skill directory to install                      |
| `path` | no       | Custom subdirectory within the source to look for the skill |

### MCP Source Fields

| Key      | Required | Description                                                                 |
| -------- | -------- | --------------------------------------------------------------------------- |
| `source` | **yes**  | Git host URL or local path containing MCP server config                     |
| `branch` | no       | Branch for remote sources (default: `main`, falls back to `master`)         |
| `ref`    | no       | Git tag, commit SHA, or ref - takes priority over `branch`                  |
| `path`   | no       | Explicit path to the MCP JSON file within the source (skips auto-discovery) |

Kasetto discovers MCP config files automatically in this order:

1. `.mcp.json` at the source root
2. `mcp.json` at the source root
3. Any `.json` file inside a `mcp/` subdirectory

Use the `path` field to point at a specific file when the source contains multiple configs or uses
a non-standard layout.

MCP config files must contain a `mcpServers` object with server definitions. Servers are merged
into each agent's native settings file (e.g., `.claude.json` for Claude Code, `.cursor/mcp.json`
for Cursor). See [how sync works](./how-sync-works.md) for merge behavior details.

## Reusable Presets

Presets let you define reusable skill bundles once and include them in repo-level configs.

```yaml
# ~/.config/kasetto/kasetto.yaml
presets:
  - name: team-core
    skills:
      - source: https://github.com/acme/shared-skills
        skills:
          - code-reviewer
          - doc-coauthoring
```

```yaml
# ./kasetto.yaml
agent: claude-code

include_presets:
  - team-core

skills:
  - source: ./skills
    skills:
      - repo-helper
```

When Kasetto loads the repo config, it expands `include_presets` into the top-level `skills` list.
Included preset sources are prepended ahead of the repo's own `skills` entries.

A few important rules:

- Presets only expand into `skills`. They do not add `mcps`.
- Presets can be defined in the current config or in the global config at `~/.config/kasetto/kasetto.yaml`.
- If the same preset name is defined twice across those sources, Kasetto fails with a duplicate-preset error.
- If `include_presets` references a name that does not exist, Kasetto fails before syncing.

## Remote Configs

Kasetto can fetch configs from any HTTPS URL:

```bash
kst sync --config https://example.com/team-skills.yaml
```

Great for sharing a single config across a team without checking it into every repository.

Kasetto recognises browser URLs from GitHub, GitLab, and Gitea / Codeberg / Forgejo, and auto-rewrites them to the matching raw-content endpoint. You can paste any of these directly:

- `https://github.com/owner/repo/blob/main/kasetto.yaml`
- `https://gitlab.com/group/repo/-/blob/main/kasetto.yaml`
- `https://codeberg.org/owner/repo/src/branch/main/kasetto.yaml`

Kasetto prints a short `note: rewrote browser URL to raw content: ...` line so you can see what was fetched. Authentication is resolved against the rewritten host, so the same tokens that work for raw URLs apply here too.

If the URL points to a private repo, Kasetto uses the same token-based authentication as skill sources. See [authentication](./authentication.md) for the full list of supported environment variables.

## Multiple Agents

The `agent` field accepts a single value or a list. With a list, Kasetto installs skills to every agent's directory and merges MCP servers into every agent's settings file:

```yaml
agent:
  - claude-code
  - cursor
  - codex

skills:
  - source: https://github.com/org/skill-pack
    skills: "*"
```

Handy when you juggle multiple agents and want them all to share the same skill set.

## Agent vs Destination

If you set both, `destination` wins. Use `agent` for convenience with [supported presets](./agents.md), or `destination` when you need full control over the install path.

!!! tip

    Use `destination` when targeting an agent that isn't in the supported list.

## Scope: Global Vs Project

By default, skills are installed globally into the agent's home-directory path. Add `scope: project` to your config, or pass `--project` on the command line, to install into the current project directory instead.

The `--project` / `--global` flags always override whatever `scope` is set in the config file.

## Environment Variables

These environment variables affect Kasetto's output behavior:

| Variable   | Effect                                                                    |
| ---------- | ------------------------------------------------------------------------- |
| `NO_TUI`   | Disables interactive screens (home menu, list browser). Set to any value. |
| `NO_COLOR` | Disables colored output. Set to any value.                                |
