#!/usr/bin/env sh
set -eu

out_dir="${1:-target/gui-smoke}"
mkdir -p "$out_dir"

expected_checksum="5635038614353063225"
expected_width="800"
expected_height="520"
expected_ppm_bytes="1248015"

cargo run -p backlit-compositor -- --backend=headless --smoke-test > "$out_dir/compositor.jsonl"
cargo run -p backlit-compositor-backend -- --backend=headless --verify > "$out_dir/backend-preflight.jsonl"
cargo run -p backlit-protocols -- --verify --list > "$out_dir/protocols.jsonl"
cargo run -p backlit-perf -- --verify > "$out_dir/perf.jsonl"
cargo run -p backlit-shell -- --component=all --socket=backlit-0 --verify > "$out_dir/shell.jsonl"
cargo run -p backlit-launcher -- --verify --list --target=terminal --desktop-dir=crates/launcher/fixtures > "$out_dir/launcher.jsonl"
cargo run -p backlit-launcher -- \
  --verify \
  --target=terminal \
  --spawn-smoke \
  --spawn-program=true \
  --wayland-display=backlit-0 > "$out_dir/launcher-spawn.jsonl"
cargo run -p backlit-shortcuts -- --verify --list --resolve=Super+Enter > "$out_dir/shortcuts.jsonl"
cargo run -p backlit-input -- --verify > "$out_dir/input.jsonl"
cargo run -p backlit-surface -- --verify > "$out_dir/surface.jsonl"
cargo run -p backlit-session-supervisor -- --verify > "$out_dir/supervisor.jsonl"
cargo run -p backlit-clipboard -- --verify > "$out_dir/clipboard.jsonl"
cargo run -p backlit-session -- \
  --backend=headless \
  --socket=backlit-0 \
  --screenshot="$out_dir/backlit-session.ppm" \
  --verify \
  --verify-launch-spawn \
  --launch-spawn-program=true \
  --wayland-display=backlit-0 \
  --verify-services \
  --verify-clean-exit \
  --service-log-dir="$out_dir/session-services" > "$out_dir/session.jsonl"
cargo run -p backlit-demo-client -- \
  --output="$out_dir/demo-client.ppm" \
  --verify > "$out_dir/demo-client.jsonl"

