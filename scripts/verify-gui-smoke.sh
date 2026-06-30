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
cargo run -p backlit-launcher -- --verify --list --target=terminal --desktop-dir=crates/launcher/fixtures --require-desktop-entries > "$out_dir/launcher.jsonl"
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
cargo run -p backlit-notification-daemon -- --verify > "$out_dir/notification-daemon.jsonl"
cargo run -p backlit-settings-daemon -- --verify > "$out_dir/settings-daemon.jsonl"
cargo run -p backlit-settings -- --verify > "$out_dir/settings-app.jsonl"
cargo run -p backlit-portal-backend -- --verify > "$out_dir/portal.jsonl"
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
grep '"xdg_shell_registered":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_surface_lifecycle":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_toplevel_created":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_initial_configured":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_ack_configure_ok":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_mapped_window":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_backend_surface_presented":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_popup_created":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_popup_mapped":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_popup_backend_surface_presented":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_popup_position_constrained":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_popup_did_not_create_window":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_popup_closed":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_presented_surfaces":2' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_presented_pixels":345600' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_focused_after_map":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_maximize_uses_work_area":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_fullscreen_uses_output":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_close_requested":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_window_removed":true' "$out_dir/compositor.jsonl" >/dev/null
grep '"xdg_windows_after_close":0' "$out_dir/compositor.jsonl" >/dev/null
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
grep '"workspace_switch_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"workspace_hidden_windows":1' "$out_dir/session.jsonl" >/dev/null
grep '"workspace_restored_focus":true' "$out_dir/session.jsonl" >/dev/null
grep '"snap_left_ok":true' "$out_dir/session.jsonl" >/dev/null
grep '"snap_right_ok":true' "$out_dir/session.jsonl" >/dev/null
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
grep '"notification_ready":true' "$out_dir/session.jsonl" >/dev/null
grep '"settings_ready":true' "$out_dir/session.jsonl" >/dev/null
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
grep '"required_components":5' "$out_dir/shell.jsonl" >/dev/null
grep '"required_roles":5' "$out_dir/shell.jsonl" >/dev/null
grep '"wallpaper_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"panel_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"launcher_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"app_switcher_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"lock_screen_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"clock_visible":true' "$out_dir/shell.jsonl" >/dev/null
grep '"battery_visible":true' "$out_dir/shell.jsonl" >/dev/null
grep '"network_visible":true' "$out_dir/shell.jsonl" >/dev/null
grep '"volume_visible":true' "$out_dir/shell.jsonl" >/dev/null
grep '"power_menu_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"power_menu_visible":true' "$out_dir/shell.jsonl" >/dev/null
grep '"power_menu_actions":4' "$out_dir/shell.jsonl" >/dev/null
grep '"power_menu_lock":true' "$out_dir/shell.jsonl" >/dev/null
grep '"power_menu_logout":true' "$out_dir/shell.jsonl" >/dev/null
grep '"power_menu_reboot":true' "$out_dir/shell.jsonl" >/dev/null
grep '"power_menu_shutdown":true' "$out_dir/shell.jsonl" >/dev/null
grep '"network_status_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"network_controls_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"network_control_commands":3' "$out_dir/shell.jsonl" >/dev/null
grep '"network_controls_dry_run":true' "$out_dir/shell.jsonl" >/dev/null
grep '"network_backend":"NetworkManager"' "$out_dir/shell.jsonl" >/dev/null
grep '"network_control_tool":"nmcli"' "$out_dir/shell.jsonl" >/dev/null
grep '"network_connected":true' "$out_dir/shell.jsonl" >/dev/null
grep '"network_strength_percent":84' "$out_dir/shell.jsonl" >/dev/null
grep '"network_wifi_scan_command":"nmcli device wifi list"' "$out_dir/shell.jsonl" >/dev/null
grep '"network_wifi_connect_command":"nmcli device wifi connect $SSID password $PASSWORD"' "$out_dir/shell.jsonl" >/dev/null
grep '"network_disconnect_command":"nmcli device disconnect $DEVICE"' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_status_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_controls_ready":true' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_control_commands":3' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_controls_dry_run":true' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_backend":"PipeWire"' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_control_tool":"wpctl"' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_muted":false' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_volume_percent":72' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_volume_up_command":"wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+"' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_volume_down_command":"wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"' "$out_dir/shell.jsonl" >/dev/null
grep '"audio_mute_toggle_command":"wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"' "$out_dir/shell.jsonl" >/dev/null
grep '"workspace_indicator_visible":true' "$out_dir/shell.jsonl" >/dev/null
grep '"workspace_count":4' "$out_dir/shell.jsonl" >/dev/null
grep '"active_workspace":0' "$out_dir/shell.jsonl" >/dev/null
grep '"launcher_targets":3' "$out_dir/shell.jsonl" >/dev/null
grep '"terminal_target":true' "$out_dir/shell.jsonl" >/dev/null
grep '"browser_target":true' "$out_dir/shell.jsonl" >/dev/null
grep '"settings_target":true' "$out_dir/shell.jsonl" >/dev/null
grep '"app_switcher_entries":3' "$out_dir/shell.jsonl" >/dev/null
grep '"lock_screen_covers_output":true' "$out_dir/shell.jsonl" >/dev/null
grep '"lock_screen_unlock_prompt_visible":true' "$out_dir/shell.jsonl" >/dev/null
grep '"lock_screen_password_field_focused":true' "$out_dir/shell.jsonl" >/dev/null
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
grep '"created_popup":true' "$out_dir/surface.jsonl" >/dev/null
grep '"popup_configured":true' "$out_dir/surface.jsonl" >/dev/null
grep '"popup_ack_configure_ok":true' "$out_dir/surface.jsonl" >/dev/null
grep '"popup_mapped":true' "$out_dir/surface.jsonl" >/dev/null
grep '"popup_position_constrained":true' "$out_dir/surface.jsonl" >/dev/null
grep '"popup_keeps_parent_focus":true' "$out_dir/surface.jsonl" >/dev/null
grep '"popup_did_not_create_window":true' "$out_dir/surface.jsonl" >/dev/null
grep '"popup_close_requested":true' "$out_dir/surface.jsonl" >/dev/null
grep '"popup_closed":true' "$out_dir/surface.jsonl" >/dev/null
grep '"windows_after_popup_close":1' "$out_dir/surface.jsonl" >/dev/null
grep '"maximize_uses_work_area":true' "$out_dir/surface.jsonl" >/dev/null
grep '"fullscreen_uses_output":true' "$out_dir/surface.jsonl" >/dev/null
grep '"window_removed":true' "$out_dir/surface.jsonl" >/dev/null
grep '"event":"supervisor.crash_smoke"' "$out_dir/supervisor.jsonl" >/dev/null
grep '"shell_crash_isolated":true' "$out_dir/supervisor.jsonl" >/dev/null
grep '"compositor_crash_ends_session":true' "$out_dir/supervisor.jsonl" >/dev/null
grep '"event":"supervisor.crash_log"' "$out_dir/supervisor.jsonl" >/dev/null
grep '"crash_logs_recorded":true' "$out_dir/supervisor.jsonl" >/dev/null
grep '"journalctl_user_scope":true' "$out_dir/supervisor.jsonl" >/dev/null
grep '"shell_journal_unit":"backlit-shell.service"' "$out_dir/supervisor.jsonl" >/dev/null
grep '"compositor_journal_unit":"backlit-compositor.service"' "$out_dir/supervisor.jsonl" >/dev/null
grep '"event":"clipboard.smoke"' "$out_dir/clipboard.jsonl" >/dev/null
grep '"generation":3' "$out_dir/clipboard.jsonl" >/dev/null
grep '"event":"notification_daemon.smoke"' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"notify_calls":3' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"replacement_preserved_id":true' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"action_invoked":true' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"closed_replaced":true' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"closed_expired":true' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"closed_dismissed":true' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"critical_persistent":true' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"spec_fields_valid":true' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"active_after_cleanup":0' "$out_dir/notification-daemon.jsonl" >/dev/null
grep '"event":"settings_daemon.verified"' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"display_validated":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"input_validated":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"power_validated":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"invalid_display_rejected":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"invalid_input_rejected":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"invalid_power_rejected":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"power_menu_complete":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"power_action_commands_complete":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"power_action_commands":5' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"power_actions_dry_run":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"disruptive_power_actions_guarded":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"lock_action_ready":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"logout_action_ready":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"suspend_action_ready":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"reboot_action_ready":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"shutdown_action_ready":true' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"logout_command":"loginctl terminate-session $XDG_SESSION_ID"' "$out_dir/settings-daemon.jsonl" >/dev/null
grep '"event":"settings_app.verified"' "$out_dir/settings-app.jsonl" >/dev/null
grep '"passed":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"application_id":"org.backlit.Settings"' "$out_dir/settings-app.jsonl" >/dev/null
grep '"launcher_target_ready":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"required_panels":3' "$out_dir/settings-app.jsonl" >/dev/null
grep '"display_panel_ready":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"display_modes":3' "$out_dir/settings-app.jsonl" >/dev/null
grep '"display_scale_options":4' "$out_dir/settings-app.jsonl" >/dev/null
grep '"display_apply_validated":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"input_panel_ready":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"keyboard_repeat_visible":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"pointer_accel_visible":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"touchpad_toggle_visible":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"power_panel_ready":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"power_menu_actions":4' "$out_dir/settings-app.jsonl" >/dev/null
grep '"power_command_plans_available":true' "$out_dir/settings-app.jsonl" >/dev/null
grep '"daemon_generation":3' "$out_dir/settings-app.jsonl" >/dev/null
grep '"event":"portal_backend.security_smoke"' "$out_dir/portal.jsonl" >/dev/null
grep '"direct_screenshot_denied":true' "$out_dir/portal.jsonl" >/dev/null
grep '"direct_screencast_denied":true' "$out_dir/portal.jsonl" >/dev/null
grep '"direct_remote_desktop_denied":true' "$out_dir/portal.jsonl" >/dev/null
grep '"unconsented_portal_denied":true' "$out_dir/portal.jsonl" >/dev/null
grep '"consented_screenshot_allowed":true' "$out_dir/portal.jsonl" >/dev/null
grep '"consented_screencast_allowed":true' "$out_dir/portal.jsonl" >/dev/null
grep '"file_chooser_allowed":true' "$out_dir/portal.jsonl" >/dev/null
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
    "notification_daemon_log": "$out_dir/notification-daemon.jsonl",
    "settings_daemon_log": "$out_dir/settings-daemon.jsonl",
    "settings_app_log": "$out_dir/settings-app.jsonl",
    "portal_log": "$out_dir/portal.jsonl",
    "session_log": "$out_dir/session.jsonl",
    "session_services_dir": "$out_dir/session-services",
    "demo_client_log": "$out_dir/demo-client.jsonl",
    "session_screenshot": "$out_dir/backlit-session.ppm",
    "demo_client_screenshot": "$out_dir/demo-client.ppm"
  },
  "checks": {
    "protocol_required_count": 7,
    "shell_required_components": 5,
    "shell_required_roles": 5,
    "shell_wallpaper": true,
    "shell_panel_status": true,
    "shell_power_menu": true,
    "shell_network_status": true,
    "shell_network_controls": true,
    "shell_audio_status": true,
    "shell_audio_controls": true,
    "shell_workspace_indicator": true,
    "shell_launcher_targets": 3,
    "shell_app_switcher": true,
    "shell_lock_screen": true,
    "launcher_required_targets": 3,
    "desktop_entries": 3,
    "launcher_spawn": true,
    "shortcut_required_bindings": 6,
    "keyboard_input": true,
    "pointer_input": true,
    "surface_lifecycle": true,
    "popup_lifecycle": true,
    "compositor_surface_lifecycle": true,
    "compositor_popup_lifecycle": true,
    "no_idle_redraw": true,
    "targeted_damage": true,
    "direct_scanout": true,
    "drag_frame_pacing": true,
    "shell_crash_isolated": true,
    "crash_logs": true,
    "clipboard_generation": 3,
    "notification_daemon": true,
    "settings_daemon": true,
    "settings_power_actions": true,
    "settings_power_actions_dry_run": true,
    "settings_suspend_action": true,
    "settings_app": true,
    "settings_app_display_panel": true,
    "settings_app_input_panel": true,
    "settings_app_power_panel": true,
    "portal_security": true,
    "session_windows_after_launch": 4,
    "session_launch_spawn": true,
    "session_services": true,
    "session_notification_service": true,
    "session_settings_service": true,
    "session_clean_exit": true,
    "session_move_resize": true,
    "session_workspace_switch": true,
    "session_snap": true,
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
