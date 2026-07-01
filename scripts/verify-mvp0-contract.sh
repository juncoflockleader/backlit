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

require_matches() {
  file="$1"
  value="$2"
  grep -E "$value" "$file" >/dev/null || fail "missing pattern in $file: $value"
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
require_executable scripts/verify-smithay-runtime-probe.sh
require_executable scripts/verify-drm-master-boundary.sh
require_executable scripts/verify-dedicated-drm-session.sh
require_executable scripts/verify-smithay-compositor-runtime.sh
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
require_contains scripts/verify-gui-smoke.sh '--desktop-entry=org.backlit.SpawnProbe.desktop'
require_contains scripts/verify-gui-smoke.sh '"launcher_desktop_spawn": true'
require_contains scripts/verify-gui-smoke.sh '--verify-desktop-launch'
require_contains scripts/verify-gui-smoke.sh '"session_desktop_launch": true'
require_contains scripts/verify-gui-smoke.sh '"session_desktop_managed_window": true'
require_contains scripts/verify-gui-smoke.sh '"session_compositor_demo_app_id_preserved": $session_compositor_demo_client'
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
require_contains scripts/verify-compositor-runtime.sh '"runtime_backend_contract": true'
require_contains scripts/verify-compositor-runtime.sh '"runtime_backend": "headless-compositor"'
require_contains scripts/verify-compositor-runtime.sh '"runtime_trait": true'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-compositor-socket.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-smithay-runtime-probe.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-drm-master-boundary.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-smithay-compositor-runtime.sh'
require_contains scripts/verify-smithay-runtime-probe.sh '--features smithay-backend'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_dependency_compiled": true'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_runtime_probe": $smithay_runtime_probe'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_runtime_bootstrap": $smithay_runtime_bootstrap'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_wayland_socket_bootstrap": $smithay_wayland_socket_bootstrap'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_wayland_client_inserted": $smithay_wayland_client_inserted'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_libinput_pointer_event_count": $smithay_libinput_pointer_event_count'
require_contains scripts/verify-drm-master-boundary.sh '"name": "backlit-drm-master-boundary"'
require_contains scripts/verify-drm-master-boundary.sh '"session_entry_drm": true'
require_contains scripts/verify-drm-master-boundary.sh '"compositor_service_drm": true'
require_contains scripts/verify-drm-master-boundary.sh '"compositor_service_smithay_runtime": true'
require_contains scripts/verify-drm-master-boundary.sh '"dedicated_session_model": "seat-owner-tty-or-display-manager-session"'
require_contains scripts/verify-dedicated-drm-session.sh '"name": "backlit-dedicated-drm-session"'
require_contains scripts/verify-dedicated-drm-session.sh '--require-drm-master-present'
require_contains scripts/verify-dedicated-drm-session.sh '"dedicated_handoff_plan": true'
require_contains scripts/verify-dedicated-drm-session.sh '"dedicated_session_acceptance": $dedicated_session_acceptance'
require_contains scripts/verify-smithay-compositor-runtime.sh '--features smithay-backend'
require_contains scripts/verify-smithay-compositor-runtime.sh '--runtime=smithay'
require_contains scripts/verify-smithay-compositor-runtime.sh '--drm-first-present-probe'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_compositor_runtime": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_runtime_launch_plan": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_core_protocol_globals": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_seat_global": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_keyboard_pointer_capabilities": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_input_sources": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_input_event_loop": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_input_seat_handles": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_input_seat_dispatch": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_input_event_classification": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_wayland_client": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_wayland_metadata": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_shm_buffer": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_wayland_policy_window": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_drm_first_present_probe": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_service_socket": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_service_socket_runtime_trait": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_demo_client_socket_lifecycle": true'
require_contains crates/compositor/src/main.rs '"compositor.drm_first_present_probe"'
require_contains scripts/verify-compositor-socket.sh '"demo_client_socket_launch": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_surface_mapped": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_surface_damaged": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_surface_closed": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_window_moved": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_window_resized": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_window_maximized": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_window_fullscreen": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_window_removed": true'
require_contains scripts/verify-compositor-socket.sh '"demo_client_disconnected": true'
require_contains scripts/verify-compositor-socket.sh '"multi_client_windows_mapped": true'
require_contains scripts/verify-compositor-socket.sh '"new_client_focused": true'
require_contains scripts/verify-compositor-socket.sh '"close_fallback_focus": true'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-launch-performance.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-launcher-desktop-discovery.sh'
require_contains scripts/verify-launcher-desktop-discovery.sh '--desktop-entry=org.backlit.SpawnProbe.desktop'
require_contains scripts/verify-launcher-desktop-discovery.sh '"fixture_desktop_spawn": true'
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
require_contains scripts/verify-debian-package-build.sh 'cargo build -p backlit-compositor --features smithay-backend'
require_contains scripts/verify-debian-package-build.sh 'cargo build -p backlit-session --features smithay-backend'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-debian-package-install.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-debian-system-install.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-staged-session-install.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-systemd-activation.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-drm-session-smoke.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-dedicated-drm-session.sh'
require_contains scripts/verify-linux-e2e.sh '"dedicated_drm_handoff": true'
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
require_contains scripts/verify-parallels-linux-e2e.sh 'dedicated-drm-session-manifest.json'
require_contains scripts/verify-session-launch.sh '--verify-systemd-units'
require_contains scripts/verify-session-launch.sh 'XDG_RUNTIME_DIR XDG_SESSION_ID XDG_SEAT XDG_SESSION_TYPE WAYLAND_DISPLAY XDG_CURRENT_DESKTOP DESKTOP_SESSION'
require_contains scripts/verify-session-launch.sh '"session_systemd_units": true'
require_contains scripts/verify-session-launch.sh '"session_systemd_target": true'
require_contains scripts/verify-session-launch.sh '"session_systemd_launch_plan": true'
require_contains scripts/verify-session-launch.sh '"session.backend_launch_plan"'
require_contains scripts/verify-session-launch.sh '"drm_backend_launch_plan": true'
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
require_contains scripts/verify-launch-readiness.sh '"backend.launch_plan"'
require_contains scripts/verify-launch-readiness.sh '"drm_launch_plan": true'
require_contains scripts/verify-drm-session-smoke.sh '"xdg_runtime_dir_owned_by_user"'
require_contains scripts/verify-drm-session-smoke.sh '"session_local"'
require_contains scripts/verify-drm-session-smoke.sh '"drm_card_access_ready"'
require_contains scripts/verify-drm-session-smoke.sh '"input_broker_ready"'
require_contains scripts/verify-drm-session-smoke.sh '"session.backend_launch_plan"'
require_contains scripts/verify-drm-session-smoke.sh '"drm_backend_launch_plan": true'
require_contains scripts/verify-drm-session-smoke.sh '--verify-drm-first-present'
require_contains scripts/verify-drm-session-smoke.sh '"session_drm_first_present_probe": $session_drm_first_present_probe'
require_contains scripts/verify-drm-session-smoke.sh '"session_desktop_launch": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_desktop_managed_window": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_demo_app_id_preserved": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_runtime": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_protocol_globals": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_input_sources": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_input_event_loop": $drm_session_smoke_ready'

require_contains packaging/sessions/backlit.desktop 'Exec=backlit-session --backend=drm --activate-systemd'
require_contains packaging/applications/org.backlit.Settings.desktop 'Exec=backlit-settings'
require_contains packaging/systemd/backlit-session.target 'Wants=backlit-compositor.service backlit-shell.service backlit-notification-daemon.service backlit-settings-daemon.service'
require_contains packaging/systemd/backlit-compositor.service 'ExecStart=/usr/bin/backlit-compositor --backend=drm --runtime=smithay --socket=backlit-0 --serve'
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
  require_file "$artifact_root/smithay-compositor-runtime/manifest.json"
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
  require_file "$artifact_root/smithay-runtime-probe/manifest.json"
  require_file "$artifact_root/drm-master-boundary/manifest.json"
  require_file "$artifact_root/launch-readiness/manifest.json"
  require_file "$artifact_root/session-clean-exit/manifest.json"
  require_file "$artifact_root/drm-session-smoke/manifest.json"
  require_file "$artifact_root/dedicated-drm-session/manifest.json"
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
  require_contains "$artifact_root/gui-smoke/manifest.json" '"launcher_desktop_spawn": true'
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
	    require_contains "$artifact_root/gui-smoke/manifest.json" '"session_compositor_demo_app_id_preserved": true'
	  else
	    require_contains "$artifact_root/gui-smoke/manifest.json" '"session_compositor_client_blocked_expected": true'
  fi
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_notification_service": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_settings_service": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_launch_spawn": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_desktop_launch": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_desktop_managed_window": true'
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
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"runtime_backend_contract": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"runtime_backend": "headless-compositor"'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"runtime_trait": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"app_surface_map": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"surface_policy_preview": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"targeted_surface_damage": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"idle_no_redraw": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"surface_close_damage": true'
	  require_contains "$artifact_root/compositor-runtime/manifest.json" '"client_disconnect_cleanup": true'
	  require_contains "$artifact_root/compositor-runtime/manifest.json" '"service_mode_runtime": true'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"name": "backlit-smithay-compositor-runtime"'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_compositor_runtime":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_runtime_trait":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_runtime_launch_plan":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_scripted_client":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_core_protocol_globals":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_seat_global":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_keyboard_pointer_capabilities":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_sources":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_event_loop":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_seat_handles":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_seat_dispatch":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_event_classification":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_wayland_client":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_wayland_metadata":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_shm_buffer":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_wayland_policy_window":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_event_loop_runtime":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_drm_first_present_probe":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_service_ready":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_event_loop_service_socket":'
  if grep '"checked": true' "$artifact_root/smithay-compositor-runtime/manifest.json" >/dev/null; then
    if grep '"drm_launch_ready": true' "$artifact_root/smithay-compositor-runtime/manifest.json" >/dev/null; then
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_compositor_runtime": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_runtime_trait": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_runtime_launch_plan": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_scripted_client": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_core_protocol_globals": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_seat_global": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_keyboard_pointer_capabilities": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_sources": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_event_loop": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_seat_handles": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_seat_dispatch": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_input_event_classification": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_wayland_client": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_wayland_metadata": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_shm_buffer": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_wayland_policy_window": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_event_loop_runtime": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_drm_first_present_probe": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_service_ready": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_service_socket": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_service_socket_runtime_trait": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_event_loop_service_socket": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_demo_client_socket_lifecycle": true'
    else
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"expected_blocked": true'
    fi
  else
    require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"expected_blocked": true'
  fi
	  require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"name": "backlit-smithay-runtime-probe"'
  require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_runtime_probe":'
  require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_runtime_bootstrap":'
  require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_wayland_display_bootstrap":'
  require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_wayland_socket_bootstrap":'
  require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_wayland_client_inserted":'
  require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_calloop_dispatch_bootstrap":'
  if grep '"checked": true' "$artifact_root/smithay-runtime-probe/manifest.json" >/dev/null; then
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_dependency_compiled": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_backend_feature": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_drm_component": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_gbm_allocator_component": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_egl_display_component": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_gles_renderer_component": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_card_opened":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_device_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_event_source_inserted":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_event_loop_dispatched":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_atomic_modesetting":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_crtc_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_connector_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_connected_connector_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_mode_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_primary_plane_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_cursor_plane_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_overlay_plane_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_plan_ready":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_connector_id":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_crtc_id":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_primary_plane_id":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_mode_width":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_mode_height":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_mode_refresh_hz":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_mode_preferred":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_legacy":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_crtc_matches_plan":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_primary_plane_matches_plan":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_pending_connector_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_current_connector_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_pending_mode_matches_plan":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_commit_pending":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_dropped_after_pause":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_added":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_test_state_succeeded":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_test_state_permission_denied":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_test_allow_modeset":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_primary_plane_matches_surface":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_width":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_height":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_released_before_surface_drop":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_framebuffer_filled":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_plane_state_ready":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_commit_attempted":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_commit_succeeded":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_vblank_event_received":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_blocked_by_drm_master":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_renderer_node_opened":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_gbm_device_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_gbm_allocator_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_egl_display_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_egl_context_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_gles_renderer_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_buffer_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_frame_rendered":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_frame_copied":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_pixel_verified":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libseat_session_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libseat_event_source_inserted":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libseat_event_loop_dispatched":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_context_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_seat_assigned":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_backend_created":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_event_source_inserted":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_event_loop_dispatched":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_event_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_keyboard_event_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_pointer_event_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_special_event_count":'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_component": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libseat_session_component": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_calloop_component": true'
    if grep '"drm_launch_ready": true' "$artifact_root/smithay-runtime-probe/manifest.json" >/dev/null; then
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_runtime_probe": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_runtime_bootstrap": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_drm_node_resolved": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_card_opened": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_device_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_event_source_inserted": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_event_loop_dispatched": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_crtc_count": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_connector_count": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_connected_connector_count": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_mode_count": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_primary_plane_count": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_cursor_plane_count": [0-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_overlay_plane_count": [0-9][0-9]*'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_plan_ready": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_connector_id": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_crtc_id": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_primary_plane_id": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_mode_width": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_mode_height": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_mode_refresh_hz": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_scanout_mode_preferred": (true|false)'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_created": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_legacy": (true|false)'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_crtc_matches_plan": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_primary_plane_matches_plan": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_pending_connector_count": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_current_connector_count": [0-9][0-9]*'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_pending_mode_matches_plan": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_commit_pending": (true|false)'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_surface_dropped_after_pause": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_added": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_test_state_succeeded": (true|false)'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_test_state_permission_denied": (true|false)'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_(test_state_succeeded|test_state_permission_denied)": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_test_allow_modeset": (true|false)'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_primary_plane_matches_surface": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_width": [1-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_height": [1-9][0-9]*'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_framebuffer_released_before_surface_drop": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_framebuffer_filled": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_plane_state_ready": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_commit_attempted": (true|false)'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_commit_succeeded": (true|false)'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_vblank_event_received": (true|false)'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_blocked_by_drm_master": (true|false)'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_(commit_succeeded|blocked_by_drm_master)": true'
      if grep -F '"smithay_kms_first_present_commit_succeeded": true' "$artifact_root/smithay-runtime-probe/manifest.json" >/dev/null; then
        require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_kms_first_present_vblank_event_received": true'
      fi
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_renderer_node_selected": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_renderer_node_opened": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_gbm_device_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_gbm_allocator_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_egl_display_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_egl_context_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_gles_renderer_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_buffer_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_frame_rendered": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_frame_copied": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_pixel_verified": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_render_width": 16'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_render_height": 16'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_render_pixels": 256'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_sample_red": 255'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_sample_green": 0'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_sample_blue": 0'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_offscreen_sample_alpha": 255'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libseat_session_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libseat_event_source_inserted": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libseat_event_loop_dispatched": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_context_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_seat_assigned": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_backend_created": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_event_source_inserted": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_event_loop_dispatched": true'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_event_count": [0-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_keyboard_event_count": [0-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_pointer_event_count": [0-9][0-9]*'
      require_matches "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_special_event_count": [0-9][0-9]*'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_wayland_display_bootstrap": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_wayland_socket_bootstrap": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_wayland_client_inserted": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_calloop_dispatch_bootstrap": true'
    else
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"expected_blocked": true'
    fi
  else
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"expected_blocked": true'
  fi
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"name": "backlit-drm-master-boundary"'
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"session_entry_drm": true'
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"compositor_service_drm": true'
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"compositor_service_smithay_runtime": true'
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"mutating_handoff_attempted": false'
  if grep '"drm_launch_ready": true' "$artifact_root/drm-master-boundary/manifest.json" >/dev/null; then
    require_contains "$artifact_root/drm-master-boundary/manifest.json" '"drm_master_boundary_checked": true'
    require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_framebuffer_filled": true'
    require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_plane_state_ready": true'
    require_matches "$artifact_root/drm-master-boundary/manifest.json" '"first_present_(commit_succeeded|blocked_by_drm_master)": true'
    if grep '"current_session_can_present": true' "$artifact_root/drm-master-boundary/manifest.json" >/dev/null; then
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_commit_succeeded": true'
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_vblank_event_received": true'
    else
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_blocked_by_drm_master": true'
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"drm_master_boundary_observed": true'
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"dedicated_session_required": true'
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"current_session_can_present": false'
    fi
  else
    require_contains "$artifact_root/drm-master-boundary/manifest.json" '"expected_blocked": true'
  fi
  if grep '"session_socket_bound": true' "$artifact_root/compositor-socket/manifest.json" >/dev/null; then
    require_contains "$artifact_root/compositor-socket/manifest.json" '"socket_accepts_client_connection": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_socket_launch": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_surface_mapped": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_surface_damaged": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_surface_closed": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_window_moved": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_window_resized": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_window_maximized": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_window_fullscreen": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_window_removed": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"demo_client_disconnected": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"multi_client_windows_mapped": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"new_client_focused": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"close_fallback_focus": true'
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
  require_contains "$artifact_root/launcher-desktop-discovery/manifest.json" '"fixture_desktop_spawn": true'
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
    require_contains "$artifact_root/debian-package-build/manifest.json" '"compositor_smithay_feature_build": true'
    require_contains "$artifact_root/debian-package-build/manifest.json" '"session_smithay_feature_build": true'
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
	    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_compositor_demo_app_id_from_extracted_debs": true'
	    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_desktop_launch_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_desktop_managed_window_from_extracted_debs": true'
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
	    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_compositor_demo_app_id_from_system_install": true'
	    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_desktop_launch_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_desktop_managed_window_from_system_install": true'
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
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_session_desktop_launch": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_session_desktop_managed_window": true'
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
  require_contains "$artifact_root/launch-readiness/manifest.json" '"headless_launch_plan": true'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_launch_plan": true'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"session_local":'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_card_access_ready":'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"input_broker_ready":'
  require_contains "$artifact_root/session-launch/manifest.json" '"session_systemd_units": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"session_systemd_target": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"session_systemd_launch_plan": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"headless_backend_launch_plan": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"drm_backend_launch_plan": true'
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
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_backend_launch_plan": true'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"xdg_runtime_dir_owned_by_user":'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_local":'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_card_access_ready":'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"input_broker_ready":'
  if grep '"drm_session_smoke_ready": true' "$artifact_root/drm-session-smoke/manifest.json" >/dev/null; then
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_session_clean_exit": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_device_selected": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_input_selected": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_drm_first_present_probe": true'
    require_matches "$artifact_root/drm-session-smoke/manifest.json" '"session_first_present_(commit_succeeded|blocked_by_drm_master)": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"notification_service": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"settings_service": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_runtime": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_protocol_globals": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_input_sources": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_input_event_loop": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_demo_app_id_preserved": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_desktop_launch": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_desktop_managed_window": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"workspace_switch": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"snap": true'
  else
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_session_smoke_blocked_expected": true'
  fi

  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"name": "backlit-dedicated-drm-session"'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"drm_master_boundary": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_handoff_plan": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_handoff_script":'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"mutating_handoff_attempted": false'
  if grep '"expected_blocked": false' "$artifact_root/dedicated-drm-session/manifest.json" >/dev/null; then
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_session_acceptance": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"first_present_commit_succeeded": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"first_present_vblank_event_received": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"session_drm_first_present_probe": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"session_clean_exit": true'
  else
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"expected_blocked": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_session_acceptance": false'
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
    "compositor_runtime_trait": true,
    "smithay_runtime_probe": true,
    "drm_master_boundary": true,
    "smithay_compositor_runtime": true,
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
    "dedicated_drm_session": true,
    "mvp1_contract": true,
    "ci_gate": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit MVP 0 contract verification passed. Artifacts: %s\n' "$out_dir"
