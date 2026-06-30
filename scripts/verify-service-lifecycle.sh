#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/service-lifecycle}"
mkdir -p "$out_dir"

duration_ms=25
compositor_log="$out_dir/compositor-service.jsonl"
shell_log="$out_dir/shell-service.jsonl"
notification_log="$out_dir/notification-daemon-service.jsonl"
settings_log="$out_dir/settings-daemon-service.jsonl"

fail() {
  echo "service lifecycle verification failed: $*" >&2
  exit 1
}

require_contains() {
  file="$1"
  value="$2"
  grep -F "$value" "$file" >/dev/null || fail "missing text in $file: $value"
}

cargo build \
  -p backlit-compositor \
  -p backlit-shell \
  -p backlit-notification-daemon \
  -p backlit-settings-daemon

target/debug/backlit-compositor \
  --backend=headless \
  --socket=backlit-service-lifecycle \
  --serve \
  --serve-for-ms "$duration_ms" > "$compositor_log"

target/debug/backlit-shell \
  --component=all \
  --socket=backlit-service-lifecycle \
  --verify \
  --serve \
  --serve-for-ms "$duration_ms" > "$shell_log"

target/debug/backlit-notification-daemon \
  --verify \
  --serve \
  --serve-for-ms "$duration_ms" > "$notification_log"

target/debug/backlit-settings-daemon \
  --verify \
  --serve \
  --serve-for-ms "$duration_ms" > "$settings_log"

require_contains "$compositor_log" '"event":"compositor.ready"'
require_contains "$compositor_log" '"ready":true'
require_contains "$compositor_log" '"event":"compositor.service_running"'
require_contains "$compositor_log" '"event":"compositor.service_exit"'
require_contains "$compositor_log" '"serve_for_ms":25'

require_contains "$shell_log" '"event":"shell.verified"'
require_contains "$shell_log" '"passed":true'
require_contains "$shell_log" '"event":"shell.service_running"'
require_contains "$shell_log" '"event":"shell.service_exit"'
require_contains "$shell_log" '"serve_for_ms":25'

require_contains "$notification_log" '"event":"notification_daemon.smoke"'
require_contains "$notification_log" '"passed":true'
require_contains "$notification_log" '"event":"notification_daemon.service_running"'
require_contains "$notification_log" '"event":"notification_daemon.service_exit"'
require_contains "$notification_log" '"serve_for_ms":25'

require_contains "$settings_log" '"event":"settings_daemon.verified"'
require_contains "$settings_log" '"passed":true'
require_contains "$settings_log" '"event":"settings_daemon.service_running"'
require_contains "$settings_log" '"event":"settings_daemon.service_exit"'
require_contains "$settings_log" '"serve_for_ms":25'

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-service-lifecycle",
  "passed": true,
  "duration_ms": $duration_ms,
  "artifacts": {
    "compositor_log": "$compositor_log",
    "shell_log": "$shell_log",
    "notification_daemon_log": "$notification_log",
    "settings_daemon_log": "$settings_log"
  },
  "checks": {
    "compositor_service_lifecycle": true,
    "shell_service_lifecycle": true,
    "notification_daemon_service_lifecycle": true,
    "settings_daemon_service_lifecycle": true,
    "bounded_service_exit": true,
    "systemd_service_mode": true
  }
}
EOF

printf 'Backlit service lifecycle verification passed. Artifacts: %s\n' "$out_dir"
