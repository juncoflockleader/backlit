#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/launch-performance}"
mkdir -p "$out_dir"

session_log="$out_dir/session.jsonl"
session_stderr="$out_dir/session.stderr"
session_screenshot="$out_dir/session.ppm"
service_log_dir="$out_dir/session-services"

startup_budget_ms="${BACKLIT_STARTUP_BUDGET_MS:-500}"
shell_ready_budget_ms="${BACKLIT_SHELL_READY_BUDGET_MS:-2000}"
terminal_launch_budget_ms="${BACKLIT_TERMINAL_LAUNCH_BUDGET_MS:-300}"

fail() {
  echo "Backlit launch performance verification failed: $*" >&2
  exit 1
}

require_event() {
  event="$1"
  grep -F "\"event\":\"$event\"" "$session_log" >/dev/null || fail "missing event $event"
}

json_u64() {
  event="$1"
  key="$2"
  value="$(
    grep -F "\"event\":\"$event\"" "$session_log" \
      | tail -n 1 \
      | sed -n "s/.*\"$key\":\([0-9][0-9]*\).*/\1/p"
  )"

  test -n "$value" || fail "missing numeric field $key on $event"
  printf '%s\n' "$value"
}

require_le() {
  label="$1"
  value="$2"
  budget="$3"
  if [ "$value" -gt "$budget" ]; then
    fail "$label ${value}ms exceeded budget ${budget}ms"
  fi
}

cargo build -p backlit-session -p backlit-compositor -p backlit-shell

target/debug/backlit-session \
  --backend=headless \
  --socket=backlit-performance \
  --screenshot="$session_screenshot" \
  --verify \
  --verify-launch-spawn \
  --launch-spawn-program=true \
  --wayland-display=backlit-performance \
  --verify-services \
  --service-log-dir="$service_log_dir" > "$session_log" 2> "$session_stderr"

require_event "session.gui_ready"
require_event "session.launch_spawn"
require_event "session.services_verified"
require_event "session.exit"

grep -F '"passed":true' "$session_log" >/dev/null || fail "session verification did not pass"
grep -F '"golden_ok":true' "$session_log" >/dev/null || fail "session golden verification did not pass"
grep -F '"spawned":true' "$session_log" >/dev/null || fail "terminal launch target did not spawn"
grep -F '"exit_success":true' "$session_log" >/dev/null || fail "terminal launch target failed"
grep -F '"wayland_display_set":true' "$session_log" >/dev/null || fail "terminal launch target did not receive WAYLAND_DISPLAY"
grep -F '"compositor_ready":true' "$session_log" >/dev/null || fail "compositor service probe was not ready"
grep -F '"shell_ready":true' "$session_log" >/dev/null || fail "shell service probe was not ready"
grep -F '"children_exited_cleanly":true' "$session_log" >/dev/null || fail "service probes did not exit cleanly"
test -s "$session_screenshot" || fail "missing session screenshot"

startup_ms="$(json_u64 session.gui_ready elapsed_ms)"
terminal_launch_ms="$(json_u64 session.launch_spawn elapsed_ms)"
shell_ready_ms="$(json_u64 session.services_verified elapsed_ms)"
compositor_probe_ms="$(json_u64 session.services_verified compositor_probe_ms)"
shell_probe_ms="$(json_u64 session.services_verified shell_probe_ms)"
session_exit_ms="$(json_u64 session.exit elapsed_ms)"
session_ppm_bytes="$(wc -c < "$session_screenshot" | tr -d ' ')"

require_le "session startup" "$startup_ms" "$startup_budget_ms"
require_le "terminal hotkey launch" "$terminal_launch_ms" "$terminal_launch_budget_ms"
require_le "shell ready after launch" "$shell_ready_ms" "$shell_ready_budget_ms"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-launch-performance",
  "passed": true,
  "backend": "headless",
  "socket": "backlit-performance",
  "artifacts": {
    "session_log": "$session_log",
    "session_stderr": "$session_stderr",
    "session_screenshot": "$session_screenshot",
    "session_services_dir": "$service_log_dir"
  },
  "budgets": {
    "startup_ms": $startup_budget_ms,
    "terminal_launch_ms": $terminal_launch_budget_ms,
    "shell_ready_ms": $shell_ready_budget_ms
  },
  "measurements": {
    "startup_ms": $startup_ms,
    "terminal_launch_ms": $terminal_launch_ms,
    "shell_ready_ms": $shell_ready_ms,
    "compositor_probe_ms": $compositor_probe_ms,
    "shell_probe_ms": $shell_probe_ms,
    "session_exit_ms": $session_exit_ms,
    "session_ppm_bytes": $session_ppm_bytes
  },
  "checks": {
    "startup_budget": true,
    "terminal_launch_budget": true,
    "shell_ready_budget": true,
    "session_launch_spawn": true,
    "session_services": true,
    "golden_gui": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit launch performance verification passed. Artifacts: %s\n' "$out_dir"
