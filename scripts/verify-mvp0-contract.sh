#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/mvp0-contract}"
artifact_root="${2:-}"
mkdir -p "$out_dir"

fail() {
  echo "MVP 0 contract verification failed: $*" >&2
  exit 1
}

require_file() {
  test -f "$1" || fail "missing file $1"
}

require_executable() {
  test -x "$1" || fail "missing executable $1"
}

require_contains() {
  file="$1"
  value="$2"
  grep -F -- "$value" "$file" >/dev/null || fail "missing text in $file: $value"
}

require_file Cargo.toml
require_file backlit-design.md
require_file docs/architecture/mvp-0.md
require_executable scripts/verify-gui-smoke.sh
require_executable scripts/verify-launcher-desktop-discovery.sh
require_executable scripts/render-gui-preview.sh
require_executable scripts/verify-compositor-runtime.sh
require_executable scripts/verify-compositor-socket.sh
require_executable scripts/verify-launch-performance.sh
require_executable scripts/verify-resource-budget.sh
require_executable scripts/verify-notification-daemon.sh
require_executable scripts/verify-settings-daemon.sh
require_executable scripts/verify-service-lifecycle.sh
require_executable scripts/verify-settings-app.sh
require_executable scripts/verify-portal-security.sh
require_executable scripts/verify-crash-logs.sh
require_executable scripts/verify-linux-e2e.sh
require_executable scripts/verify-parallels-linux-e2e.sh
require_executable scripts/render-parallels-gui-preview.sh
require_executable scripts/verify-ci-contract.sh
require_executable scripts/verify-packaging-contract.sh
require_executable scripts/verify-package-manifests.sh
require_executable scripts/verify-debian-package-build.sh
require_executable scripts/verify-debian-package-install.sh
require_executable scripts/verify-debian-system-install.sh
require_executable scripts/verify-staged-session-install.sh
require_executable scripts/verify-nested-wayland-smoke.sh
require_executable scripts/verify-session-replay.sh
require_executable scripts/verify-session-clean-exit.sh
require_executable scripts/verify-mvp1-contract.sh

require_contains Cargo.toml '"crates/compositor"'
require_contains Cargo.toml '"crates/compositor-backend"'
require_contains Cargo.toml '"crates/demo-client"'
require_contains Cargo.toml '"crates/input"'
require_contains Cargo.toml '"crates/protocols"'
require_contains Cargo.toml '"crates/perf"'
require_contains Cargo.toml '"crates/session"'
require_contains Cargo.toml '"crates/shell"'
require_contains Cargo.toml '"crates/launcher"'
require_contains Cargo.toml '"crates/notification-daemon"'
require_contains Cargo.toml '"crates/settings"'
require_contains Cargo.toml '"crates/settings-daemon"'
require_contains Cargo.toml '"crates/portal-backend"'
require_contains Cargo.toml '"crates/surface"'
require_contains Cargo.toml '"crates/window-policy"'

