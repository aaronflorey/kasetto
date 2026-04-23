# Cookbook

**When you need this:** You want copy-paste setups for common real workflows (teams, monorepos, multiple agents, pinned rollouts).

**What you'll learn:**

- Patterns that work well in practice
- How to pin and roll out changes safely

## Team Bootstrap From A URL Config

Host a shared `kasetto.yaml` somewhere reachable over HTTPS (public or private), then have each developer run:

```bash
kst sync --config https://example.com/team/kasetto.yaml
```

For private configs hosted on git providers, set the matching token env var (see [Authentication](./authentication.md)).

## Monorepo: Project Scope Per Workspace

Keep one `kasetto.yaml` per workspace folder and make it project-scoped:

```yaml
scope: project
agent: cursor

skills:
  - source: https://github.com/acme/monorepo-skills
    skills:
      - code-reviewer
      - doc-coauthoring
```

Then run sync from each workspace directory:

```bash
kst sync
```

Each workspace gets its own `./kasetto.lock`.

## Multiple Agents From One Config

Install the same skills (and MCP servers) into multiple agents:

```yaml
agent:
  - claude-code
  - cursor
  - codex

skills:
  - source: https://github.com/acme/skills
    skills: "*"

mcps:
  - source: https://github.com/acme/mcp-packs
```

## Shared Team Presets With Repo Overrides

Put reusable team defaults in the global config, then include them from each repo:

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

Kasetto expands `include_presets` before the repo-local `skills` list, so teams can keep a shared baseline and still add per-repo skills.

## MCP Packs: Pinning And Rollouts

Pin an MCP pack source to a git tag or commit SHA:

```yaml
agent: claude-code

skills:
  - source: https://github.com/acme/skills
    skills: "*"

mcps:
  - source: https://github.com/acme/mcp-packs
    ref: v2.4.1
```

Roll forward by bumping `ref`, then use `--dry-run` to preview:

```bash
kst sync --dry-run
```

## Explicit MCP Pack Path (`mcps.path`)

If a repo contains multiple MCP config files or doesn't match the default discovery layout, point directly at one:

```yaml
mcps:
  - source: https://github.com/acme/monorepo
    path: mcps/servers/pack.json
```
