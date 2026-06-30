#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/session-replay}"
mkdir -p "$out_dir"

session_log="$out_dir/session.jsonl"
session_err="$out_dir/session.stderr"
initial_screenshot="$out_dir/initial.ppm"
frames_dir="$out_dir/frames"
expected_frame_count="8"
expected_ppm_bytes="1248015"

fail() {
  echo "Session replay verification failed: $*" >&2
  exit 1
}

cargo run -p backlit-session -- \
  --backend=headless \
  --socket=backlit-replay \
  --screenshot="$initial_screenshot" \
  --verify \
  --scripted-replay-dir="$frames_dir" > "$session_log" 2> "$session_err"

grep '"event":"session.gui_ready"' "$session_log" >/dev/null
grep '"event":"session.verified"' "$session_log" >/dev/null
grep '"event":"session.replay"' "$session_log" >/dev/null
grep '"passed":true' "$session_log" >/dev/null
grep '"frame_count":8' "$session_log" >/dev/null
grep '"frames_written":8' "$session_log" >/dev/null
grep '"distinct_checksums":7' "$session_log" >/dev/null
grep '"app_switcher_focus_changed":true' "$session_log" >/dev/null
grep '"terminal_launch_resolved":true' "$session_log" >/dev/null
grep '"windows_after_launch":4' "$session_log" >/dev/null
grep '"move_begin":true' "$session_log" >/dev/null
grep '"move_frame_changed":true' "$session_log" >/dev/null
grep '"move_grab_ended":true' "$session_log" >/dev/null
grep '"resize_begin":true' "$session_log" >/dev/null
grep '"resize_frame_changed":true' "$session_log" >/dev/null
grep '"resize_grab_ended":true' "$session_log" >/dev/null
grep '"snap_frame_ok":true' "$session_log" >/dev/null
grep '"workspace_hidden":true' "$session_log" >/dev/null
grep '"workspace_switch_ok":true' "$session_log" >/dev/null
grep '"final_visible_windows":1' "$session_log" >/dev/null

for frame in \
  00-initial.ppm \
  01-app-switcher.ppm \
  02-terminal-launch.ppm \
  03-window-moved.ppm \
  04-window-resized.ppm \
  05-window-snapped.ppm \
  06-workspace-hidden.ppm \
  07-workspace-switched.ppm
do
  frame_path="$frames_dir/$frame"
  test -s "$frame_path" || fail "missing replay frame $frame_path"
  frame_bytes="$(wc -c < "$frame_path" | tr -d ' ')"
  test "$frame_bytes" = "$expected_ppm_bytes" || fail "unexpected byte count for $frame_path: $frame_bytes"
done

frame_count="$(find "$frames_dir" -maxdepth 1 -name '*.ppm' -type f | wc -l | tr -d ' ')"
test "$frame_count" = "$expected_frame_count" || fail "unexpected replay frame count: $frame_count"
test -s "$initial_screenshot" || fail "missing initial screenshot"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-session-replay",
  "passed": true,
  "backend": "headless",
  "socket": "backlit-replay",
  "expected_frame_count": $expected_frame_count,
  "expected_ppm_bytes": $expected_ppm_bytes,
  "artifacts": {
    "session_log": "$session_log",
    "session_stderr": "$session_err",
    "initial_screenshot": "$initial_screenshot",
    "frames_dir": "$frames_dir"
  },
  "checks": {
    "session_rendered": true,
    "session_verified": true,
    "session_replay_event": true,
    "frame_count": $frame_count,
    "frames_written": 8,
    "distinct_checksums": 7,
    "app_switcher_focus_changed": true,
    "terminal_launch": true,
    "move_frame": true,
    "resize_frame": true,
    "snap_frame": true,
    "workspace_hidden": true,
    "workspace_switch": true,
    "final_visible_windows": 1
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit session replay verification passed. Artifacts: %s\n' "$out_dir"
