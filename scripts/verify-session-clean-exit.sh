#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/session-clean-exit}"
mkdir -p "$out_dir"

session_log="$out_dir/session.jsonl"
session_err="$out_dir/session.stderr"
screenshot="$out_dir/backlit-session.ppm"

fail() {
  echo "Session clean-exit verification failed: $*" >&2
  exit 1
}

cargo run -p backlit-session -- \
  --backend=headless \
  --socket=backlit-clean-exit \
  --screenshot="$screenshot" \
  --verify \
  --verify-clean-exit > "$session_log" 2> "$session_err"

grep '"event":"session.gui_ready"' "$session_log" >/dev/null
grep '"event":"session.verified"' "$session_log" >/dev/null
grep '"event":"session.clean_exit"' "$session_log" >/dev/null
grep '"passed":true' "$session_log" >/dev/null
grep '"requested":true' "$session_log" >/dev/null
grep '"windows_before_shutdown":3' "$session_log" >/dev/null
grep '"windows_closed":3' "$session_log" >/dev/null
grep '"windows_after_shutdown":0' "$session_log" >/dev/null
grep '"focus_cleared":true' "$session_log" >/dev/null
grep '"event":"session.exit"' "$session_log" >/dev/null
test -s "$screenshot" || fail "missing screenshot artifact"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-session-clean-exit",
  "passed": true,
  "backend": "headless",
  "socket": "backlit-clean-exit",
  "artifacts": {
    "session_log": "$session_log",
    "session_stderr": "$session_err",
    "screenshot": "$screenshot"
  },
  "checks": {
    "session_rendered": true,
    "session_verified": true,
    "clean_exit_event": true,
    "shutdown_requested": true,
    "windows_before_shutdown": 3,
    "windows_closed": 3,
    "windows_after_shutdown": 0,
    "focus_cleared": true,
    "session_exit_event": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit session clean-exit verification passed. Artifacts: %s\n' "$out_dir"
