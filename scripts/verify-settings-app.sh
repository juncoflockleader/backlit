#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/settings-app}"
mkdir -p "$out_dir"

settings_app_log="$out_dir/settings-app.jsonl"

cargo run -p backlit-settings -- --verify > "$settings_app_log"

grep '"event":"settings_app.verified"' "$settings_app_log" >/dev/null
grep '"passed":true' "$settings_app_log" >/dev/null
grep '"application_id":"org.backlit.Settings"' "$settings_app_log" >/dev/null
grep '"launcher_target_ready":true' "$settings_app_log" >/dev/null
grep '"required_panels":3' "$settings_app_log" >/dev/null
grep '"display_panel_ready":true' "$settings_app_log" >/dev/null
grep '"display_output":"Virtual-1"' "$settings_app_log" >/dev/null
grep '"display_modes":3' "$settings_app_log" >/dev/null
grep '"display_scale_options":4' "$settings_app_log" >/dev/null
grep '"display_apply_validated":true' "$settings_app_log" >/dev/null
grep '"input_panel_ready":true' "$settings_app_log" >/dev/null
grep '"keyboard_repeat_visible":true' "$settings_app_log" >/dev/null
grep '"pointer_accel_visible":true' "$settings_app_log" >/dev/null
grep '"touchpad_toggle_visible":true' "$settings_app_log" >/dev/null
grep '"input_apply_validated":true' "$settings_app_log" >/dev/null
grep '"power_panel_ready":true' "$settings_app_log" >/dev/null
grep '"power_idle_policy_visible":true' "$settings_app_log" >/dev/null
grep '"power_lid_action_visible":true' "$settings_app_log" >/dev/null
grep '"power_menu_visible":true' "$settings_app_log" >/dev/null
grep '"power_menu_actions":4' "$settings_app_log" >/dev/null
grep '"power_command_plans_available":true' "$settings_app_log" >/dev/null
grep '"power_apply_validated":true' "$settings_app_log" >/dev/null
grep '"daemon_generation":3' "$settings_app_log" >/dev/null

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-settings-app",
  "passed": true,
  "artifacts": {
    "settings_app_log": "$settings_app_log"
  },
  "checks": {
    "application_id": "org.backlit.Settings",
    "launcher_target_ready": true,
    "required_panels": 3,
    "display_panel": true,
    "display_modes": 3,
    "display_scale_options": 4,
    "display_apply_validated": true,
    "input_panel": true,
    "keyboard_repeat_visible": true,
    "pointer_accel_visible": true,
    "touchpad_toggle_visible": true,
    "input_apply_validated": true,
    "power_panel": true,
    "power_menu_actions": 4,
    "power_command_plans_available": true,
    "power_apply_validated": true,
    "daemon_generation": 3
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit settings app verification passed. Artifacts: %s\n' "$out_dir"
