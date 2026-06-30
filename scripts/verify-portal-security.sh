#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/portal-security}"
mkdir -p "$out_dir"

portal_log="$out_dir/portal.jsonl"

cargo run -p backlit-portal-backend -- --verify > "$portal_log"

grep '"event":"portal_backend.security_smoke"' "$portal_log" >/dev/null
grep '"passed":true' "$portal_log" >/dev/null
grep '"direct_screenshot_denied":true' "$portal_log" >/dev/null
grep '"direct_screencast_denied":true' "$portal_log" >/dev/null
grep '"direct_remote_desktop_denied":true' "$portal_log" >/dev/null
grep '"unconsented_portal_denied":true' "$portal_log" >/dev/null
grep '"consented_screenshot_allowed":true' "$portal_log" >/dev/null
grep '"consented_screencast_allowed":true' "$portal_log" >/dev/null
grep '"file_chooser_allowed":true' "$portal_log" >/dev/null

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-portal-security",
  "passed": true,
  "artifacts": {
    "portal_log": "$portal_log"
  },
  "checks": {
    "direct_screenshot_denied": true,
    "direct_screencast_denied": true,
    "direct_remote_desktop_denied": true,
    "unconsented_portal_denied": true,
    "consented_screenshot_allowed": true,
    "consented_screencast_allowed": true,
    "file_chooser_allowed": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit portal security verification passed. Artifacts: %s\n' "$out_dir"
