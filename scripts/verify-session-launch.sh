#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/session-launch}"
mkdir -p "$out_dir"

session_desktop="packaging/sessions/backlit.desktop"
headless_log="$out_dir/headless-session.jsonl"
headless_err="$out_dir/headless-session.stderr"
systemd_units_log="$out_dir/systemd-units.jsonl"
systemd_units_err="$out_dir/systemd-units.stderr"
drm_log="$out_dir/drm-session.jsonl"
drm_err="$out_dir/drm-session.stderr"

fail() {
  echo "Session launch verification failed: $*" >&2
  exit 1
}

count_matching() {
  dir="$1"
  pattern="$2"

  if [ ! -d "$dir" ]; then
    printf '0'
    return
  fi

  find "$dir" -maxdepth 1 -name "$pattern" -print 2>/dev/null | wc -l | tr -d ' '
}

test -f "$session_desktop" || fail "missing $session_desktop"
session_exec="$(sed -n 's/^Exec=//p' "$session_desktop")"
session_exec_program="${session_exec%% *}"
test "$session_exec" = "backlit-session --backend=drm" || fail "unexpected desktop Exec=$session_exec"
test "$session_exec_program" = "backlit-session" || fail "unexpected desktop Exec program=$session_exec_program"

cargo run -p backlit-session -- \
  --backend=headless \
  --socket=backlit-launch-check \
  --preflight-only > "$headless_log" 2> "$headless_err"

grep '"event":"session.backend_preflight"' "$headless_log" >/dev/null
grep '"backend":"headless"' "$headless_log" >/dev/null
grep '"ready":true' "$headless_log" >/dev/null
grep '"event":"session.launch_ready"' "$headless_log" >/dev/null
grep '"passed":true' "$headless_log" >/dev/null
grep '"preflight_only":true' "$headless_log" >/dev/null
grep '"event":"session.exit"' "$headless_log" >/dev/null

cargo run -p backlit-session -- \
  --backend=headless \
  --socket=backlit-launch-check \
  --preflight-only \
  --verify-systemd-units \
  --systemd-unit-dir packaging/systemd > "$systemd_units_log" 2> "$systemd_units_err"

grep '"event":"session.systemd_units_verified"' "$systemd_units_log" >/dev/null
grep '"event":"session.systemd_launch_plan"' "$systemd_units_log" >/dev/null
grep '"passed":true' "$systemd_units_log" >/dev/null
grep '"session_target_ready":true' "$systemd_units_log" >/dev/null
grep '"session_target_wants_services":true' "$systemd_units_log" >/dev/null
grep '"launch_plan_ready":true' "$systemd_units_log" >/dev/null
grep '"target":"backlit-session.target"' "$systemd_units_log" >/dev/null
grep '"service_units":4' "$systemd_units_log" >/dev/null
grep '"import_environment_command":"systemctl --user import-environment XDG_RUNTIME_DIR XDG_SESSION_ID XDG_SEAT XDG_SESSION_TYPE WAYLAND_DISPLAY XDG_CURRENT_DESKTOP DESKTOP_SESSION"' "$systemd_units_log" >/dev/null
grep '"start_target_command":"systemctl --user start backlit-session.target"' "$systemd_units_log" >/dev/null
grep '"stop_target_command":"systemctl --user stop backlit-session.target"' "$systemd_units_log" >/dev/null
grep '"units_present":true' "$systemd_units_log" >/dev/null
grep '"exec_starts":true' "$systemd_units_log" >/dev/null
grep '"startup_order":true' "$systemd_units_log" >/dev/null
grep '"graphical_session_target":true' "$systemd_units_log" >/dev/null
grep '"journal_output":true' "$systemd_units_log" >/dev/null
grep '"rust_backtrace_enabled":true' "$systemd_units_log" >/dev/null
grep '"restart_policy":true' "$systemd_units_log" >/dev/null
grep '"event":"session.exit"' "$systemd_units_log" >/dev/null

set +e
cargo run -p backlit-session -- \
  --backend=drm \
  --socket=backlit-launch-check \
  --preflight-only > "$drm_log" 2> "$drm_err"
drm_status="$?"
set -e

grep '"event":"session.backend_preflight"' "$drm_log" >/dev/null
grep '"backend":"drm"' "$drm_log" >/dev/null
grep '"event":"session.launch_ready"' "$drm_log" >/dev/null
grep '"preflight_only":true' "$drm_log" >/dev/null

drm_session_ready=false
if [ "$drm_status" -eq 0 ]; then
  grep '"passed":true' "$drm_log" >/dev/null
  grep '"ready":true' "$drm_log" >/dev/null
  grep '"event":"session.exit"' "$drm_log" >/dev/null
  drm_session_ready=true
else
  grep '"passed":false' "$drm_log" >/dev/null
  grep '"ready":false' "$drm_log" >/dev/null
fi

runtime_present=false
if [ -n "${XDG_RUNTIME_DIR:-}" ] && [ -d "${XDG_RUNTIME_DIR:-}" ]; then
  runtime_present=true
fi

session_present=false
if [ -n "${XDG_SESSION_ID:-}" ]; then
  session_present=true
fi

drm_card_nodes="$(count_matching /dev/dri 'card*')"
drm_render_nodes="$(count_matching /dev/dri 'renderD*')"
input_event_nodes="$(count_matching /dev/input 'event*')"
drm_node_count=$((drm_card_nodes + drm_render_nodes))

drm_session_expected_ready=false
if [ "$(uname -s)" = "Linux" ] \
  && [ "$runtime_present" = true ] \
  && [ "$session_present" = true ] \
  && [ "$drm_node_count" -gt 0 ] \
  && [ "$input_event_nodes" -gt 0 ]; then
  drm_session_expected_ready=true
fi

drm_session_blocked_expected=false
if [ "$drm_session_expected_ready" = true ]; then
  if [ "$drm_session_ready" != true ]; then
    cat "$drm_log" >&2
    cat "$drm_err" >&2
    fail "DRM session preflight should be ready on this host"
  fi
else
  if [ "$drm_session_ready" = true ]; then
    cat "$drm_log" >&2
    fail "DRM session preflight passed on a host this verifier expected to be blocked"
  fi
  drm_session_blocked_expected=true
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-session-launch",
  "passed": true,
  "target_os": "$(uname -s)",
  "artifacts": {
    "session_desktop": "$session_desktop",
    "headless_session_log": "$headless_log",
    "headless_session_stderr": "$headless_err",
    "systemd_units_log": "$systemd_units_log",
    "systemd_units_stderr": "$systemd_units_err",
    "drm_session_log": "$drm_log",
    "drm_session_stderr": "$drm_err"
  },
  "checks": {
    "desktop_exec": "$session_exec",
    "desktop_exec_program": "$session_exec_program",
    "headless_session_launch_ready": true,
    "session_systemd_units": true,
    "session_systemd_target": true,
    "session_systemd_launch_plan": true,
    "drm_session_checked": true,
    "drm_session_ready": $drm_session_ready,
    "drm_session_expected_ready": $drm_session_expected_ready,
    "drm_session_blocked_expected": $drm_session_blocked_expected,
    "xdg_runtime_dir_present": $runtime_present,
    "session_present": $session_present,
    "drm_card_nodes": $drm_card_nodes,
    "drm_render_nodes": $drm_render_nodes,
    "input_event_nodes": $input_event_nodes
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit session launch verification passed. Artifacts: %s\n' "$out_dir"
