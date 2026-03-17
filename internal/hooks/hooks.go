package hooks

import (
	"fmt"
	"os"
	"path/filepath"
)

type Params struct {
	ConfigPath      string
	TimeoutSeconds  int
	CacheTTLSeconds int
}

func Install(p Params) ([]string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return nil, err
	}

	runnerDir := filepath.Join(home, ".sukiro", "hooks")
	if err := os.MkdirAll(runnerDir, 0o755); err != nil {
		return nil, err
	}
	runnerPath := filepath.Join(runnerDir, "session-start.sh")
	if err := os.WriteFile(runnerPath, []byte(runnerScript(p.ConfigPath, p.TimeoutSeconds, p.CacheTTLSeconds)), 0o755); err != nil {
		return nil, err
	}

	targets := []string{
		filepath.Join(home, ".claude", "hooks", "session-start.sh"),
		filepath.Join(home, ".cursor", "hooks", "session-start.sh"),
	}

	installed := []string{runnerPath}
	for _, t := range targets {
		if err := os.MkdirAll(filepath.Dir(t), 0o755); err != nil {
			return nil, err
		}
		content := fmt.Sprintf("#!/usr/bin/env bash\nexec %q\n", runnerPath)
		if err := os.WriteFile(t, []byte(content), 0o755); err != nil {
			return nil, err
		}
		installed = append(installed, t)
	}

	return installed, nil
}

func runnerScript(configPath string, timeoutSeconds, ttlSeconds int) string {
	return fmt.Sprintf(`#!/usr/bin/env bash
set -euo pipefail

CONFIG=%q
LOCK_FILE="${HOME}/.sukiro/hooks/sync.lock"
STAMP_FILE="${HOME}/.sukiro/hooks/last_sync_unix"
TIMEOUT=%d
TTL=%d

mkdir -p "${HOME}/.sukiro/hooks"

# cache guard
if [[ -f "$STAMP_FILE" ]]; then
  last=$(cat "$STAMP_FILE" || echo 0)
  now=$(date +%%s)
  if (( now - last < TTL )); then
    exit 0
  fi
fi

# lock guard
exec 9>"$LOCK_FILE"
if ! flock -n 9; then
  exit 0
fi

if timeout "$TIMEOUT" sukiro sync --config "$CONFIG" --quiet; then
  date +%%s > "$STAMP_FILE"
fi
`, configPath, timeoutSeconds, ttlSeconds)
}