grep '"event":"compositor.smoke_test"' "$out_dir/compositor.jsonl" >/dev/null
grep '"idle_damaged_surfaces":0' "$out_dir/compositor.jsonl" >/dev/null
grep '"targeted_damage_surfaces":1' "$out_dir/compositor.jsonl" >/dev/null
grep '"post_damage_idle_surfaces":0' "$out_dir/compositor.jsonl" >/dev/null
grep '"no_idle_redraw":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"targeted_damage_ok":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"direct_scanout_eligible":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"direct_scanout_dmabuf":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"direct_scanout_fullscreen":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"direct_scanout_overlay_blocked":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"direct_scanout_shm_blocked":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"event":"session.verified"' "$out_dir/session.jsonl" >/dev/null
grep '"event":"session.interactions"' "$out_dir/session.jsonl" >/dev/null
grep '"event":"session.launch_spawn"' "$out_dir/session.jsonl" >/dev/null
grep '"event":"session.services_verified"' "$out_dir/session.jsonl" >/dev/null
grep '"event":"session.clean_exit"' "$out_dir/session.jsonl" >/dev/null
grep '"windows_after_launch":4' "$out_dir/session.jsonl" >/dev/null
grep '"terminal_launch_resolved":true' "$out_dir/session.jsonl" >/dev/null
grep '"shortcut_resolved":true' "$out_dir/session.jsonl" >/dev/null
grep '"target_resolved":true' "$out_dir/session.jsonl" >/dev/null
grep '"spawned":true' "$out_dir/session.jsonl" >/dev/null
grep '"exit_success":true' "$out_dir/session.jsonl" >/dev/null
grep '"wayland_display_set":true' "$out_dir/session.jsonl" >/dev/null
grep '"move_resize_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"minimize_skips_focus":true' "$out_dir/session.jsonl" >/dev/null
grep '"resized_width":920' "$out_dir/session.jsonl" >/dev/null
grep '"maximize_uses_work_area":true' "$out_dir/session.jsonl" >/dev/null
grep '"fullscreen_uses_output":true' "$out_dir/session.jsonl" >/dev/null
grep '"close_fallback_focus_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"keyboard_input_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"pointer_input_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"input_windows_after_terminal_launch":4' "$out_dir/session.jsonl" >/dev/null
grep '"surface_lifecycle_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"surface_windows_after_close":0' "$out_dir/session.jsonl" >/dev/null
grep '"windows_after_close":3' "$out_dir/session.jsonl" >/dev/null
grep '"passed":true' "$out_dir/session.jsonl" >/dev/null
grep '"golden_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"compositor_ready":true' "$out_dir/session.jsonl" >/dev/null
grep '"shell_ready":true' "$out_dir/session.jsonl" >/dev/null
grep '"children_exited_cleanly":true' "$out_dir/session.jsonl" >/dev/null
grep '"logs_written":true' "$out_dir/session.jsonl" >/dev/null
grep '"windows_before_shutdown":3' "$out_dir/session.jsonl" >/dev/null
grep '"windows_closed":3' "$out_dir/session.jsonl" >/dev/null
grep '"windows_after_shutdown":0' "$out_dir/session.jsonl" >/dev/null
grep '"focus_cleared":true' "$out_dir/session.jsonl" >/dev/null
grep "\"checksum\":$expected_checksum" "$out_dir/session.jsonl" >/dev/null
grep '"event":"backend.preflight"' "$out_dir/backend-preflight.jsonl" >/dev/null
grep '"ready":true' "$out_dir/backend-preflight.jsonl" >/dev/null
grep '"event":"protocol.smoke"' "$out_dir/protocols.jsonl" >/dev/null
grep '"required_protocols":7' "$out_dir/protocols.jsonl" >/dev/null
grep '"event":"perf.smoke"' "$out_dir/perf.jsonl" >/dev/null
grep '"passed":true' "$out_dir/perf.jsonl" >/dev/null
grep '"golden_ok":true' "$out_dir/perf.jsonl" >/dev/null
grep '"idle_damaged_surfaces":0' "$out_dir/perf.jsonl" >/dev/null
grep '"targeted_damage_surfaces":1' "$out_dir/perf.jsonl" >/dev/null
grep '"post_damage_idle_surfaces":0' "$out_dir/perf.jsonl" >/dev/null
grep '"no_idle_redraw":true' "$out_dir/perf.jsonl" >/dev/null
grep '"targeted_damage_ok":true' "$out_dir/perf.jsonl" >/dev/null
grep '"pointer_frame_budget_us":16000' "$out_dir/perf.jsonl" >/dev/null
grep '"drag_frames":60' "$out_dir/perf.jsonl" >/dev/null
grep '"drag_dropped_frames":0' "$out_dir/perf.jsonl" >/dev/null
grep '"drag_dropped_frame_budget":0' "$out_dir/perf.jsonl" >/dev/null
grep '"drag_damage_ok":true' "$out_dir/perf.jsonl" >/dev/null
grep '"drag_frame_pacing_ok":true' "$out_dir/perf.jsonl" >/dev/null
grep '"event":"shell.verified"' "$out_dir/shell.jsonl" >/dev/null
grep '"required_components":4' "$out_dir/shell.jsonl" >/dev/null
grep '"event":"launcher.verified"' "$out_dir/launcher.jsonl" >/dev/null
grep '"required_targets":3' "$out_dir/launcher.jsonl" >/dev/null
grep '"desktop_entries":3' "$out_dir/launcher.jsonl" >/dev/null
grep '"target":"terminal"' "$out_dir/launcher.jsonl" >/dev/null
grep '"event":"launcher.spawn"' "$out_dir/launcher-spawn.jsonl" >/dev/null
grep '"target":"terminal"' "$out_dir/launcher-spawn.jsonl" >/dev/null
grep '"spawned":true' "$out_dir/launcher-spawn.jsonl" >/dev/null
grep '"exit_success":true' "$out_dir/launcher-spawn.jsonl" >/dev/null
grep '"wayland_display_set":true' "$out_dir/launcher-spawn.jsonl" >/dev/null
grep '"event":"shortcut.verified"' "$out_dir/shortcuts.jsonl" >/dev/null
grep '"required_bindings":6' "$out_dir/shortcuts.jsonl" >/dev/null
grep '"action":"launch-terminal"' "$out_dir/shortcuts.jsonl" >/dev/null
grep '"event":"input.smoke"' "$out_dir/input.jsonl" >/dev/null
grep '"terminal_launch_resolved":true' "$out_dir/input.jsonl" >/dev/null
grep '"app_switcher_changed_focus":true' "$out_dir/input.jsonl" >/dev/null
grep '"pointer_move_window":true' "$out_dir/input.jsonl" >/dev/null
grep '"pointer_resize_window":true' "$out_dir/input.jsonl" >/dev/null
grep '"pointer_grab_ended":true' "$out_dir/input.jsonl" >/dev/null
grep '"event":"surface.lifecycle"' "$out_dir/surface.jsonl" >/dev/null
grep '"xdg_shell_registered":true' "$out_dir/surface.jsonl" >/dev/null
grep '"mapped_window":true' "$out_dir/surface.jsonl" >/dev/null
grep '"focused_after_map":true' "$out_dir/surface.jsonl" >/dev/null
grep '"maximize_uses_work_area":true' "$out_dir/surface.jsonl" >/dev/null
grep '"fullscreen_uses_output":true' "$out_dir/surface.jsonl" >/dev/null
grep '"window_removed":true' "$out_dir/surface.jsonl" >/dev/null
grep '"event":"supervisor.crash_smoke"' "$out_dir/supervisor.jsonl" >/dev/null
grep '"shell_crash_isolated":true' "$out_dir/supervisor.jsonl" >/dev/null
grep '"compositor_crash_ends_session":true' "$out_dir/supervisor.jsonl" >/dev/null
grep '"event":"clipboard.smoke"' "$out_dir/clipboard.jsonl" >/dev/null
grep '"generation":3' "$out_dir/clipboard.jsonl" >/dev/null
grep '"event":"demo_client.verified"' "$out_dir/demo-client.jsonl" >/dev/null
grep '"passed":true' "$out_dir/demo-client.jsonl" >/dev/null
grep '"golden_ok":true' "$out_dir/demo-client.jsonl" >/dev/null
test -s "$out_dir/backlit-session.ppm"
test -s "$out_dir/demo-client.ppm"

