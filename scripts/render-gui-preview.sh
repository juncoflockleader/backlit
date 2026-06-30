#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/gui-preview}"
mkdir -p "$out_dir"

session_ppm="$out_dir/backlit-session.ppm"
session_png="$out_dir/backlit-session.png"
session_log="$out_dir/session.jsonl"
service_log_dir="$out_dir/session-services"
expected_checksum="5635038614353063225"
expected_ppm_bytes="1248015"

cargo run -p backlit-session -- \
  --backend=headless \
  --socket=backlit-preview \
  --screenshot="$session_ppm" \
  --verify \
  --verify-services \
  --service-log-dir="$service_log_dir" > "$session_log"

grep '"event":"session.verified"' "$session_log" >/dev/null
grep '"event":"session.services_verified"' "$session_log" >/dev/null
grep '"passed":true' "$session_log" >/dev/null
grep '"golden_ok":true' "$session_log" >/dev/null
grep '"compositor_ready":true' "$session_log" >/dev/null
grep '"shell_ready":true' "$session_log" >/dev/null
grep "\"checksum\":$expected_checksum" "$session_log" >/dev/null
test -s "$session_ppm"

session_ppm_bytes="$(wc -c < "$session_ppm" | tr -d ' ')"
test "$session_ppm_bytes" = "$expected_ppm_bytes"

preview_image="$session_ppm"
preview_format="ppm"
png_written=false
converter="none"

if command -v sips >/dev/null 2>&1; then
  if sips -s format png "$session_ppm" --out "$session_png" >/dev/null 2>&1; then
    preview_image="$session_png"
    preview_format="png"
    png_written=true
    converter="sips"
  fi
elif command -v magick >/dev/null 2>&1; then
  if magick "$session_ppm" "$session_png" >/dev/null 2>&1; then
    preview_image="$session_png"
    preview_format="png"
    png_written=true
    converter="magick"
  fi
elif command -v convert >/dev/null 2>&1; then
  if convert "$session_ppm" "$session_png" >/dev/null 2>&1; then
    preview_image="$session_png"
    preview_format="png"
    png_written=true
    converter="convert"
  fi
elif command -v pnmtopng >/dev/null 2>&1; then
  if pnmtopng "$session_ppm" > "$session_png"; then
    preview_image="$session_png"
    preview_format="png"
    png_written=true
    converter="pnmtopng"
  fi
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-gui-preview",
  "passed": true,
  "backend": "headless",
  "socket": "backlit-preview",
  "checksum": $expected_checksum,
  "expected_ppm_bytes": $expected_ppm_bytes,
  "artifacts": {
    "session_log": "$session_log",
    "session_services_dir": "$service_log_dir",
    "session_screenshot_ppm": "$session_ppm",
    "preview_image": "$preview_image"
  },
  "checks": {
    "session_verified": true,
    "session_services": true,
    "ppm_bytes": $session_ppm_bytes,
    "png_written": $png_written,
    "preview_format": "$preview_format",
    "converter": "$converter"
  }
}
EOF

printf 'Backlit GUI preview rendered: %s\n' "$preview_image"
printf 'Manifest: %s\n' "$out_dir/manifest.json"
if [ "$preview_format" = "png" ]; then
  printf 'To view on macOS: open %s\n' "$preview_image"
else
  printf 'No PNG converter found; view the PPM directly or install ImageMagick/netpbm.\n'
fi
