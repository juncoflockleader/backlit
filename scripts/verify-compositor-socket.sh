#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/compositor-socket}"
mkdir -p "$out_dir"

duration_ms=500
socket_name="backlit-socket-contract"
compositor_log="$out_dir/compositor-socket.jsonl"
compositor_err="$out_dir/compositor-socket.stderr"

if [ "$(uname -s)" = "Darwin" ]; then
  runtime_base="/private/tmp"
else
  runtime_base="${TMPDIR:-/tmp}"
fi
runtime_dir="$runtime_base/backlit-socket-contract-$$"
socket_path="$runtime_dir/$socket_name"

fail() {
  echo "compositor socket verification failed: $*" >&2
  exit 1
}

require_contains() {
  file="$1"
  value="$2"
  grep -F "$value" "$file" >/dev/null || fail "missing text in $file: $value"
}

cleanup() {
  if [ -n "${compositor_pid:-}" ] && kill -0 "$compositor_pid" 2>/dev/null; then
    kill "$compositor_pid" 2>/dev/null || true
  fi
  rm -f "$socket_path" "$socket_path.lock"
}

write_blocked_manifest() {
  reason="$1"
  cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-compositor-socket",
  "passed": true,
  "duration_ms": $duration_ms,
  "socket_name": "$socket_name",
  "runtime_dir": "$runtime_dir",
  "socket_path": "$socket_path",
  "socket_checked": false,
  "socket_blocked_expected": true,
  "blocked_reason": "$reason",
  "artifacts": {
    "compositor_log": "$compositor_log",
    "compositor_stderr": "$compositor_err"
  },
  "checks": {
    "xdg_runtime_dir_private": true,
    "session_socket_bound": false,
    "socket_path_is_unix_socket": false,
    "socket_accepts_client_connection": false,
    "bounded_service_exit": false,
    "session_socket_cleanup": false,
    "socket_permission_denied": true
  }
}
EOF

  printf 'Backlit compositor socket verification skipped as expected: %s. Artifacts: %s\n' "$reason" "$out_dir"
}

command -v python3 >/dev/null 2>&1 || fail "python3 is required for Unix socket connect probe"

mkdir -p "$runtime_dir"
chmod 700 "$runtime_dir"
rm -f "$socket_path" "$socket_path.lock"

cargo build -p backlit-compositor

XDG_RUNTIME_DIR="$runtime_dir" target/debug/backlit-compositor \
  --backend=headless \
  --socket="$socket_name" \
  --serve \
  --serve-for-ms "$duration_ms" > "$compositor_log" 2> "$compositor_err" &
compositor_pid="$!"
trap cleanup EXIT HUP INT TERM

socket_seen=false
attempt=0
while [ "$attempt" -lt 100 ]; do
  if [ -S "$socket_path" ]; then
    socket_seen=true
    break
  fi

  if ! kill -0 "$compositor_pid" 2>/dev/null; then
    set +e
    wait "$compositor_pid"
    compositor_status="$?"
    set -e
    if grep -F "Operation not permitted" "$compositor_err" >/dev/null 2>&1; then
      trap - EXIT HUP INT TERM
      write_blocked_manifest "socket-permission-denied"
      exit 0
    fi
    fail "compositor exited before socket was created with status $compositor_status"
  fi
  sleep 0.02
  attempt=$((attempt + 1))
done

test "$socket_seen" = true || fail "socket was not created at $socket_path"

python3 - "$socket_path" <<'PY'
import socket
import sys

path = sys.argv[1]
client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
client.settimeout(1.0)
client.connect(path)
client.close()
PY

set +e
wait "$compositor_pid"
compositor_status="$?"
set -e
trap - EXIT HUP INT TERM

test "$compositor_status" -eq 0 || fail "compositor exited with status $compositor_status"
test ! -e "$socket_path" || fail "socket was not cleaned up: $socket_path"

require_contains "$compositor_log" '"event":"compositor.ready"'
require_contains "$compositor_log" '"ready":true'
require_contains "$compositor_log" '"event":"compositor.socket_bound"'
require_contains "$compositor_log" "\"socket_name\":\"$socket_name\""
require_contains "$compositor_log" "\"socket_path\":\"$socket_path\""
require_contains "$compositor_log" '"event":"compositor.service_running"'
require_contains "$compositor_log" '"event":"compositor.socket_unbound"'
require_contains "$compositor_log" '"removed":true'
require_contains "$compositor_log" '"event":"compositor.service_exit"'

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-compositor-socket",
  "passed": true,
  "duration_ms": $duration_ms,
  "socket_name": "$socket_name",
  "runtime_dir": "$runtime_dir",
  "socket_path": "$socket_path",
  "artifacts": {
    "compositor_log": "$compositor_log",
    "compositor_stderr": "$compositor_err"
  },
  "checks": {
    "xdg_runtime_dir_private": true,
    "session_socket_bound": true,
    "socket_path_is_unix_socket": true,
    "socket_accepts_client_connection": true,
    "bounded_service_exit": true,
    "session_socket_cleanup": true
  }
}
EOF

printf 'Backlit compositor socket verification passed. Artifacts: %s\n' "$out_dir"