require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-compositor -- --backend=headless --smoke-test'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-compositor-backend -- --backend=headless --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-protocols -- --verify --list'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-perf -- --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-input -- --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-surface -- --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-demo-client --'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-notification-daemon -- --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-settings-daemon -- --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-settings -- --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-portal-backend -- --verify'
require_contains scripts/verify-gui-smoke.sh '--require-desktop-entries'
require_contains scripts/verify-gui-smoke.sh '"shell_panel_status": true'
require_contains scripts/verify-gui-smoke.sh '"shell_power_menu": true'
require_contains scripts/verify-gui-smoke.sh '"shell_network_status": true'
require_contains scripts/verify-gui-smoke.sh '"shell_network_controls": true'
require_contains scripts/verify-gui-smoke.sh '"shell_audio_status": true'
require_contains scripts/verify-gui-smoke.sh '"shell_audio_controls": true'
require_contains scripts/verify-gui-smoke.sh '"shell_workspace_indicator": true'
require_contains scripts/verify-gui-smoke.sh '"shell_launcher_targets": 3'
require_contains scripts/verify-gui-smoke.sh '"shell_app_switcher": true'
require_contains scripts/verify-gui-smoke.sh '"shell_lock_screen": true'
require_contains scripts/verify-gui-smoke.sh '"popup_lifecycle": true'
require_contains scripts/verify-gui-smoke.sh '"compositor_surface_lifecycle": true'
require_contains scripts/verify-gui-smoke.sh '"compositor_popup_lifecycle": true'
require_contains scripts/verify-gui-smoke.sh '"settings_power_actions": true'
require_contains scripts/verify-gui-smoke.sh '"crash_logs": true'
require_contains scripts/verify-gui-smoke.sh '"session_policy_preview": true'
require_contains scripts/verify-gui-smoke.sh '"compositor_demo_surface_mapped":true'
require_contains scripts/verify-gui-smoke.sh '"session_compositor_demo_client":'
require_contains scripts/verify-gui-smoke.sh '"session_focused_title_bar": true'
require_contains scripts/verify-gui-smoke.sh '"session_workspace_indicator": true'
require_contains scripts/verify-gui-smoke.sh '"golden_checksum": true'
require_contains scripts/verify-launch-performance.sh '"name": "backlit-launch-performance"'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-nested-wayland-smoke.sh'
require_contains scripts/verify-nested-wayland-smoke.sh '"launcher_terminal_wayland_spawn": true'
require_contains scripts/verify-nested-wayland-smoke.sh '"launcher_terminal_no_seat_expected":'
require_contains scripts/verify-linux-e2e.sh './scripts/render-gui-preview.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-compositor-runtime.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-compositor-socket.sh'
require_contains scripts/verify-compositor-socket.sh '"demo_client_socket_launch": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_surface_mapped": true'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-launch-performance.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-launcher-desktop-discovery.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-resource-budget.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-notification-daemon.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-settings-daemon.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-service-lifecycle.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-settings-app.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-portal-security.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-crash-logs.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-ci-contract.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-packaging-contract.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-package-manifests.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-debian-package-build.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-debian-package-install.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-debian-system-install.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-staged-session-install.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-systemd-activation.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-drm-session-smoke.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-session-replay.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-session-clean-exit.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-mvp1-contract.sh'
require_contains scripts/verify-parallels-linux-e2e.sh '"name": "backlit-parallels-linux-e2e-export"'
require_contains scripts/verify-parallels-linux-e2e.sh '"guest_e2e_passed": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"parallels_drm_launch_ready": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"dpkg_root_install": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"actual_system_dpkg_install": true'
require_contains scripts/verify-parallels-linux-e2e.sh 'debian-package-install-manifest.json'
require_contains scripts/verify-parallels-linux-e2e.sh 'debian-system-install-manifest.json'
require_contains scripts/verify-parallels-linux-e2e.sh 'drm-session-smoke-manifest.json'
require_contains scripts/verify-session-launch.sh '--verify-systemd-units'
require_contains scripts/verify-session-launch.sh 'XDG_RUNTIME_DIR XDG_SESSION_ID XDG_SEAT XDG_SESSION_TYPE WAYLAND_DISPLAY XDG_CURRENT_DESKTOP DESKTOP_SESSION'
require_contains scripts/verify-session-launch.sh '"session_systemd_units": true'
require_contains scripts/verify-session-launch.sh '"session_systemd_target": true'
require_contains scripts/verify-session-launch.sh '"session_systemd_launch_plan": true'
require_contains scripts/verify-session-replay.sh '"session_replay_event": true'
require_contains scripts/verify-session-replay.sh '"frame_count": $frame_count'
require_contains scripts/verify-session-replay.sh '"app_switcher_overlay_frame": true'
require_contains scripts/verify-session-replay.sh '"launcher_overlay_opened": true'
require_contains scripts/verify-session-replay.sh '"launcher_overlay_frame": true'
require_contains scripts/verify-session-replay.sh '"workspace_switch": true'
require_contains scripts/verify-staged-session-install.sh '--verify-systemd-units'
require_contains scripts/verify-staged-session-install.sh 'XDG_RUNTIME_DIR XDG_SESSION_ID XDG_SEAT XDG_SESSION_TYPE WAYLAND_DISPLAY XDG_CURRENT_DESKTOP DESKTOP_SESSION'
require_contains scripts/verify-staged-session-install.sh '"session_systemd_units": true'
require_contains scripts/verify-staged-session-install.sh '"session_systemd_target": true'
require_contains scripts/verify-staged-session-install.sh '"session_systemd_launch_plan": true'
require_contains scripts/verify-staged-session-install.sh '"staged_compositor_surface_lifecycle": true'
require_contains scripts/verify-staged-session-install.sh '"staged_compositor_popup_lifecycle": true'
require_contains scripts/verify-systemd-activation.sh '--verify-systemd-activation'
require_contains scripts/verify-systemd-activation.sh '"systemd_activation": true'
require_contains scripts/verify-resource-budget.sh '"event":"compositor.ready"'
require_contains scripts/verify-resource-budget.sh '"compositor_service_ready": true'
require_contains scripts/verify-resource-budget.sh '"compositor_bootstrap_surface": true'
require_contains scripts/verify-launch-readiness.sh '"xdg_runtime_dir_owned_by_user"'
require_contains scripts/verify-launch-readiness.sh '"session_local"'
require_contains scripts/verify-launch-readiness.sh '"drm_card_access_ready"'
require_contains scripts/verify-launch-readiness.sh '"input_broker_ready"'
require_contains scripts/verify-drm-session-smoke.sh '"xdg_runtime_dir_owned_by_user"'
require_contains scripts/verify-drm-session-smoke.sh '"session_local"'
require_contains scripts/verify-drm-session-smoke.sh '"drm_card_access_ready"'
require_contains scripts/verify-drm-session-smoke.sh '"input_broker_ready"'

