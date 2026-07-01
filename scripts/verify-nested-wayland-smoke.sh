#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/nested-wayland-smoke}"
socket_name="${BACKLIT_NESTED_WAYLAND_SOCKET:-backlit-parent-$$}"
mkdir -p "$out_dir"

if [ "$(uname -s)" != "Linux" ]; then
  echo "nested Wayland smoke requires Linux" >&2
  exit 2
fi

if ! command -v weston >/dev/null 2>&1; then
  echo "nested Wayland smoke requires weston" >&2
  exit 2
fi

if command -v wayland-info >/dev/null 2>&1; then
  info_tool="wayland-info"
elif command -v weston-info >/dev/null 2>&1; then
  info_tool="weston-info"
else
  echo "nested Wayland smoke requires wayland-info or weston-info" >&2
  exit 2
fi

if ! command -v foot >/dev/null 2>&1; then
  echo "nested Wayland smoke requires foot for terminal launch verification" >&2
  exit 2
fi

if [ -z "${XDG_RUNTIME_DIR:-}" ] || [ ! -d "$XDG_RUNTIME_DIR" ] || [ ! -w "$XDG_RUNTIME_DIR" ]; then
  XDG_RUNTIME_DIR="$repo_root/$out_dir/runtime"
  export XDG_RUNTIME_DIR
  mkdir -p "$XDG_RUNTIME_DIR"
  chmod 700 "$XDG_RUNTIME_DIR"
fi

rm -f "$XDG_RUNTIME_DIR/$socket_name" "$XDG_RUNTIME_DIR/$socket_name.lock"

weston --backend=headless-backend.so --socket="$socket_name" --idle-time=0 > "$out_dir/weston.log" 2>&1 &
weston_pid="$!"

cleanup() {
  kill "$weston_pid" 2>/dev/null || true
  wait "$weston_pid" 2>/dev/null || true
}
trap cleanup EXIT

for _ in $(seq 1 50); do
  if [ -S "$XDG_RUNTIME_DIR/$socket_name" ]; then
    break
  fi
  if ! kill -0 "$weston_pid" 2>/dev/null; then
    cat "$out_dir/weston.log" >&2
    exit 1
  fi
  sleep 0.1
done

test -S "$XDG_RUNTIME_DIR/$socket_name"

WAYLAND_DISPLAY="$socket_name" timeout 5s "$info_tool" > "$out_dir/wayland-info.txt"
WAYLAND_DISPLAY="$socket_name" cargo run -p backlit-compositor-backend -- --backend=wayland --verify > "$out_dir/backend-preflight.jsonl"
WAYLAND_DISPLAY="$socket_name" cargo run -p backlit-compositor -- --backend=wayland --socket=backlit-0 --smoke-test > "$out_dir/compositor.jsonl"
WAYLAND_DISPLAY="$socket_name" cargo run -p backlit-launcher -- \
  --verify \
  --target=terminal \
  --spawn-smoke \
  --spawn-program="$info_tool" \
  --wayland-display="$socket_name" > "$out_dir/launcher-spawn.jsonl"
WAYLAND_DISPLAY="$socket_name" cargo run -p backlit-launcher -- \
  --verify \
  --target=terminal \
  --spawn-smoke \
  --spawn-arg=-- \
  --spawn-arg=sh \
  --spawn-arg=-lc \
  --spawn-arg=true \
  --allow-status-code=230 \
  --wayland-display="$socket_name" > "$out_dir/terminal-launch.jsonl"
cargo build \
  -p backlit-session \
  -p backlit-compositor \
  -p backlit-demo-client \
  -p backlit-shell \
  -p backlit-notification-daemon \
  -p backlit-settings-daemon
WAYLAND_DISPLAY="$socket_name" cargo run -p backlit-session -- \
  --backend=wayland \
  --socket=backlit-0 \
  --screenshot="$out_dir/session.ppm" \
  --verify \
  --verify-launch-spawn \
  --launch-spawn-program="$info_tool" \
  --verify-desktop-launch \
  --desktop-dir=crates/launcher/fixtures \
  --desktop-entry=org.backlit.SpawnProbe.desktop \
  --wayland-display="$socket_name" \
  --verify-services \
  --verify-clean-exit \
  --service-log-dir="$out_dir/session-services" > "$out_dir/session.jsonl"

grep '"event":"backend.preflight"' "$out_dir/backend-preflight.jsonl" >/dev/null
grep '"backend":"wayland"' "$out_dir/backend-preflight.jsonl" >/dev/null
grep '"ready":true' "$out_dir/backend-preflight.jsonl" >/dev/null
grep '"event":"compositor.smoke_test"' "$out_dir/compositor.jsonl" >/dev/null
grep '"backend":"wayland"' "$out_dir/compositor.jsonl" >/dev/null
grep '"event":"launcher.spawn"' "$out_dir/launcher-spawn.jsonl" >/dev/null
grep '"spawned":true' "$out_dir/launcher-spawn.jsonl" >/dev/null
grep '"exit_success":true' "$out_dir/launcher-spawn.jsonl" >/dev/null
grep '"wayland_display_set":true' "$out_dir/launcher-spawn.jsonl" >/dev/null
grep '"event":"launcher.spawn"' "$out_dir/terminal-launch.jsonl" >/dev/null
grep '"target":"terminal"' "$out_dir/terminal-launch.jsonl" >/dev/null
grep '"program":"foot"' "$out_dir/terminal-launch.jsonl" >/dev/null
grep '"arg_count":4' "$out_dir/terminal-launch.jsonl" >/dev/null
grep '"spawned":true' "$out_dir/terminal-launch.jsonl" >/dev/null
grep '"wayland_display_set":true' "$out_dir/terminal-launch.jsonl" >/dev/null
if grep '"exit_success":true' "$out_dir/terminal-launch.jsonl" >/dev/null; then
  terminal_no_seat_expected=false
