#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/smithay-real-shm-frame}"
mkdir -p "$out_dir"

log="$out_dir/smithay-real-shm-frame.jsonl"
err="$out_dir/smithay-real-shm-frame.stderr"
frame_ppm="$out_dir/backlit-real-shm-frame.ppm"
manifest="$out_dir/manifest.json"

fail() {
  echo "Smithay real SHM frame verification failed: $*" >&2
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
  "name": "backlit-smithay-real-shm-frame",
  "passed": true,
  "checked": $checked,
  "expected_blocked": true,
  "reason": "$reason",
  "artifacts": {
    "log": "$log",
    "stderr": "$err",
    "frame_ppm": "$frame_ppm"
  },
  "checks": {
    "smithay_real_shm_frame": false,
    "real_wayland_client": false,
    "real_wayland_metadata": false,
    "real_shm_pixels_captured": false,
    "real_shm_pixels_composited": false,
    "real_client_pixel_samples_verified": false,
    "policy_window_from_real_surface": false,
    "frame_ppm_written": false,
    "drm_launch_ready": false
  }
}
EOF
}

write_manifest() {
  frame_ppm_bytes="$1"
  cat > "$manifest" <<EOF
{
  "name": "backlit-smithay-real-shm-frame",
  "passed": true,
  "checked": true,
  "expected_blocked": false,
  "reason": "real-shm-frame-rendered",
  "artifacts": {
    "log": "$log",
    "stderr": "$err",
    "frame_ppm": "$frame_ppm"
  },
  "checks": {
    "smithay_real_shm_frame": true,
    "real_wayland_client": true,
    "real_wayland_metadata": true,
    "real_shm_pixels_captured": true,
    "real_shm_pixels_composited": true,
    "real_client_pixel_samples_verified": true,
    "policy_window_from_real_surface": true,
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
  printf 'Backlit Smithay real SHM frame skipped as expected: non-linux-host. Artifacts: %s\n' "$out_dir"
  exit 0
fi

cargo build -p backlit-compositor --features smithay-backend

set +e
target/debug/backlit-compositor \
  --backend=drm \
  --runtime=smithay \
  --smithay-real-shm-frame \
  --smithay-real-shm-frame-output "$frame_ppm" > "$log" 2> "$err"
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
    printf 'Backlit Smithay real SHM frame blocked as expected by DRM preflight. Artifacts: %s\n' "$out_dir"
    exit 0
  fi
  cat "$log" >&2 || true
  cat "$err" >&2 || true
  fail "compositor exited with status $status on a launch-ready host"
fi

require_contains "$log" '"event":"compositor.start"'
require_contains "$log" '"backend":"drm"'
require_contains "$log" '"runtime":"smithay"'
require_contains "$log" '"smithay_real_shm_frame":true'
require_contains "$log" '"event":"compositor.backend_preflight","backend":"drm","socket":"backlit-0","ready":true'
require_contains "$log" '"event":"compositor.backend_launch_plan"'
require_contains "$log" '"implementation":"smithay-compositor-runtime"'
require_line_contains_all "$log" \
  '"event":"compositor.smithay_real_shm_frame"' \
  '"passed":true' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"real_wayland_client":true' \
  '"real_wayland_metadata":true' \
  '"real_shm_pixels_captured":true' \
  '"real_shm_pixels_composited":true' \
  '"real_client_pixel_samples_verified":true' \
  '"policy_window_from_real_surface":true' \
  '"policy_app_id_preserved":true' \
  '"policy_geometry_preserved":true' \
  '"frame_ppm_written":true' \
  '"client_width":320' \
  '"client_height":240' \
  '"source_pixel_count":76800' \
  '"composited_pixels":76800' \
  '"source_top_left_red":238' \
  '"source_center_green":187' \
  '"source_bottom_right_blue":224'

test -s "$frame_ppm" || fail "missing frame PPM artifact: $frame_ppm"
frame_ppm_bytes="$(wc -c < "$frame_ppm" | tr -d '[:space:]')"
if [ "$frame_ppm_bytes" -le 10000 ]; then
  fail "frame PPM artifact is too small: $frame_ppm_bytes bytes"
fi

write_manifest "$frame_ppm_bytes"
printf 'Backlit Smithay real SHM frame verification passed. Artifacts: %s\n' "$out_dir"
