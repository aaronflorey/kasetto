const FEATURES = [
  {
    title: "DECLARATIVE",
    desc: "One YAML file replaces every manual setup script you have ever written. Define your skills, MCP servers, and agents once — then forget about them. Your config becomes the single source of truth across every machine and every teammate.",
  },
  {
    title: "ENTERPRISE & PRIVATE REPOS",
    desc: "Works with GitHub, GitLab, Bitbucket, Codeberg, Gitea, and self-hosted instances out of the box. Onboard new engineers in one command. Everyone gets the exact same environment — zero drift, zero surprises.",
  },
  {
    title: "MULTI-AGENT",
    desc: "Ship to 21 agents at once — Claude Code, Cursor, Codex, Windsurf, Copilot, and beyond. Stop maintaining separate configs for each tool. One sync, every agent updated, every time.",
  },
  {
    title: "SKILLS & MCP",
    desc: "Any directory with a SKILL.md is a skill — no registry, no boilerplate. MCP server configs are auto-merged into every supported format. Distribute rules and tools as easily as sharing a repo link.",
  },
  {
    title: "SPEED",
    desc: "Built in Rust for instant startup. SHA-256 hashing and lock file diffing mean only what changed gets touched. Full sync across all 21 agents finishes in seconds, not coffee breaks.",
  },
  {
    title: "UNIVERSAL",
    desc: "One static binary — macOS, Linux, Windows. Drop it into CI pipelines with --json output and proper exit codes. Same behavior on a laptop, a Docker container, or a GitHub Actions runner.",
  },
];

type Token = { t: string; v?: string };

const CONFIG_LINES: Token[] = [
  { t: "key", v: "agent" },
  { t: "punct", v: ":" },
  { t: "nl" },
  { t: "dash", v: "  " },
  { t: "punct", v: "- " },
  { t: "str", v: "claude-code" },
  { t: "nl" },
  { t: "dash", v: "  " },
  { t: "punct", v: "- " },
  { t: "str", v: "cursor" },
  { t: "nl" },
  { t: "dash", v: "  " },
  { t: "punct", v: "- " },
  { t: "str", v: "opencode" },
  { t: "nl" },
  { t: "nl" },
  { t: "key", v: "scope" },
  { t: "punct", v: ": " },
  { t: "str", v: "project" },
  { t: "cmt", v: " # or global" },
  { t: "nl" },
  { t: "nl" },
  { t: "key", v: "skills" },
  { t: "punct", v: ":" },
  { t: "nl" },

  // Source 1: wildcard — sync all skills from repo
  { t: "dash", v: "  " },
  { t: "punct", v: "- " },
  { t: "key", v: "source" },
  { t: "punct", v: ": " },
  { t: "url", v: "github.com/acme/frontend-pack" },
  { t: "nl" },
  { t: "dash", v: "    " },
  { t: "key", v: "ref" },
  { t: "punct", v: ": " },
  { t: "str", v: "v2.1.0" },
  { t: "cmt", v: " # pin to tag or commit" },
  { t: "nl" },
  { t: "dash", v: "    " },
  { t: "key", v: "skills" },
  { t: "punct", v: ": " },
  { t: "str", v: '"*"' },
  { t: "cmt", v: " # all skills in repo" },
  { t: "nl" },
  { t: "nl" },

  // Source 2: named skills — pick specific ones
  { t: "dash", v: "  " },
  { t: "punct", v: "- " },
  { t: "key", v: "source" },
  { t: "punct", v: ": " },
  { t: "url", v: "gitlab.com/team/internal-tools" },
  { t: "nl" },
  { t: "dash", v: "    " },
  { t: "key", v: "branch" },
  { t: "punct", v: ": " },
  { t: "str", v: "main" },
  { t: "nl" },
  { t: "dash", v: "    " },
  { t: "key", v: "skills" },
  { t: "punct", v: ":" },
  { t: "cmt", v: " # pick by name" },
  { t: "nl" },
  { t: "dash", v: "      " },
  { t: "punct", v: "- " },
  { t: "str", v: "react-patterns" },
  { t: "nl" },
  { t: "dash", v: "      " },
  { t: "punct", v: "- " },
  { t: "str", v: "go-standards" },
  { t: "nl" },
  { t: "nl" },

  // Source 3: skill with custom path
  { t: "dash", v: "  " },
  { t: "punct", v: "- " },
  { t: "key", v: "source" },
  { t: "punct", v: ": " },
  { t: "url", v: "codeberg.org/oss/shared" },
  { t: "nl" },
  { t: "dash", v: "    " },
  { t: "key", v: "skills" },
  { t: "punct", v: ":" },
  { t: "nl" },
  { t: "dash", v: "      " },
  { t: "punct", v: "- " },
  { t: "key", v: "name" },
  { t: "punct", v: ": " },
  { t: "str", v: "custom-lint" },
  { t: "nl" },
  { t: "dash", v: "        " },
  { t: "key", v: "path" },
  { t: "punct", v: ": " },
  { t: "str", v: "rules/custom-lint" },
  { t: "cmt", v: " # custom path to skill dir" },
  { t: "nl" },
  { t: "nl" },

  // MCP sources
  { t: "key", v: "mcps" },
  { t: "punct", v: ":" },
  { t: "nl" },
  { t: "dash", v: "  " },
  { t: "punct", v: "- " },
  { t: "key", v: "source" },
  { t: "punct", v: ": " },
  { t: "url", v: "github.com/acme/mcp-pack" },
  { t: "nl" },
  { t: "dash", v: "  " },
  { t: "punct", v: "- " },
  { t: "key", v: "source" },
  { t: "punct", v: ": " },
  { t: "url", v: "github.com/team/tools" },
  { t: "nl" },
  { t: "dash", v: "    " },
  { t: "key", v: "path" },
  { t: "punct", v: ": " },
  { t: "str", v: ".mcp.json" },
  { t: "cmt", v: " # explicit file in repo" },
];

function renderTokens(tokens: Token[]) {
  const parts: React.ReactNode[] = [];
  let key = 0;
  for (const tok of tokens) {
    if (tok.t === "nl") {
      parts.push(<br key={key++} />);
    } else {
      const cls =
        tok.t === "key"
          ? "sy-key"
          : tok.t === "str"
            ? "sy-str"
            : tok.t === "url"
              ? "sy-url"
              : tok.t === "cmt"
                ? "sy-cmt"
                : tok.t === "dash"
                  ? "sy-dash"
                  : "sy-punct";
      parts.push(
        <span key={key++} className={cls}>
          {tok.v}
        </span>,
      );
    }
  }
  return parts;
}

export function FeatureList() {
  return (
    <div className="grid-box">
      <div className="feat-grid">
        {FEATURES.map((f) => (
          <div key={f.title} className="feat-cell">
            <p className="feat-cell-title">{f.title}</p>
            <p className="feat-cell-desc">{f.desc}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

export function ConfigExample() {
  return (
    <div className="feat-code-block">
      <div className="feat-code-header">kasetto.yaml</div>
      <div className="feat-code-body">{renderTokens(CONFIG_LINES)}</div>
    </div>
  );
}
