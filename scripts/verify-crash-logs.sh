#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/crash-logs}"
mkdir -p "$out_dir"

supervisor_log="$out_dir/supervisor.jsonl"

fail() {
  echo "crash log verification failed: $*" >&2
  exit 1
}

require_line() {
  file="$1"
  line="$2"
  grep -Fx "$line" "$file" >/dev/null || fail "missing line in $file: $line"
}

require_contains() {
  file="$1"
  value="$2"
  grep -F "$value" "$file" >/dev/null || fail "missing text in $file: $value"
}

cargo run -p backlit-session-supervisor -- --verify > "$supervisor_log"

require_contains "$supervisor_log" '"event":"supervisor.crash_log"'
require_contains "$supervisor_log" '"role":"shell-panel"'
require_contains "$supervisor_log" '"journal_unit":"backlit-shell.service"'
require_contains "$supervisor_log" '"syslog_identifier":"backlit-shell"'
require_contains "$supervisor_log" '"role":"compositor"'
require_contains "$supervisor_log" '"journal_unit":"backlit-compositor.service"'
require_contains "$supervisor_log" '"syslog_identifier":"backlit-compositor"'
require_contains "$supervisor_log" '"recorded":true'
require_contains "$supervisor_log" '"event":"supervisor.crash_smoke"'
require_contains "$supervisor_log" '"shell_crash_isolated":true'
require_contains "$supervisor_log" '"compositor_crash_ends_session":true'
require_contains "$supervisor_log" '"crash_logs_recorded":true'
require_contains "$supervisor_log" '"journalctl_user_scope":true'
require_contains "$supervisor_log" '"shell_journal_unit":"backlit-shell.service"'
require_contains "$supervisor_log" '"compositor_journal_unit":"backlit-compositor.service"'

crash_log_events="$(grep -c '"event":"supervisor.crash_log"' "$supervisor_log" | tr -d ' ')"
test "$crash_log_events" = "2" || fail "expected 2 crash log events, got $crash_log_events"

require_line packaging/systemd/backlit-compositor.service "Environment=RUST_BACKTRACE=1"
require_line packaging/systemd/backlit-compositor.service "SyslogIdentifier=backlit-compositor"
require_line packaging/systemd/backlit-compositor.service "StandardOutput=journal"
require_line packaging/systemd/backlit-compositor.service "StandardError=journal"
require_line packaging/systemd/backlit-compositor.service "Restart=on-failure"

require_line packaging/systemd/backlit-shell.service "Environment=RUST_BACKTRACE=1"
require_line packaging/systemd/backlit-shell.service "SyslogIdentifier=backlit-shell"
require_line packaging/systemd/backlit-shell.service "StandardOutput=journal"
require_line packaging/systemd/backlit-shell.service "StandardError=journal"
require_line packaging/systemd/backlit-shell.service "Restart=on-failure"

require_line packaging/systemd/backlit-settings-daemon.service "Environment=RUST_BACKTRACE=1"
require_line packaging/systemd/backlit-settings-daemon.service "SyslogIdentifier=backlit-settings-daemon"
require_line packaging/systemd/backlit-settings-daemon.service "StandardOutput=journal"
require_line packaging/systemd/backlit-settings-daemon.service "StandardError=journal"
require_line packaging/systemd/backlit-settings-daemon.service "Restart=on-failure"

journalctl_available=false
if command -v journalctl >/dev/null 2>&1; then
  journalctl_available=true
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-crash-logs",
  "passed": true,
  "target_os": "$(uname -s)",
  "artifacts": {
    "supervisor_log": "$supervisor_log",
    "compositor_service": "packaging/systemd/backlit-compositor.service",
    "shell_service": "packaging/systemd/backlit-shell.service",
    "settings_daemon_service": "packaging/systemd/backlit-settings-daemon.service"
  },
  "checks": {
    "crash_log_events": $crash_log_events,
    "crash_logs_recorded": true,
    "journalctl_user_scope": true,
    "journalctl_available": $journalctl_available,
    "shell_crash_journal_unit": true,
    "compositor_crash_journal_unit": true,
    "systemd_journal_output": true,
    "systemd_syslog_identifiers": true,
    "rust_backtrace_enabled": true,
    "restart_policy": true
  }
}
EOF

printf 'Backlit crash log verification passed. Artifacts: %s\n' "$out_dir"