else
  grep '"status_code":230' "$out_dir/terminal-launch.jsonl" >/dev/null
  grep '"status_allowed":true' "$out_dir/terminal-launch.jsonl" >/dev/null
  terminal_no_seat_expected=true
fi
grep '"event":"session.services_verified"' "$out_dir/session.jsonl" >/dev/null
grep '"event":"session.launch_spawn"' "$out_dir/session.jsonl" >/dev/null
grep '"event":"session.desktop_launch"' "$out_dir/session.jsonl" >/dev/null
grep '"event":"session.clean_exit"' "$out_dir/session.jsonl" >/dev/null
grep '"backend":"wayland"' "$out_dir/session.jsonl" >/dev/null
grep '"shortcut_resolved":true' "$out_dir/session.jsonl" >/dev/null
grep '"spawned":true' "$out_dir/session.jsonl" >/dev/null
grep '"exit_success":true' "$out_dir/session.jsonl" >/dev/null
grep '"wayland_display_set":true' "$out_dir/session.jsonl" >/dev/null
grep '"entry_selector":"org.backlit.SpawnProbe.desktop"' "$out_dir/session.jsonl" >/dev/null
grep '"entry_resolved":true' "$out_dir/session.jsonl" >/dev/null
grep '"entry_program":"sh"' "$out_dir/session.jsonl" >/dev/null
grep '"program_resolved":true' "$out_dir/session.jsonl" >/dev/null
grep '"managed_window_mapped":true' "$out_dir/session.jsonl" >/dev/null
grep '"managed_window_app_id":"org.backlit.SpawnProbe.desktop"' "$out_dir/session.jsonl" >/dev/null
grep '"managed_windows_after_launch":4' "$out_dir/session.jsonl" >/dev/null
grep '"focused_launched_window":true' "$out_dir/session.jsonl" >/dev/null
grep '"compositor_ready":true' "$out_dir/session.jsonl" >/dev/null
grep '"shell_ready":true' "$out_dir/session.jsonl" >/dev/null
grep '"notification_ready":true' "$out_dir/session.jsonl" >/dev/null
grep '"settings_ready":true' "$out_dir/session.jsonl" >/dev/null
grep '"children_exited_cleanly":true' "$out_dir/session.jsonl" >/dev/null
grep '"workspace_switch_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"snap_left_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"snap_right_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"windows_before_shutdown":3' "$out_dir/session.jsonl" >/dev/null
grep '"windows_closed":3' "$out_dir/session.jsonl" >/dev/null
grep '"windows_after_shutdown":0' "$out_dir/session.jsonl" >/dev/null
grep '"focus_cleared":true' "$out_dir/session.jsonl" >/dev/null
test -s "$out_dir/session.ppm"

session_compositor_demo_client=false
session_compositor_client_blocked_expected=false
if grep '"compositor_demo_surface_mapped":true' "$out_dir/session.jsonl" >/dev/null; then
  grep '"compositor_service_socket_bound":true' "$out_dir/session.jsonl" >/dev/null
  grep '"compositor_demo_client_resolved":true' "$out_dir/session.jsonl" >/dev/null
  grep '"compositor_demo_client_exit_ok":true' "$out_dir/session.jsonl" >/dev/null
  grep '"compositor_demo_client_connected":true' "$out_dir/session.jsonl" >/dev/null
  grep '"compositor_service_socket_cleanup":true' "$out_dir/session.jsonl" >/dev/null
  grep '"compositor_demo_app_id_preserved":true' "$out_dir/session.jsonl" >/dev/null
  session_compositor_demo_client=true
else
  grep '"compositor_client_blocked_expected":true' "$out_dir/session.jsonl" >/dev/null
  session_compositor_client_blocked_expected=true
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-nested-wayland-smoke",
  "passed": true,
  "parent_compositor": "weston-headless",
  "socket": "$socket_name",
  "runtime_dir": "$XDG_RUNTIME_DIR",
  "info_tool": "$info_tool",
  "artifacts": {
    "weston_log": "$out_dir/weston.log",
    "wayland_info": "$out_dir/wayland-info.txt",
    "backend_preflight_log": "$out_dir/backend-preflight.jsonl",
    "compositor_log": "$out_dir/compositor.jsonl",
    "launcher_spawn_log": "$out_dir/launcher-spawn.jsonl",
    "terminal_launch_log": "$out_dir/terminal-launch.jsonl",
    "session_log": "$out_dir/session.jsonl",
    "session_screenshot": "$out_dir/session.ppm",
    "session_services_dir": "$out_dir/session-services"
  },
  "checks": {
    "parent_socket_ready": true,
    "wayland_preflight_ready": true,
    "compositor_wayland_smoke": true,
    "launcher_wayland_client_spawn": true,
    "launcher_terminal_wayland_spawn": true,
    "launcher_terminal_no_seat_expected": $terminal_no_seat_expected,
    "session_wayland_client_spawn": true,
    "session_wayland_desktop_launch": true,
    "session_wayland_desktop_managed_window": true,
    "session_wayland_services": true,
    "session_wayland_demo_client": $session_compositor_demo_client,
    "session_wayland_demo_app_id_preserved": $session_compositor_demo_client,
    "session_wayland_demo_client_blocked_expected": $session_compositor_client_blocked_expected,
    "session_notification_service": true,
    "session_settings_service": true,
    "session_workspace_switch": true,
    "session_snap": true,
    "session_wayland_clean_exit": true
  }
}
EOF

printf 'Backlit nested Wayland smoke verification passed. Artifacts: %s\n' "$out_dir"