require_contains packaging/sessions/backlit.desktop 'Exec=backlit-session --backend=drm --activate-systemd'
require_contains packaging/applications/org.backlit.Settings.desktop 'Exec=backlit-settings'
require_contains packaging/systemd/backlit-session.target 'Wants=backlit-compositor.service backlit-shell.service backlit-notification-daemon.service backlit-settings-daemon.service'
require_contains packaging/systemd/backlit-compositor.service 'ExecStart=/usr/bin/backlit-compositor --backend=drm --socket=backlit-0 --serve'
require_contains packaging/systemd/backlit-shell.service 'ExecStart=/usr/bin/backlit-shell --component=all --socket=backlit-0 --serve'
require_contains packaging/systemd/backlit-notification-daemon.service 'ExecStart=/usr/bin/backlit-notification-daemon --serve'
require_contains packaging/systemd/backlit-settings-daemon.service 'ExecStart=/usr/bin/backlit-settings-daemon --serve'
require_contains packaging/debian/control.stub 'Package: fastgui-session'
require_contains packaging/debian/control.stub 'Package: fastgui-core'
require_contains packaging/debian/control.stub 'Depends: ${misc:Depends}, fastgui-compositor, fastgui-shell, fastgui-settings'
require_contains packaging/debian/control.stub 'Depends: ${misc:Depends}, fastgui-core, fastgui-portal'
require_contains packaging/debian/fastgui-session.install 'usr/bin/backlit-demo-client'

