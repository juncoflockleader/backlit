#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/compositor-runtime}"
mkdir -p "$out_dir"

duration_ms=25
compositor_log="$out_dir/compositor-runtime.jsonl"

fail() {
  echo "compositor runtime verification failed: $*" >&2
  exit 1
}

require_contains() {
  file="$1"
  value="$2"
  grep -F "$value" "$file" >/dev/null || fail "missing text in $file: $value"
}

cargo build -p backlit-compositor

target/debug/backlit-compositor \
  --backend=headless \
  --socket=backlit-runtime \
  --scripted-client \
  --serve \
  --serve-for-ms "$duration_ms" > "$compositor_log"

require_contains "$compositor_log" '"event":"compositor.scripted_client"'
require_contains "$compositor_log" '"passed":true'
require_contains "$compositor_log" '"client_connected":true'
require_contains "$compositor_log" '"surfaces_after_map":2'
require_contains "$compositor_log" '"first_frame_damaged_surfaces":2'
require_contains "$compositor_log" '"idle_frame_damaged_surfaces":0'
require_contains "$compositor_log" '"damage_frame_damaged_surfaces":1'
require_contains "$compositor_log" '"post_damage_idle_surfaces":0'
require_contains "$compositor_log" '"close_frame_damaged_surfaces":1'
require_contains "$compositor_log" '"disconnect_frame_damaged_surfaces":1'
require_contains "$compositor_log" '"final_idle_damaged_surfaces":0'
require_contains "$compositor_log" '"surfaces_after_close":1'
require_contains "$compositor_log" '"surfaces_after_disconnect":0'
require_contains "$compositor_log" '"clients_after_disconnect":0'
require_contains "$compositor_log" '"frames":7'
require_contains "$compositor_log" '"no_idle_redraw":true'
require_contains "$compositor_log" '"targeted_damage_ok":true'
require_contains "$compositor_log" '"close_damage_ok":true'
require_contains "$compositor_log" '"disconnect_damage_ok":true'
require_contains "$compositor_log" '"clean_disconnect":true'
require_contains "$compositor_log" '"event":"compositor.ready"'
require_contains "$compositor_log" '"ready":true'
require_contains "$compositor_log" '"event":"compositor.service_running"'
require_contains "$compositor_log" '"event":"compositor.service_exit"'

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-compositor-runtime",
  "passed": true,
  "duration_ms": $duration_ms,
  "artifacts": {
    "compositor_log": "$compositor_log"
  },
  "checks": {
    "scripted_client_runtime": true,
    "app_surface_map": true,
    "targeted_surface_damage": true,
    "idle_no_redraw": true,
    "surface_close_damage": true,
    "client_disconnect_cleanup": true,
    "service_mode_runtime": true
  }
}
EOF

printf 'Backlit compositor runtime verification passed. Artifacts: %s\n' "$out_dir"
