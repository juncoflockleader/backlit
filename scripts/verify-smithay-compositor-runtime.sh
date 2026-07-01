#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/smithay-compositor-runtime}"
mkdir -p "$out_dir"

log="$out_dir/smithay-compositor-runtime.jsonl"
err="$out_dir/smithay-compositor-runtime.stderr"
service_log="$out_dir/smithay-compositor-service.jsonl"
service_err="$out_dir/smithay-compositor-service.stderr"
first_demo_client_log="$out_dir/demo-client-first-socket.jsonl"
demo_client_log="$out_dir/demo-client-socket.jsonl"
service_duration_ms=500
socket_name="backlit-smithay-service-contract-$$"
socket_path=""
compositor_pid=""

fail() {
  echo "Smithay compositor runtime verification failed: $*" >&2
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

cleanup() {
  if [ -n "$compositor_pid" ] && kill -0 "$compositor_pid" 2>/dev/null; then
    kill "$compositor_pid" 2>/dev/null || true
  fi
  if [ -n "$socket_path" ]; then
    rm -f "$socket_path"
  fi
}

write_blocked_manifest() {
  reason="$1"
  checked="$2"
  cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-smithay-compositor-runtime",
  "passed": true,
  "checked": $checked,
  "expected_blocked": true,
  "reason": "$reason",
  "artifacts": {
    "compositor_log": "$log",
    "compositor_stderr": "$err",
    "service_log": "$service_log",
    "service_stderr": "$service_err",
    "first_demo_client_log": "$first_demo_client_log",
    "demo_client_log": "$demo_client_log"
  },
  "checks": {
    "smithay_compositor_runtime": false,
    "smithay_runtime_trait": false,
    "smithay_scripted_client": false,
    "smithay_event_loop_runtime": false,
    "smithay_service_ready": false,
    "smithay_service_socket": false,
    "smithay_service_socket_runtime_trait": false,
    "smithay_event_loop_service_socket": false,
    "smithay_demo_client_socket_lifecycle": false,
    "drm_launch_ready": false
  }
}
EOF
}

if [ "$(uname -s)" != "Linux" ]; then
  : > "$log"
  : > "$err"
  write_blocked_manifest "non-linux-host" false
  printf 'Backlit Smithay compositor runtime skipped as expected: non-linux-host. Artifacts: %s\n' "$out_dir"
  exit 0
fi

cargo build -p backlit-compositor --features smithay-backend
cargo build -p backlit-demo-client

set +e
target/debug/backlit-compositor \
  --backend=drm \
  --runtime=smithay \
  --scripted-client > "$log" 2> "$err"
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
    printf 'Backlit Smithay compositor runtime blocked as expected by DRM preflight. Artifacts: %s\n' "$out_dir"
    exit 0
  fi
  cat "$log" >&2 || true
  cat "$err" >&2 || true
  fail "compositor exited with status $status on a launch-ready host"
fi

require_contains "$log" '"event":"compositor.start"'
require_contains "$log" '"backend":"drm"'
require_contains "$log" '"runtime":"smithay"'
require_contains "$log" '"event":"compositor.backend_preflight","backend":"drm","socket":"backlit-0","ready":true'
require_contains "$log" '"event":"compositor.scripted_client"'
require_contains "$log" '"passed":true'
require_contains "$log" '"runtime_backend":"smithay-compositor-runtime"'
require_contains "$log" '"runtime_trait":true'
require_line_contains_all "$log" \
  '"event":"compositor.scripted_client"' \
  '"inserted_wayland_clients":1' \
  '"wayland_dispatch_count":7' \
  '"calloop_dispatch_count":7'
require_contains "$log" '"client_connected":true'
require_contains "$log" '"surfaces_after_map":2'
require_contains "$log" '"targeted_damage_ok":true'
require_contains "$log" '"clean_disconnect":true'
require_contains "$log" '"event":"compositor.ready"'
require_contains "$log" '"ready":true'
require_line_contains_all "$log" \
  '"event":"compositor.ready"' \
  '"inserted_wayland_clients":1' \
  '"wayland_dispatch_count":1' \
  '"calloop_dispatch_count":1'
require_contains "$log" '"bootstrap_client_connected":true'
require_contains "$log" '"bootstrap_surface_presented":true'

runtime_dir="${XDG_RUNTIME_DIR:-}"
test -n "$runtime_dir" || fail "XDG_RUNTIME_DIR missing on launch-ready Linux host"
socket_path="$runtime_dir/$socket_name"
rm -f "$socket_path"

target/debug/backlit-compositor \
  --backend=drm \
  --runtime=smithay \
  --socket "$socket_name" \
  --serve \
  --serve-for-ms "$service_duration_ms" > "$service_log" 2> "$service_err" &
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
    service_status="$?"
    set -e
    cat "$service_log" >&2 || true
    cat "$service_err" >&2 || true
    fail "Smithay service exited before socket was created with status $service_status"
  fi
  sleep 0.02
  attempt=$((attempt + 1))
done

test "$socket_seen" = true || fail "Smithay service socket was not created at $socket_path"

target/debug/backlit-demo-client \
  --connect-socket "$socket_name" \
  --connect-title smithay-socket-terminal \
  --connect-app-id org.backlit.SmithaySocketTerminal \
  --connect-only \
  --width 640 \
  --height 480 > "$first_demo_client_log"

