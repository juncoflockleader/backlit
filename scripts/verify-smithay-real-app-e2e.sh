#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/smithay-real-app-e2e}"
mkdir -p "$out_dir"

log="$out_dir/smithay-real-app-e2e.jsonl"
err="$out_dir/smithay-real-app-e2e.stderr"
frame_ppm="$out_dir/backlit-real-app-frame.ppm"
manifest="$out_dir/manifest.json"

fail() {
  echo "Smithay real app E2E verification failed: $*" >&2
  exit 1
}

require_contains() {
  file="$1"
  value="$2"
  grep -F -- "$value" "$file" >/dev/null || fail "missing text in $file: $value"
}

require_line_contains_all() {
  file="$1"
  shift

  while IFS= read -r line; do
    line_matches=true
    for value in "$@"; do
      case "$line" in
        *"$value"*) ;;
        *)
          line_matches=false
          break
          ;;
      esac
    done

    if [ "$line_matches" = true ]; then
      return 0
    fi
  done < "$file"

  fail "missing line in $file containing: $*"
}

write_blocked_manifest() {
  reason="$1"
  checked="$2"
  cat > "$manifest" <<EOF
{
  "name": "backlit-smithay-real-app-e2e",
  "passed": true,
  "checked": $checked,
  "expected_blocked": true,
  "reason": "$reason",
  "app_command": "",
  "artifacts": {
    "log": "$log",
    "stderr": "$err",
    "frame_ppm": "$frame_ppm"
  },
  "checks": {
    "smithay_real_app_e2e": false,
    "real_installed_app": false,
    "real_app_wayland_client_connected": false,
    "real_app_metadata_observed": false,
    "real_app_shm_pixels_captured": false,
    "real_app_pixels_composited": false,
    "real_app_frame_samples_verified": false,
    "policy_window_from_real_app": false,
    "policy_geometry_preserved": false,
    "frame_ppm_written": false,
    "drm_launch_ready": false
  }
}
EOF
}

write_manifest() {
  app_command="$1"
  frame_ppm_bytes="$2"
  cat > "$manifest" <<EOF
{
  "name": "backlit-smithay-real-app-e2e",
  "passed": true,
  "checked": true,
  "expected_blocked": false,
  "reason": "real-installed-wayland-app-rendered",
  "app_command": "$app_command",
  "artifacts": {
    "log": "$log",
    "stderr": "$err",
    "frame_ppm": "$frame_ppm"
  },
  "checks": {
    "smithay_real_app_e2e": true,
    "real_installed_app": true,
    "real_app_wayland_client_connected": true,
    "real_app_metadata_observed": true,
    "real_app_shm_pixels_captured": true,
    "real_app_pixels_composited": true,
    "real_app_frame_samples_verified": true,
    "policy_window_from_real_app": true,
    "policy_geometry_preserved": true,
    "frame_ppm_written": true,
    "drm_launch_ready": true,
    "frame_ppm_bytes": $frame_ppm_bytes
  }
}
EOF
}

if [ "$(uname -s)" != "Linux" ]; then
  : > "$log"
  : > "$err"
  write_blocked_manifest "non-linux-host" false
  printf 'Backlit Smithay real app E2E skipped as expected: non-linux-host. Artifacts: %s\n' "$out_dir"
  exit 0
fi

app_command="${BACKLIT_REAL_APP_E2E_COMMAND:-}"
if [ -z "$app_command" ]; then
  if command -v weston-simple-shm >/dev/null 2>&1; then
    app_command="$(command -v weston-simple-shm)"
  elif command -v weston-simple-damage >/dev/null 2>&1; then
    app_command="$(command -v weston-simple-damage)"
  else
    : > "$log"
    : > "$err"
    write_blocked_manifest "missing-installed-wayland-app" true
    printf 'Backlit Smithay real app E2E blocked as expected: no installed Wayland SHM app found. Artifacts: %s\n' "$out_dir"
    exit 0
  fi
fi

test -x "$app_command" || fail "installed Wayland app is not executable: $app_command"

cargo build -p backlit-compositor --features smithay-backend

set +e
target/debug/backlit-compositor \
  --backend=drm \
  --runtime=smithay \
  --smithay-real-app-e2e \
  --smithay-real-app-command "$app_command" \
  --smithay-real-app-frame-output "$frame_ppm" > "$log" 2> "$err"
status=$?
set -e

drm_launch_ready=false
if grep -F '"event":"compositor.backend_preflight","backend":"drm","socket":"backlit-0","ready":true' "$log" >/dev/null; then
  drm_launch_ready=true
fi

if [ "$status" -ne 0 ]; then
  if [ "$drm_launch_ready" = false ]; then
    require_contains "$log" '"event":"compositor.backend_preflight","backend":"drm"'
    require_contains "$log" '"ready":false'
    write_blocked_manifest "drm-preflight-blocked" true
    printf 'Backlit Smithay real app E2E blocked as expected by DRM preflight. Artifacts: %s\n' "$out_dir"
    exit 0
  fi
  cat "$log" >&2 || true
  cat "$err" >&2 || true
  fail "compositor exited with status $status on a launch-ready host"
fi

require_contains "$log" '"event":"compositor.start"'
require_contains "$log" '"backend":"drm"'
require_contains "$log" '"runtime":"smithay"'
require_contains "$log" '"smithay_real_app_e2e":true'
require_contains "$log" '"event":"compositor.backend_preflight","backend":"drm","socket":"backlit-0","ready":true'
require_contains "$log" '"event":"compositor.backend_launch_plan"'
require_contains "$log" '"implementation":"smithay-compositor-runtime"'
require_line_contains_all "$log" \
  '"event":"compositor.smithay_real_app_e2e"' \
  '"passed":true' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"real_installed_app_spawned":true' \
  '"real_app_wayland_client_connected":true' \
  '"real_app_metadata_observed":true' \
  '"real_app_shm_pixels_captured":true' \
  '"real_app_pixels_composited":true' \
  '"real_app_frame_samples_verified":true' \
  '"policy_window_from_real_app":true' \
  '"policy_geometry_preserved":true' \
  '"frame_ppm_written":true' \
  '"app_command":"'"$app_command"'"'

test -s "$frame_ppm" || fail "missing frame PPM artifact: $frame_ppm"
frame_ppm_bytes="$(wc -c < "$frame_ppm" | tr -d '[:space:]')"
if [ "$frame_ppm_bytes" -le 10000 ]; then
  fail "frame PPM artifact is too small: $frame_ppm_bytes bytes"
fi

write_manifest "$app_command" "$frame_ppm_bytes"
printf 'Backlit Smithay real app E2E verification passed. Artifacts: %s\n' "$out_dir"
