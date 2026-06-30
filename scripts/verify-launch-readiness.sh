#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/launch-readiness}"
mkdir -p "$out_dir"

headless_log="$out_dir/headless-preflight.jsonl"
drm_log="$out_dir/drm-preflight.jsonl"

fail() {
  echo "Launch readiness verification failed: $*" >&2
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

cargo run -p backlit-compositor-backend -- --backend=headless --verify > "$headless_log"
cargo run -p backlit-compositor-backend -- --backend=drm > "$drm_log"

grep '"event":"backend.preflight"' "$headless_log" >/dev/null
grep '"backend":"headless"' "$headless_log" >/dev/null
grep '"ready":true' "$headless_log" >/dev/null
grep '"event":"backend.preflight"' "$drm_log" >/dev/null
grep '"backend":"drm"' "$drm_log" >/dev/null
grep '"xdg_runtime_dir_owned_by_user":' "$drm_log" >/dev/null

drm_ready=false
if grep '"ready":true' "$drm_log" >/dev/null; then
  drm_ready=true
else
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

runtime_owned_by_user=false
if [ "$(uname -s)" = "Linux" ] && [ "$runtime_present" = true ]; then
  runtime_owner_uid="$(stat -c '%u' "$XDG_RUNTIME_DIR" 2>/dev/null || printf unknown)"
  current_uid="$(id -u)"
  if [ "$runtime_owner_uid" = "$current_uid" ]; then
    runtime_owned_by_user=true
  fi
fi

drm_card_nodes="$(count_matching /dev/dri 'card*')"
drm_render_nodes="$(count_matching /dev/dri 'renderD*')"
input_event_nodes="$(count_matching /dev/input 'event*')"
drm_node_count=$((drm_card_nodes + drm_render_nodes))

drm_expected_ready=false
if [ "$(uname -s)" = "Linux" ] \
  && [ "$runtime_present" = true ] \
  && [ "$runtime_owned_by_user" = true ] \
  && [ "$session_present" = true ] \
  && [ "$drm_node_count" -gt 0 ] \
  && [ "$input_event_nodes" -gt 0 ]; then
  drm_expected_ready=true
fi

drm_blocked_expected=false
if [ "$drm_expected_ready" = true ]; then
  if [ "$drm_ready" != true ]; then
    cat "$drm_log" >&2
    fail "DRM preflight should be ready on this host"
  fi
else
  if [ "$drm_ready" = false ]; then
    drm_blocked_expected=true
  fi
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-launch-readiness",
  "passed": true,
  "target_os": "$(uname -s)",
  "artifacts": {
    "headless_preflight_log": "$headless_log",
    "drm_preflight_log": "$drm_log"
  },
  "checks": {
    "headless_ready": true,
    "drm_checked": true,
    "drm_ready": $drm_ready,
    "drm_expected_ready": $drm_expected_ready,
    "drm_blocked_expected": $drm_blocked_expected,
    "xdg_runtime_dir_present": $runtime_present,
    "xdg_runtime_dir_owned_by_user": $runtime_owned_by_user,
    "session_present": $session_present,
    "drm_card_nodes": $drm_card_nodes,
    "drm_render_nodes": $drm_render_nodes,
    "input_event_nodes": $input_event_nodes
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit launch readiness verification passed. Artifacts: %s\n' "$out_dir"