target/debug/backlit-demo-client \
  --connect-socket "$socket_name" \
  --connect-title smithay-socket-browser \
  --connect-app-id org.backlit.SmithaySocketBrowser \
  --connect-management \
  --connect-lifecycle \
  --connect-only \
  --width 900 \
  --height 600 > "$demo_client_log"

set +e
wait "$compositor_pid"
service_status="$?"
set -e
compositor_pid=""
trap - EXIT HUP INT TERM

test "$service_status" -eq 0 || fail "Smithay service exited with status $service_status"
test ! -e "$socket_path" || fail "Smithay service socket was not cleaned up: $socket_path"

require_contains "$service_log" '"event":"compositor.start"'
require_contains "$service_log" '"backend":"drm"'
require_contains "$service_log" '"runtime":"smithay"'
require_contains "$service_log" '"event":"compositor.ready"'
require_contains "$service_log" '"runtime_backend":"smithay-compositor-runtime"'
require_contains "$service_log" '"ready":true'
require_line_contains_all "$service_log" \
  '"event":"compositor.ready"' \
  '"inserted_wayland_clients":1' \
  '"wayland_dispatch_count":1' \
  '"calloop_dispatch_count":1'
require_contains "$service_log" '"event":"compositor.socket_bound"'
require_contains "$service_log" "\"socket_name\":\"$socket_name\""
require_contains "$service_log" "\"socket_path\":\"$socket_path\""
require_contains "$service_log" '"event":"compositor.socket_client"'
require_contains "$service_log" '"runtime_backend":"smithay-compositor-runtime"'
require_line_contains_all "$service_log" \
  '"action":"surface"' \
  '"title":"smithay-socket-terminal"' \
  '"app_id":"org.backlit.SmithaySocketTerminal"' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"backend_surface_presented":true' \
  '"inserted_wayland_clients":1' \
  '"wayland_dispatch_count":1' \
  '"calloop_dispatch_count":1' \
  '"policy_window_mapped":true' \
  '"policy_app_id_preserved":true'
require_line_contains_all "$service_log" \
  '"action":"surface"' \
  '"title":"smithay-socket-browser"' \
  '"app_id":"org.backlit.SmithaySocketBrowser"' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"backend_surface_presented":true' \
  '"policy_window_mapped":true' \
  '"policy_app_id_preserved":true' \
  '"backend_clients":2' \
  '"backend_surfaces":2' \
  '"inserted_wayland_clients":2' \
  '"wayland_dispatch_count":2' \
  '"calloop_dispatch_count":2'
require_line_contains_all "$service_log" \
  '"action":"move"' \
  '"title":"smithay-socket-browser"' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"backend_surface_damaged":true' \
  '"policy_window_moved":true'
require_line_contains_all "$service_log" \
  '"action":"resize"' \
  '"title":"smithay-socket-browser"' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"backend_surface_damaged":true' \
  '"policy_window_resized":true'
require_line_contains_all "$service_log" \
  '"action":"maximize"' \
  '"title":"smithay-socket-browser"' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"backend_surface_damaged":true' \
  '"policy_window_maximized":true' \
  '"policy_state":"maximized"'
require_line_contains_all "$service_log" \
  '"action":"fullscreen"' \
  '"title":"smithay-socket-browser"' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"backend_surface_damaged":true' \
  '"policy_window_fullscreen":true' \
  '"policy_state":"fullscreen"'
require_line_contains_all "$service_log" \
  '"action":"close"' \
  '"title":"smithay-socket-browser"' \
  '"runtime_backend":"smithay-compositor-runtime"' \
  '"backend_surface_closed":true' \
  '"policy_window_closed":true' \
  '"client_disconnected":true' \
  '"backend_clients":1' \
  '"backend_surfaces":1' \
  '"inserted_wayland_clients":2' \
  '"wayland_dispatch_count":8' \
  '"calloop_dispatch_count":8' \
  '"policy_windows":1'
require_contains "$service_log" '"event":"compositor.socket_unbound"'
require_contains "$service_log" '"removed":true'
require_contains "$service_log" '"event":"compositor.service_exit"'
require_contains "$first_demo_client_log" '"event":"demo_client.socket_connected"'
require_contains "$first_demo_client_log" '"connected":true'
require_contains "$demo_client_log" '"event":"demo_client.socket_connected"'
require_contains "$demo_client_log" '"management":true'
require_contains "$demo_client_log" '"lifecycle":true'
require_contains "$demo_client_log" '"connected":true'

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-smithay-compositor-runtime",
  "passed": true,
  "checked": true,
  "expected_blocked": false,
  "socket_name": "$socket_name",
  "socket_path": "$socket_path",
  "artifacts": {
    "compositor_log": "$log",
    "compositor_stderr": "$err",
    "service_log": "$service_log",
    "service_stderr": "$service_err",
    "first_demo_client_log": "$first_demo_client_log",
    "demo_client_log": "$demo_client_log"
  },
  "checks": {
    "smithay_compositor_runtime": true,
    "smithay_runtime_trait": true,
    "smithay_scripted_client": true,
    "smithay_event_loop_runtime": true,
    "smithay_service_ready": true,
    "smithay_service_socket": true,
    "smithay_service_socket_runtime_trait": true,
    "smithay_event_loop_service_socket": true,
    "smithay_demo_client_socket_lifecycle": true,
    "drm_launch_ready": $drm_launch_ready
  }
}
EOF

printf 'Backlit Smithay compositor runtime verification passed. Artifacts: %s\n' "$out_dir"
