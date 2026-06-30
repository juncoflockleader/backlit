#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/drm-session-smoke}"
mkdir -p "$out_dir"

session_log="$out_dir/session.jsonl"
session_err="$out_dir/session.stderr"
session_screenshot="$out_dir/drm-session.ppm"
service_log_dir="$out_dir/session-services"
expected_checksum="5635038614353063225"
expected_ppm_bytes="1248015"

fail() {
  echo "DRM session smoke verification failed: $*" >&2
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

drm_expected_ready=false
if [ "$(uname -s)" = "Linux" ] \
  && [ "$runtime_present" = true ] \
  && [ "$session_present" = true ] \
  && [ "$drm_node_count" -gt 0 ] \
  && [ "$input_event_nodes" -gt 0 ]; then
  drm_expected_ready=true
fi

drm_session_smoke_ready=false
drm_session_smoke_blocked_expected=false
drm_session_clean_exit=false

if [ "$drm_expected_ready" = true ]; then
  cargo build -p backlit-session -p backlit-compositor -p backlit-shell -p backlit-settings-daemon
  target/debug/backlit-session \
    --backend=drm \
    --socket=backlit-drm-smoke \
    --screenshot="$session_screenshot" \
    --verify \
    --verify-launch-spawn \
    --launch-spawn-program=true \
    --wayland-display=backlit-drm-smoke \
    --verify-services \
    --verify-clean-exit \
    --service-log-dir="$service_log_dir" > "$session_log" 2> "$session_err"

  grep '"event":"session.backend_preflight"' "$session_log" >/dev/null
  grep '"backend":"drm"' "$session_log" >/dev/null
  grep '"ready":true' "$session_log" >/dev/null
  grep '"event":"session.gui_ready"' "$session_log" >/dev/null
  grep '"event":"session.verified"' "$session_log" >/dev/null
  grep '"event":"session.launch_spawn"' "$session_log" >/dev/null
  grep '"event":"session.services_verified"' "$session_log" >/dev/null
  grep '"event":"session.clean_exit"' "$session_log" >/dev/null
  grep '"passed":true' "$session_log" >/dev/null
  grep '"golden_ok":true' "$session_log" >/dev/null
  grep '"spawned":true' "$session_log" >/dev/null
  grep '"exit_success":true' "$session_log" >/dev/null
  grep '"wayland_display_set":true' "$session_log" >/dev/null
  grep '"compositor_ready":true' "$session_log" >/dev/null
  grep '"shell_ready":true' "$session_log" >/dev/null
  grep '"settings_ready":true' "$session_log" >/dev/null
  grep '"children_exited_cleanly":true' "$session_log" >/dev/null
  grep '"windows_before_shutdown":3' "$session_log" >/dev/null
  grep '"windows_closed":3' "$session_log" >/dev/null
  grep '"windows_after_shutdown":0' "$session_log" >/dev/null
  grep '"focus_cleared":true' "$session_log" >/dev/null
  grep "\"checksum\":$expected_checksum" "$session_log" >/dev/null
  test -s "$session_screenshot"

  session_ppm_bytes="$(wc -c < "$session_screenshot" | tr -d ' ')"
  test "$session_ppm_bytes" = "$expected_ppm_bytes"
  drm_session_smoke_ready=true
  drm_session_clean_exit=true
else
  set +e
  cargo run -p backlit-session -- \
    --backend=drm \
    --socket=backlit-drm-smoke \
    --preflight-only > "$session_log" 2> "$session_err"
  session_status="$?"
  set -e

  test "$session_status" -ne 0 || fail "DRM session preflight unexpectedly passed"
  grep '"event":"session.backend_preflight"' "$session_log" >/dev/null
  grep '"backend":"drm"' "$session_log" >/dev/null
  grep '"ready":false' "$session_log" >/dev/null
  grep '"event":"session.launch_ready"' "$session_log" >/dev/null
  grep '"passed":false' "$session_log" >/dev/null
  drm_session_smoke_blocked_expected=true
  session_ppm_bytes=0
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-drm-session-smoke",
  "passed": true,
  "target_os": "$(uname -s)",
  "artifacts": {
    "session_log": "$session_log",
    "session_stderr": "$session_err",
    "session_screenshot": "$session_screenshot",
    "session_services_dir": "$service_log_dir"
  },
  "checks": {
    "drm_expected_ready": $drm_expected_ready,
    "drm_session_smoke_ready": $drm_session_smoke_ready,
    "drm_session_smoke_blocked_expected": $drm_session_smoke_blocked_expected,
    "drm_session_clean_exit": $drm_session_clean_exit,
    "settings_service": $drm_session_smoke_ready,
    "xdg_runtime_dir_present": $runtime_present,
    "session_present": $session_present,
    "drm_card_nodes": $drm_card_nodes,
    "drm_render_nodes": $drm_render_nodes,
    "input_event_nodes": $input_event_nodes,
    "session_ppm_bytes": $session_ppm_bytes
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit DRM session smoke verification passed. Artifacts: %s\n' "$out_dir"
