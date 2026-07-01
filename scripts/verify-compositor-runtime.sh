#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/compositor-runtime}"
mkdir -p "$out_dir"

duration_ms=25
compositor_log="$out_dir/compositor-runtime.jsonl"
policy_preview_ppm="$out_dir/scripted-client-policy-preview.ppm"
policy_preview_png="$out_dir/scripted-client-policy-preview.png"

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
  --scripted-client-preview "$policy_preview_ppm" \
  --serve \
  --serve-for-ms "$duration_ms" > "$compositor_log"

require_contains "$compositor_log" '"event":"compositor.scripted_client"'
require_contains "$compositor_log" '"passed":true'
require_contains "$compositor_log" '"runtime_backend":"headless-compositor"'
require_contains "$compositor_log" '"runtime_trait":true'
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
require_contains "$compositor_log" '"policy_windows_after_map":2'
require_contains "$compositor_log" '"policy_visible_windows_after_map":2'
require_contains "$compositor_log" '"policy_focused_after_map":true'
require_contains "$compositor_log" '"policy_preview_requested":true'
require_contains "$compositor_log" '"policy_preview_written":true'
require_contains "$compositor_log" '"policy_preview_verified":true'
require_contains "$compositor_log" '"event":"compositor.ready"'
require_contains "$compositor_log" '"ready":true'
require_contains "$compositor_log" '"event":"compositor.service_running"'
require_contains "$compositor_log" '"event":"compositor.service_exit"'
test -s "$policy_preview_ppm" || fail "missing compositor policy preview $policy_preview_ppm"

policy_preview_ppm_bytes="$(wc -c < "$policy_preview_ppm" | tr -d ' ')"
preview_image="$policy_preview_ppm"
preview_format="ppm"
png_written=false
converter="none"

if command -v sips >/dev/null 2>&1; then
  if sips -s format png "$policy_preview_ppm" --out "$policy_preview_png" >/dev/null 2>&1; then
    preview_image="$policy_preview_png"
    preview_format="png"
    png_written=true
    converter="sips"
  fi
elif command -v magick >/dev/null 2>&1; then
  if magick "$policy_preview_ppm" "$policy_preview_png" >/dev/null 2>&1; then
    preview_image="$policy_preview_png"
    preview_format="png"
    png_written=true
    converter="magick"
  fi
elif command -v convert >/dev/null 2>&1; then
  if convert "$policy_preview_ppm" "$policy_preview_png" >/dev/null 2>&1; then
    preview_image="$policy_preview_png"
    preview_format="png"
    png_written=true
    converter="convert"
  fi
elif command -v pnmtopng >/dev/null 2>&1; then
  if pnmtopng "$policy_preview_ppm" > "$policy_preview_png"; then
    preview_image="$policy_preview_png"
    preview_format="png"
    png_written=true
    converter="pnmtopng"
  fi
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-compositor-runtime",
  "passed": true,
  "duration_ms": $duration_ms,
  "artifacts": {
    "compositor_log": "$compositor_log",
    "policy_preview_ppm": "$policy_preview_ppm",
    "preview_image": "$preview_image"
  },
  "checks": {
    "scripted_client_runtime": true,
    "runtime_backend_contract": true,
    "runtime_backend": "headless-compositor",
    "runtime_trait": true,
    "app_surface_map": true,
    "surface_policy_preview": true,
    "targeted_surface_damage": true,
    "idle_no_redraw": true,
    "surface_close_damage": true,
    "client_disconnect_cleanup": true,
    "service_mode_runtime": true,
    "policy_preview_ppm_bytes": $policy_preview_ppm_bytes,
    "png_written": $png_written,
    "preview_format": "$preview_format",
    "converter": "$converter"
  }
}
EOF

printf 'Backlit compositor runtime verification passed. Artifacts: %s\n' "$out_dir"
if [ "$preview_format" = "png" ]; then
  printf 'Compositor runtime preview: %s\n' "$preview_image"
fi
