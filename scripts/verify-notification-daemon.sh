#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/notification-daemon}"
mkdir -p "$out_dir"

notification_log="$out_dir/notification-daemon.jsonl"

cargo run -p backlit-notification-daemon -- --verify > "$notification_log"

grep '"event":"notification_daemon.smoke"' "$notification_log" >/dev/null
grep '"passed":true' "$notification_log" >/dev/null
grep '"notify_calls":3' "$notification_log" >/dev/null
grep '"active_after_replace":1' "$notification_log" >/dev/null
grep '"replacement_preserved_id":true' "$notification_log" >/dev/null
grep '"action_invoked":true' "$notification_log" >/dev/null
grep '"closed_replaced":true' "$notification_log" >/dev/null
grep '"closed_expired":true' "$notification_log" >/dev/null
grep '"closed_dismissed":true' "$notification_log" >/dev/null
grep '"critical_persistent":true' "$notification_log" >/dev/null
grep '"spec_fields_valid":true' "$notification_log" >/dev/null
grep '"active_after_cleanup":0' "$notification_log" >/dev/null

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-notification-daemon",
  "passed": true,
  "artifacts": {
    "notification_log": "$notification_log"
  },
  "checks": {
    "notify_calls": 3,
    "replace_id": true,
    "action_invoked": true,
    "close_reasons": true,
    "critical_persistent": true,
    "dbus_spec_fields": true,
    "active_after_cleanup": 0
  }
}
EOF

printf 'Backlit notification daemon verification passed. Artifacts: %s\n' "$out_dir"
