#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/staged-session-install}"
stage_dir="$out_dir/root"
bin_dir="$stage_dir/usr/bin"
session_dir="$stage_dir/usr/share/wayland-sessions"
systemd_dir="$stage_dir/usr/lib/systemd/user"
session_desktop="$session_dir/backlit.desktop"
compositor_service="$systemd_dir/backlit-compositor.service"
shell_service="$systemd_dir/backlit-shell.service"
session_log="$out_dir/session.jsonl"
session_screenshot="$out_dir/staged-session.ppm"
compositor_log="$out_dir/compositor.jsonl"
shell_log="$out_dir/shell.jsonl"

fail() {
  echo "staged session install verification failed: $*" >&2
  exit 1
}

require_file() {
  test -f "$1" || fail "missing file $1"
}

require_executable() {
  test -x "$1" || fail "missing executable $1"
}

require_line() {
  file="$1"
  line="$2"
  grep -Fx "$line" "$file" >/dev/null || fail "missing line in $file: $line"
}

resolve_usr_bin() {
  value="$1"
  case "$value" in
    /usr/bin/*) printf '%s/%s\n' "$bin_dir" "${value#/usr/bin/}" ;;
    *) fail "ExecStart does not use /usr/bin: $value" ;;
  esac
}

mkdir -p "$bin_dir" "$session_dir" "$systemd_dir" "$out_dir"

cargo build -p backlit-session -p backlit-compositor -p backlit-shell

install -m 0755 target/debug/backlit-session "$bin_dir/backlit-session"
install -m 0755 target/debug/backlit-compositor "$bin_dir/backlit-compositor"
install -m 0755 target/debug/backlit-shell "$bin_dir/backlit-shell"
install -m 0644 packaging/sessions/backlit.desktop "$session_desktop"
install -m 0644 packaging/systemd/backlit-compositor.service "$compositor_service"
install -m 0644 packaging/systemd/backlit-shell.service "$shell_service"

require_file "$session_desktop"
require_file "$compositor_service"
require_file "$shell_service"

require_line "$session_desktop" "Exec=backlit-session"
desktop_exec="$(sed -n 's/^Exec=//p' "$session_desktop")"
test "$desktop_exec" = "backlit-session" || fail "unexpected session desktop Exec=$desktop_exec"
require_executable "$bin_dir/$desktop_exec"

require_line "$compositor_service" "ExecStart=/usr/bin/backlit-compositor --backend=drm --socket=backlit-0"
require_line "$shell_service" "ExecStart=/usr/bin/backlit-shell --component=all --socket=backlit-0"

compositor_exec_start="$(sed -n 's/^ExecStart=//p' "$compositor_service")"
shell_exec_start="$(sed -n 's/^ExecStart=//p' "$shell_service")"
compositor_command="${compositor_exec_start%% *}"
shell_command="${shell_exec_start%% *}"
require_executable "$(resolve_usr_bin "$compositor_command")"
require_executable "$(resolve_usr_bin "$shell_command")"

"$bin_dir/backlit-session" --help > "$out_dir/backlit-session.help"
"$bin_dir/backlit-compositor" --help > "$out_dir/backlit-compositor.help"
"$bin_dir/backlit-shell" --help > "$out_dir/backlit-shell.help"

"$bin_dir/backlit-session" \
  --backend=headless \
  --socket=backlit-0 \
  --screenshot "$session_screenshot" \
  --verify > "$session_log"

require_file "$session_screenshot"
grep -F '"event":"session.interactions"' "$session_log" >/dev/null || fail "missing session interaction event"
grep -F '"event":"session.verified"' "$session_log" >/dev/null || fail "missing session verification event"
grep -F '"passed":true' "$session_log" >/dev/null || fail "session verification did not pass"
grep -F '"golden_ok":true' "$session_log" >/dev/null || fail "session golden verification did not pass"

"$bin_dir/backlit-compositor" --backend=headless --socket=backlit-0 --smoke-test > "$compositor_log"
grep -F '"event":"compositor.smoke_test"' "$compositor_log" >/dev/null || fail "missing compositor smoke event"

"$bin_dir/backlit-shell" --component=all --socket=backlit-0 --verify > "$shell_log"
grep -F '"event":"shell.verified"' "$shell_log" >/dev/null || fail "missing shell verification event"
grep -F '"passed":true' "$shell_log" >/dev/null || fail "shell verification did not pass"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-staged-session-install",
  "passed": true,
  "stage_root": "$stage_dir",
  "artifacts": {
    "session_desktop": "$session_desktop",
    "compositor_service": "$compositor_service",
    "shell_service": "$shell_service",
    "session_log": "$session_log",
    "session_screenshot": "$session_screenshot",
    "compositor_log": "$compositor_log",
    "shell_log": "$shell_log"
  },
  "checks": {
    "desktop_exec_resolves": true,
    "systemd_exec_resolves": true,
    "staged_session_help": true,
    "staged_session_gui": true,
    "staged_compositor_smoke": true,
    "staged_shell_verify": true
  }
}
EOF

printf 'Backlit staged session install verification passed. Artifacts: %s\n' "$out_dir"
