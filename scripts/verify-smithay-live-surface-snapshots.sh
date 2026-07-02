#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/smithay-live-surface-snapshots}"
mkdir -p "$out_dir"

log="$out_dir/smithay-live-surface-snapshots.jsonl"
err="$out_dir/smithay-live-surface-snapshots.stderr"
manifest="$out_dir/manifest.json"

fail() {
  echo "Smithay live surface snapshot verification failed: $*" >&2
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
  "name": "backlit-smithay-live-surface-snapshots",
  "passed": true,
  "checked": $checked,
  "expected_blocked": true,
  "reason": "$reason",
  "artifacts": {
    "log": "$log",
    "stderr": "$err"
  },
  "checks": {
    "smithay_live_surface_snapshots": false,
    "real_wayland_client": false,
    "live_snapshot_pipeline": false,
    "live_snapshot_persisted": false,
    "live_snapshot_metadata_preserved": false,
    "live_snapshot_pixels_copied": false,
    "live_snapshot_damage_recorded": false,
    "live_snapshot_samples_verified": false,
    "policy_window_from_live_snapshot": false,
    "drm_launch_ready": false
  }
}
EOF
}

write_manifest() {
  cat > "$manifest" <<EOF
{
  "name": "backlit-smithay-live-surface-snapshots",
  "passed": true,
  "checked": true,
  "expected_blocked": false,
  "reason": "live-surface-snapshots-captured",
  "artifacts": {
    "log": "$log",
    "stderr": "$err"
  },
  "checks": {
    "smithay_live_surface_snapshots": true,
    "real_wayland_client": true,
    "live_snapshot_pipeline": true,
    "live_snapshot_persisted": true,
    "live_snapshot_metadata_preserved": true,
    "live_snapshot_pixels_copied": true,
    "live_snapshot_damage_recorded": true,
    "live_snapshot_samples_verified": true,
    "policy_window_from_live_snapshot": true,
    "drm_launch_ready": true
  }
}
EOF
}

if [ "$(uname -s)" != "Linux" ]; then
  : > "$log"
  : > "$err"
  write_blocked_manifest "non-linux-host" false
  printf 'Backlit Smithay live surface snapshots skipped as expected: non-linux-host. Artifacts: %s\n' "$out_dir"
  exit 0
fi

cargo build -p backlit-compositor --features smithay-backend

set +e
target/debug/backlit-compositor \
  --backend=drm \
  --runtime=smithay \
  --smithay-live-surface-snapshots > "$log" 2> "$err"
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
    printf 'Backlit Smithay live surface snapshots blocked as expected by DRM preflight. Artifacts: %s\n' "$out_dir"
    exit 0
  fi
  cat "$log" >&2 || true
  cat "$err" >&2 || true
  fail "compositor exited with status $status on a launch-ready host"
fi

require_contains "$log" '"event":"compositor.start"'
require_contains "$log" '"backend":"drm"'
require_contains "$log" '"runtime":"smithay"'
require_contains "$log" '"smithay_live_surface_snapshots":true'
require_contains "$log" '"event":"compositor.backend_preflight","backend":"drm","socket":"backlit-0","ready":true'
require_contains "$log" '"event":"compositor.backend_launch_plan"'
require_contains "$log" '"implementation":"smithay-compositor-runtime"'
require_line_contains_all "$log" \
  '"event":"compositor.smithay_live_surface_snapshots"' \
  '"passed":true' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"real_wayland_client":true' \
  '"live_snapshot_pipeline":true' \
  '"live_snapshot_persisted":true' \
  '"live_snapshot_metadata_preserved":true' \
  '"live_snapshot_pixels_copied":true' \
  '"live_snapshot_damage_recorded":true' \
  '"live_snapshot_samples_verified":true' \
  '"policy_window_from_live_snapshot":true' \
  '"policy_app_id_preserved":true' \
  '"policy_geometry_preserved":true' \
  '"snapshot_count":1' \
  '"persisted_snapshot_count":1' \
  '"damage_x":0' \
  '"damage_y":0' \
  '"damage_width":320' \
  '"damage_height":240' \
  '"snapshot_width":320' \
  '"snapshot_height":240' \
  '"snapshot_pixel_count":76800' \
  '"source_top_left_red":238' \
  '"source_center_green":187' \
  '"source_bottom_right_blue":224'

write_manifest
printf 'Backlit Smithay live surface snapshot verification passed. Artifacts: %s\n' "$out_dir"
