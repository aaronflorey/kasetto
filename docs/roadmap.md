# Roadmap

Planned features for future releases:

- **Agents management** — list detected agents, show their install paths, and check whether agent binaries are present on the system
- **Hooks management** — run user-defined commands before or after sync (e.g., reload an agent, run a linter, send a notification)
- **Audit command** — scan config and installed MCP servers for security issues (unpinned sources, suspicious commands, overly broad permissions)
- **Smart URL rewriting** — auto-rewrite GitHub `/blob/` URLs to raw content URLs so `--config` works with copy-pasted browser URLs

Have an idea? [Open an issue](https://github.com/pivoshenko/kasetto/issues) — community suggestions shape what gets built next.