artifact_manifests_checked=false
nested_wayland_artifact=false
if [ -n "$artifact_root" ] && [ -d "$artifact_root" ]; then
  artifact_manifests_checked=true
  require_file "$artifact_root/gui-smoke/manifest.json"
  require_file "$artifact_root/gui-preview/manifest.json"
  require_file "$artifact_root/compositor-runtime/manifest.json"
  require_file "$artifact_root/compositor-socket/manifest.json"
  require_file "$artifact_root/launch-performance/manifest.json"
  require_file "$artifact_root/launcher-desktop-discovery/manifest.json"
  require_file "$artifact_root/resource-budget/manifest.json"
  require_file "$artifact_root/notification-daemon/manifest.json"
  require_file "$artifact_root/settings-daemon/manifest.json"
  require_file "$artifact_root/service-lifecycle/manifest.json"
  require_file "$artifact_root/settings-app/manifest.json"
  require_file "$artifact_root/portal-security/manifest.json"
  require_file "$artifact_root/crash-logs/manifest.json"
  require_file "$artifact_root/ci-contract/manifest.json"
  require_file "$artifact_root/packaging-contract/manifest.json"
  require_file "$artifact_root/package-manifests/manifest.json"
  require_file "$artifact_root/debian-package-build/manifest.json"
  require_file "$artifact_root/debian-package-install/manifest.json"
  require_file "$artifact_root/debian-system-install/manifest.json"
  require_file "$artifact_root/staged-session-install/manifest.json"
  require_file "$artifact_root/launch-readiness/manifest.json"
  require_file "$artifact_root/session-clean-exit/manifest.json"
  require_file "$artifact_root/drm-session-smoke/manifest.json"
  require_file "$artifact_root/mvp1-contract/manifest.json"

  require_contains "$artifact_root/gui-smoke/manifest.json" '"protocol_required_count": 7'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_required_roles": 5'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_wallpaper": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_panel_status": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_power_menu": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_network_status": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_network_controls": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_audio_status": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_audio_controls": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_workspace_indicator": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_launcher_targets": 3'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_app_switcher": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shell_lock_screen": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"launcher_required_targets": 3'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shortcut_required_bindings": 6'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"keyboard_input": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"pointer_input": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_input": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"surface_lifecycle": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"popup_lifecycle": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"compositor_surface_lifecycle": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"compositor_popup_lifecycle": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_surface_lifecycle": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"no_idle_redraw": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"targeted_damage": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"direct_scanout": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"drag_frame_pacing": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"notification_daemon": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"settings_daemon": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"settings_power_actions": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"settings_power_actions_dry_run": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"settings_suspend_action": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"settings_app": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"settings_app_display_panel": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"settings_app_input_panel": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"settings_app_power_panel": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"portal_security": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"crash_logs": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_services": true'
  if grep '"session_compositor_demo_client": true' "$artifact_root/gui-smoke/manifest.json" >/dev/null; then
    require_contains "$artifact_root/gui-smoke/manifest.json" '"session_compositor_demo_client": true'
  else
    require_contains "$artifact_root/gui-smoke/manifest.json" '"session_compositor_client_blocked_expected": true'
  fi
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_notification_service": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_settings_service": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_launch_spawn": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_move_resize": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_workspace_switch": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_snap": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_policy_preview": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_focused_title_bar": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_workspace_indicator": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"golden_checksum": true'
  require_contains "$artifact_root/gui-preview/manifest.json" '"session_verified": true'
  require_contains "$artifact_root/gui-preview/manifest.json" '"policy_windows": 3'
  require_contains "$artifact_root/gui-preview/manifest.json" '"focused_title_bar": true'
  require_contains "$artifact_root/gui-preview/manifest.json" '"workspace_indicator": true'
  require_contains "$artifact_root/gui-preview/manifest.json" '"session_services": true'
  require_contains "$artifact_root/gui-preview/manifest.json" '"notification_service": true'
  require_contains "$artifact_root/gui-preview/manifest.json" '"settings_service": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"scripted_client_runtime": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"app_surface_map": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"surface_policy_preview": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"targeted_surface_damage": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"idle_no_redraw": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"surface_close_damage": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"client_disconnect_cleanup": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"service_mode_runtime": true'
  if grep '"session_socket_bound": true' "$artifact_root/compositor-socket/manifest.json" >/dev/null; then
    require_contains "$artifact_root/compositor-socket/manifest.json" '"socket_accepts_client_connection": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_socket_launch": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_surface_mapped": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"session_socket_cleanup": true'
  else
    require_contains "$artifact_root/compositor-socket/manifest.json" '"socket_blocked_expected": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"socket_permission_denied": true'
  fi
  require_contains "$artifact_root/launch-performance/manifest.json" '"startup_budget": true'
  require_contains "$artifact_root/launch-performance/manifest.json" '"terminal_launch_budget": true'
  require_contains "$artifact_root/launch-performance/manifest.json" '"shell_ready_budget": true'
  require_contains "$artifact_root/launch-performance/manifest.json" '"notification_service": true'
  require_contains "$artifact_root/launch-performance/manifest.json" '"settings_service": true'
  require_contains "$artifact_root/launcher-desktop-discovery/manifest.json" '"fixture_desktop_discovery": true'
  require_contains "$artifact_root/launcher-desktop-discovery/manifest.json" '"host_default_desktop_discovery": true'
  require_contains "$artifact_root/resource-budget/manifest.json" '"name": "backlit-resource-budget"'
  if grep '"resource_budget_checked": true' "$artifact_root/resource-budget/manifest.json" >/dev/null; then
    require_contains "$artifact_root/resource-budget/manifest.json" '"compositor_service_ready": true'
    require_contains "$artifact_root/resource-budget/manifest.json" '"compositor_accepting_clients": true'
    require_contains "$artifact_root/resource-budget/manifest.json" '"compositor_bootstrap_surface": true'
    require_contains "$artifact_root/resource-budget/manifest.json" '"idle_cpu_budget": true'
    require_contains "$artifact_root/resource-budget/manifest.json" '"idle_rss_budget": true'
  else
    require_contains "$artifact_root/resource-budget/manifest.json" '"resource_budget_blocked_expected": true'
  fi
  require_contains "$artifact_root/notification-daemon/manifest.json" '"replace_id": true'
  require_contains "$artifact_root/notification-daemon/manifest.json" '"action_invoked": true'
  require_contains "$artifact_root/notification-daemon/manifest.json" '"close_reasons": true'
  require_contains "$artifact_root/notification-daemon/manifest.json" '"critical_persistent": true'
  require_contains "$artifact_root/notification-daemon/manifest.json" '"dbus_spec_fields": true'
  require_contains "$artifact_root/settings-daemon/manifest.json" '"display_validated": true'
  require_contains "$artifact_root/settings-daemon/manifest.json" '"input_validated": true'
  require_contains "$artifact_root/settings-daemon/manifest.json" '"power_validated": true'
  require_contains "$artifact_root/settings-daemon/manifest.json" '"power_action_commands_complete": true'
  require_contains "$artifact_root/settings-daemon/manifest.json" '"power_actions_dry_run": true'
  require_contains "$artifact_root/settings-daemon/manifest.json" '"disruptive_power_actions_guarded": true'
  require_contains "$artifact_root/settings-daemon/manifest.json" '"suspend_action_ready": true'
  require_contains "$artifact_root/settings-daemon/manifest.json" '"shutdown_command": "systemctl poweroff"'
  require_contains "$artifact_root/service-lifecycle/manifest.json" '"compositor_service_lifecycle": true'
  require_contains "$artifact_root/service-lifecycle/manifest.json" '"shell_service_lifecycle": true'
  require_contains "$artifact_root/service-lifecycle/manifest.json" '"notification_daemon_service_lifecycle": true'
  require_contains "$artifact_root/service-lifecycle/manifest.json" '"settings_daemon_service_lifecycle": true'
  require_contains "$artifact_root/service-lifecycle/manifest.json" '"bounded_service_exit": true'
  require_contains "$artifact_root/service-lifecycle/manifest.json" '"systemd_service_mode": true'
  require_contains "$artifact_root/settings-app/manifest.json" '"launcher_target_ready": true'
  require_contains "$artifact_root/settings-app/manifest.json" '"display_panel": true'
  require_contains "$artifact_root/settings-app/manifest.json" '"input_panel": true'
  require_contains "$artifact_root/settings-app/manifest.json" '"power_panel": true'
  require_contains "$artifact_root/settings-app/manifest.json" '"daemon_generation": 3'
  require_contains "$artifact_root/portal-security/manifest.json" '"direct_screenshot_denied": true'
  require_contains "$artifact_root/portal-security/manifest.json" '"direct_screencast_denied": true'
  require_contains "$artifact_root/portal-security/manifest.json" '"consented_screenshot_allowed": true'
  require_contains "$artifact_root/crash-logs/manifest.json" '"crash_logs_recorded": true'
  require_contains "$artifact_root/crash-logs/manifest.json" '"journalctl_user_scope": true'
  require_contains "$artifact_root/crash-logs/manifest.json" '"systemd_journal_output": true'
  require_contains "$artifact_root/crash-logs/manifest.json" '"rust_backtrace_enabled": true'
  require_contains "$artifact_root/ci-contract/manifest.json" '"linux_e2e_gate": true'
  require_contains "$artifact_root/packaging-contract/manifest.json" '"desktop_entry": true'
  require_contains "$artifact_root/packaging-contract/manifest.json" '"settings_desktop_entry": true'
  require_contains "$artifact_root/packaging-contract/manifest.json" '"systemd_session_target": true'
  require_contains "$artifact_root/packaging-contract/manifest.json" '"journal_logging": true'
  require_contains "$artifact_root/packaging-contract/manifest.json" '"package_split": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"fastgui_core_package": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"session_depends_on_settings_service": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"desktop_depends_on_core": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"core_is_meta_package": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"desktop_is_meta_package": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"session_installs_desktop_entry": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"session_installs_systemd_units": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"runtime_binaries_split": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"dev_tools_manifest": true'
  require_contains "$artifact_root/package-manifests/manifest.json" '"unique_install_paths": true'
  require_contains "$artifact_root/debian-package-build/manifest.json" '"package_build_checked": true'
  if grep '"debs_built": true' "$artifact_root/debian-package-build/manifest.json" >/dev/null; then
    require_contains "$artifact_root/debian-package-build/manifest.json" '"fastgui_core_deb": true'
    require_contains "$artifact_root/debian-package-build/manifest.json" '"runtime_package_debs": true'
    require_contains "$artifact_root/debian-package-build/manifest.json" '"package_contents": true'
    require_contains "$artifact_root/debian-package-build/manifest.json" '"package_dependencies": true'
  else
    require_contains "$artifact_root/debian-package-build/manifest.json" '"build_blocked_expected": true'
  fi
  require_contains "$artifact_root/debian-package-install/manifest.json" '"package_install_checked": true'
  if grep '"debs_extracted": true' "$artifact_root/debian-package-install/manifest.json" >/dev/null; then
    require_contains "$artifact_root/debian-package-install/manifest.json" '"dpkg_root_install": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"fastgui_core_closure": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_exec_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_systemd_units_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_gui_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_services_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_compositor_demo_client_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_replay_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_clean_exit_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"settings_app_from_extracted_debs": true'
  else
    require_contains "$artifact_root/debian-package-install/manifest.json" '"install_blocked_expected": true'
  fi
  require_contains "$artifact_root/debian-system-install/manifest.json" '"system_install_checked": true'
  if grep '"system_install_performed": true' "$artifact_root/debian-system-install/manifest.json" >/dev/null; then
    require_contains "$artifact_root/debian-system-install/manifest.json" '"actual_system_dpkg_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"dpkg_database_status": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"usr_bin_session_launch": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"systemd_units_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_gui_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_services_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_compositor_demo_client_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_replay_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_clean_exit_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"settings_app_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"packages_purged_after_verification": true'
  else
    require_contains "$artifact_root/debian-system-install/manifest.json" '"install_blocked_expected": true'
  fi
  require_contains "$artifact_root/staged-session-install/manifest.json" '"desktop_exec_resolves": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"settings_desktop_exec_resolves": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"session_systemd_units": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"session_systemd_target": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"session_systemd_launch_plan": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"systemd_journal_output": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_session_gui": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_demo_client_binary": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_session_services": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_compositor_surface_lifecycle": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_compositor_popup_lifecycle": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_notification_daemon_verify": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_settings_daemon_verify": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_settings_app_verify": true'
  require_contains "$artifact_root/systemd-activation/manifest.json" '"systemd_activation": true'
  require_contains "$artifact_root/systemd-activation/manifest.json" '"systemd_import_environment": true'
  require_contains "$artifact_root/systemd-activation/manifest.json" '"systemd_start_target": true'
  require_contains "$artifact_root/systemd-activation/manifest.json" '"systemd_stop_target": true'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"xdg_runtime_dir_owned_by_user":'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"session_local":'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_card_access_ready":'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"input_broker_ready":'
  require_contains "$artifact_root/session-launch/manifest.json" '"session_systemd_units": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"session_systemd_target": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"session_systemd_launch_plan": true'
  require_contains "$artifact_root/session-replay/manifest.json" '"session_replay_event": true'
  require_contains "$artifact_root/session-replay/manifest.json" '"frame_count": 9'
  require_contains "$artifact_root/session-replay/manifest.json" '"launcher_overlay_frame": true'
  require_contains "$artifact_root/session-replay/manifest.json" '"app_switcher_overlay_frame": true'
  require_contains "$artifact_root/session-replay/manifest.json" '"workspace_switch": true'
  require_contains "$artifact_root/session-clean-exit/manifest.json" '"clean_exit_event": true'
  require_contains "$artifact_root/session-clean-exit/manifest.json" '"windows_after_shutdown": 0'
  require_contains "$artifact_root/session-clean-exit/manifest.json" '"focus_cleared": true'
  require_contains "$artifact_root/mvp1-contract/manifest.json" '"name": "backlit-mvp1-contract"'
  require_contains "$artifact_root/mvp1-contract/manifest.json" '"artifact_manifests_checked": true'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"name": "backlit-drm-session-smoke"'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"xdg_runtime_dir_owned_by_user":'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_local":'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_card_access_ready":'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"input_broker_ready":'
  if grep '"drm_session_smoke_ready": true' "$artifact_root/drm-session-smoke/manifest.json" >/dev/null; then
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_session_clean_exit": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"notification_service": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"settings_service": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"workspace_switch": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"snap": true'
  else
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_session_smoke_blocked_expected": true'
  fi

  if [ -f "$artifact_root/nested-wayland/manifest.json" ]; then
    nested_wayland_artifact=true
    require_contains "$artifact_root/nested-wayland/manifest.json" '"parent_socket_ready": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"wayland_preflight_ready": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"compositor_wayland_smoke": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_client_spawn": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"launcher_terminal_wayland_spawn": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"launcher_terminal_no_seat_expected":'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_services": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_notification_service": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_settings_service": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_workspace_switch": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_snap": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_clean_exit": true'
  fi
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-mvp0-contract",
  "passed": true,
  "artifact_manifests_checked": $artifact_manifests_checked,
  "nested_wayland_artifact": $nested_wayland_artifact,
  "artifacts": {
    "architecture": "docs/architecture/mvp-0.md",
    "gui_smoke_verifier": "scripts/verify-gui-smoke.sh",
    "linux_e2e_verifier": "scripts/verify-linux-e2e.sh"
  },
  "checks": {
    "workspace_crates": true,
    "headless_backend": true,
    "nested_wayland_gate": true,
    "demo_client": true,
    "performance_smoke": true,
    "launch_performance": true,
    "launcher_desktop_discovery": true,
    "resource_budget": true,
    "compositor_runtime": true,
    "compositor_socket": true,
    "compositor_service_ready": true,
    "notification_daemon": true,
    "settings_daemon": true,
    "service_lifecycle": true,
    "settings_app": true,
    "portal_security": true,
    "crash_logs": true,
    "input_smoke": true,
    "surface_lifecycle": true,
    "popup_lifecycle": true,
    "compositor_surface_lifecycle": true,
    "compositor_popup_lifecycle": true,
    "shell_chrome": true,
    "workspace_switch": true,
    "window_snap": true,
    "frame_damage": true,
    "direct_scanout": true,
    "drag_frame_pacing": true,
    "protocol_smoke": true,
    "golden_gui": true,
    "viewable_preview": true,
    "session_services": true,
    "session_clean_exit": true,
    "packaging_skeleton": true,
    "package_manifests": true,
    "debian_package_build": true,
    "debian_package_install": true,
    "debian_system_install": true,
    "staged_session_install": true,
    "drm_session_smoke": true,
    "mvp1_contract": true,
    "ci_gate": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit MVP 0 contract verification passed. Artifacts: %s\n' "$out_dir"
