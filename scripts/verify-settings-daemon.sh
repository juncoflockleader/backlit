#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/settings-daemon}"
mkdir -p "$out_dir"

settings_log="$out_dir/settings-daemon.jsonl"

cargo run -p backlit-settings-daemon -- --verify > "$settings_log"

grep '"event":"settings_daemon.verified"' "$settings_log" >/dev/null
grep '"passed":true' "$settings_log" >/dev/null
grep '"display_validated":true' "$settings_log" >/dev/null
grep '"input_validated":true' "$settings_log" >/dev/null
grep '"power_validated":true' "$settings_log" >/dev/null
grep '"invalid_display_rejected":true' "$settings_log" >/dev/null
grep '"invalid_input_rejected":true' "$settings_log" >/dev/null
grep '"invalid_power_rejected":true' "$settings_log" >/dev/null
grep '"power_menu_complete":true' "$settings_log" >/dev/null
grep '"power_menu_actions":4' "$settings_log" >/dev/null
grep '"state_generation":3' "$settings_log" >/dev/null

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-settings-daemon",
  "passed": true,
  "artifacts": {
    "settings_log": "$settings_log"
  },
  "checks": {
    "display_validated": true,
    "input_validated": true,
    "power_validated": true,
    "invalid_display_rejected": true,
    "invalid_input_rejected": true,
    "invalid_power_rejected": true,
    "power_menu_complete": true,
    "power_menu_actions": 4,
    "state_generation": 3
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit settings daemon verification passed. Artifacts: %s\n' "$out_dir"
