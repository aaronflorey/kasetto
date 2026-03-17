# Contributing to Sukiro

Thanks for contributing.

## Development setup

```bash
go version
# expected: go 1.22+

go mod tidy
go build ./...
go test ./...
```

## Project structure

- `cmd/sukiro`: CLI entrypoint
- `internal/config`: config parsing
- `internal/source`: local/GitHub source resolution
- `internal/syncer`: core sync engine
- `internal/state`: persistent state
- `internal/report`: run reports
- `internal/hooks`: Claude/Cursor hook installer

## Commit style

Use conventional-ish prefixes:
- `feat:` new functionality
- `fix:` bug fixes
- `refactor:` behavior-preserving code changes
- `build:` tooling/dependency updates
- `docs:` documentation only
- `ci:` workflow changes

## Pull requests

Before opening a PR:

```bash
go test ./...
go vet ./...
go build ./...
```

Include:
- what changed
- why
- any breaking behavior

## Release

Releases are tag-driven via GitHub Actions.
Tag format: `vX.Y.Z`