session_ppm_bytes="$(wc -c < "$out_dir/backlit-session.ppm" | tr -d ' ')"
demo_ppm_bytes="$(wc -c < "$out_dir/demo-client.ppm" | tr -d ' ')"
test "$session_ppm_bytes" = "$expected_ppm_bytes"
test "$demo_ppm_bytes" = "$expected_ppm_bytes"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-gui-smoke",
  "passed": true,
  "backend": "headless",
  "socket": "backlit-0",
  "width": $expected_width,
  "height": $expected_height,
  "checksum": $expected_checksum,
  "expected_ppm_bytes": $expected_ppm_bytes,
  "artifacts": {
    "compositor_log": "$out_dir/compositor.jsonl",
    "backend_preflight_log": "$out_dir/backend-preflight.jsonl",
    "protocols_log": "$out_dir/protocols.jsonl",
    "perf_log": "$out_dir/perf.jsonl",
    "shell_log": "$out_dir/shell.jsonl",
    "launcher_log": "$out_dir/launcher.jsonl",
    "launcher_spawn_log": "$out_dir/launcher-spawn.jsonl",
    "shortcuts_log": "$out_dir/shortcuts.jsonl",
    "input_log": "$out_dir/input.jsonl",
    "surface_log": "$out_dir/surface.jsonl",
    "supervisor_log": "$out_dir/supervisor.jsonl",
    "clipboard_log": "$out_dir/clipboard.jsonl",
    "session_log": "$out_dir/session.jsonl",
    "session_services_dir": "$out_dir/session-services",
    "demo_client_log": "$out_dir/demo-client.jsonl",
    "session_screenshot": "$out_dir/backlit-session.ppm",
    "demo_client_screenshot": "$out_dir/demo-client.ppm"
  },
  "checks": {
    "protocol_required_count": 7,
    "shell_required_components": 4,
    "launcher_required_targets": 3,
    "desktop_entries": 3,
    "launcher_spawn": true,
    "shortcut_required_bindings": 6,
    "keyboard_input": true,
    "pointer_input": true,
    "surface_lifecycle": true,
    "no_idle_redraw": true,
    "targeted_damage": true,
    "direct_scanout": true,
    "drag_frame_pacing": true,
    "shell_crash_isolated": true,
    "clipboard_generation": 3,
    "session_windows_after_launch": 4,
    "session_launch_spawn": true,
    "session_services": true,
    "session_clean_exit": true,
    "session_move_resize": true,
    "session_minimize_skips_focus": true,
    "session_close_fallback_focus": true,
    "session_input": true,
    "session_surface_lifecycle": true,
    "work_area_y": 42,
    "session_ppm_bytes": $session_ppm_bytes,
    "demo_ppm_bytes": $demo_ppm_bytes,
    "golden_checksum": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit GUI smoke verification passed. Artifacts: %s\n' "$out_dir"
