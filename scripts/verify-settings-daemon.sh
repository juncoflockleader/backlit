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
grep '"power_action_commands_complete":true' "$settings_log" >/dev/null
grep '"power_action_commands":5' "$settings_log" >/dev/null
grep '"power_actions_dry_run":true' "$settings_log" >/dev/null
grep '"disruptive_power_actions_guarded":true' "$settings_log" >/dev/null
grep '"lock_action_ready":true' "$settings_log" >/dev/null
grep '"logout_action_ready":true' "$settings_log" >/dev/null
grep '"suspend_action_ready":true' "$settings_log" >/dev/null
grep '"reboot_action_ready":true' "$settings_log" >/dev/null
grep '"shutdown_action_ready":true' "$settings_log" >/dev/null
grep '"logout_requires_session_id":true' "$settings_log" >/dev/null
grep '"lock_command":"loginctl lock-session"' "$settings_log" >/dev/null
grep '"logout_command":"loginctl terminate-session $XDG_SESSION_ID"' "$settings_log" >/dev/null
grep '"suspend_command":"systemctl suspend"' "$settings_log" >/dev/null
grep '"reboot_command":"systemctl reboot"' "$settings_log" >/dev/null
grep '"shutdown_command":"systemctl poweroff"' "$settings_log" >/dev/null
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
    "power_action_commands_complete": true,
    "power_action_commands": 5,
    "power_actions_dry_run": true,
    "disruptive_power_actions_guarded": true,
    "lock_action_ready": true,
    "logout_action_ready": true,
    "suspend_action_ready": true,
    "reboot_action_ready": true,
    "shutdown_action_ready": true,
    "logout_requires_session_id": true,
    "lock_command": "loginctl lock-session",
    "logout_command": "loginctl terminate-session \$XDG_SESSION_ID",
    "suspend_command": "systemctl suspend",
    "reboot_command": "systemctl reboot",
    "shutdown_command": "systemctl poweroff",
    "state_generation": 3
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit settings daemon verification passed. Artifacts: %s\n' "$out_dir"
