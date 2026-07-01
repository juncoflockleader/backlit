#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/mvp1-contract}"
artifact_root="${2:-}"
manifest="$out_dir/manifest.json"
mkdir -p "$out_dir"

fail() {
  echo "MVP 1 contract verification failed: $*" >&2
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

require_file docs/architecture/mvp-1.md
require_executable scripts/verify-launch-readiness.sh
require_executable scripts/verify-session-launch.sh
require_executable scripts/verify-drm-session-smoke.sh
require_executable scripts/verify-session-replay.sh
require_executable scripts/verify-compositor-socket.sh
require_executable scripts/verify-launcher-desktop-discovery.sh
require_executable scripts/verify-debian-package-install.sh
require_executable scripts/verify-debian-system-install.sh
require_executable scripts/verify-launch-performance.sh
require_executable scripts/verify-resource-budget.sh
require_executable scripts/verify-smithay-runtime-probe.sh
require_executable scripts/verify-smithay-compositor-runtime.sh
require_executable scripts/verify-linux-e2e.sh

require_contains docs/architecture/mvp-1.md 'MVP 1 is the bare graphical session'
require_contains docs/architecture/mvp-1.md 'DRM/KMS backend'
require_contains docs/architecture/mvp-1.md 'libinput keyboard and pointer support'
require_contains docs/architecture/mvp-1.md 'Wayland app windows'
require_contains docs/architecture/mvp-1.md 'terminal hotkey'
require_contains docs/architecture/mvp-1.md 'app switcher'
require_contains docs/architecture/mvp-1.md 'clean exit'
require_contains docs/architecture/mvp-1.md 'does not claim the real DRM compositor loop is complete'
require_contains scripts/verify-launch-readiness.sh '"drm_expected_ready"'
require_contains scripts/verify-launch-readiness.sh '"drm_card_access_ready"'
require_contains scripts/verify-launch-readiness.sh '"input_broker_ready"'
require_contains scripts/verify-launch-readiness.sh '"backend.launch_plan"'
require_contains scripts/verify-launch-readiness.sh '"drm_launch_plan": true'
require_contains scripts/verify-session-launch.sh 'backlit-session --backend=drm --activate-systemd'
require_contains scripts/verify-session-launch.sh '"session_systemd_launch_plan"'
require_contains scripts/verify-session-launch.sh '"session.backend_launch_plan"'
require_contains scripts/verify-session-launch.sh '"drm_backend_launch_plan": true'
require_contains scripts/verify-drm-session-smoke.sh '--backend=drm'
require_contains scripts/verify-drm-session-smoke.sh '"drm_session_smoke_ready"'
require_contains scripts/verify-drm-session-smoke.sh '"drm_session_clean_exit"'
require_contains scripts/verify-drm-session-smoke.sh '"session.backend_launch_plan"'
require_contains scripts/verify-drm-session-smoke.sh '"drm_backend_launch_plan": true'
require_contains scripts/verify-drm-session-smoke.sh '--verify-desktop-launch'
require_contains scripts/verify-drm-session-smoke.sh '"session_desktop_launch": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_desktop_managed_window": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_demo_client": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_demo_app_id_preserved": $drm_session_smoke_ready'
require_contains scripts/verify-session-replay.sh '"launcher_overlay_frame": true'
require_contains scripts/verify-session-replay.sh '"app_switcher_overlay_frame": true'
require_contains scripts/verify-compositor-socket.sh '"session_socket_bound": true'
require_contains scripts/verify-compositor-socket.sh '"socket_accepts_client_connection": true'
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
require_contains scripts/verify-launcher-desktop-discovery.sh '--desktop-entry=org.backlit.SpawnProbe.desktop'
require_contains scripts/verify-launcher-desktop-discovery.sh '"fixture_desktop_spawn": true'
require_contains scripts/verify-compositor-runtime.sh '"runtime_backend_contract": true'
require_contains scripts/verify-compositor-runtime.sh '"runtime_backend": "headless-compositor"'
require_contains scripts/verify-compositor-runtime.sh '"runtime_trait": true'
require_contains scripts/verify-smithay-runtime-probe.sh '--features smithay-backend'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_dependency_compiled": true'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_runtime_probe": $smithay_runtime_probe'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_runtime_bootstrap": $smithay_runtime_bootstrap'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_wayland_socket_bootstrap": $smithay_wayland_socket_bootstrap'
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_wayland_client_inserted": $smithay_wayland_client_inserted'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-smithay-runtime-probe.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-smithay-compositor-runtime.sh'
require_contains scripts/verify-smithay-compositor-runtime.sh '--features smithay-backend'
require_contains scripts/verify-smithay-compositor-runtime.sh '--runtime=smithay'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_compositor_runtime": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_core_protocol_globals": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_wayland_client": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_service_socket": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_service_socket_runtime_trait": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_demo_client_socket_lifecycle": true'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-drm-session-smoke.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-mvp1-contract.sh'

artifact_manifests_checked=false
drm_launch_ready_artifact=false
drm_session_smoke_ready_artifact=false
debian_package_install_replay_artifact=false
debian_system_install_replay_artifact=false
nested_wayland_artifact=false
compositor_socket_artifact=false
smithay_runtime_probe_artifact=false
smithay_compositor_runtime_artifact=false

if [ -n "$artifact_root" ] && [ -d "$artifact_root" ]; then
  artifact_manifests_checked=true

  require_file "$artifact_root/launch-readiness/manifest.json"
  require_file "$artifact_root/session-launch/manifest.json"
  require_file "$artifact_root/drm-session-smoke/manifest.json"
  require_file "$artifact_root/session-replay/manifest.json"
  require_file "$artifact_root/launch-performance/manifest.json"
  require_file "$artifact_root/resource-budget/manifest.json"
  require_file "$artifact_root/compositor-runtime/manifest.json"
  require_file "$artifact_root/compositor-socket/manifest.json"
  require_file "$artifact_root/smithay-runtime-probe/manifest.json"
  require_file "$artifact_root/smithay-compositor-runtime/manifest.json"
  require_file "$artifact_root/launcher-desktop-discovery/manifest.json"
  require_file "$artifact_root/debian-package-install/manifest.json"
  require_file "$artifact_root/debian-system-install/manifest.json"

  require_contains "$artifact_root/launch-readiness/manifest.json" '"name": "backlit-launch-readiness"'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_checked": true'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"headless_launch_plan": true'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_launch_plan": true'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"xdg_runtime_dir_owned_by_user":'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"session_local":'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_card_access_ready":'
  require_contains "$artifact_root/launch-readiness/manifest.json" '"input_broker_ready":'

  if grep '"drm_expected_ready": true' "$artifact_root/launch-readiness/manifest.json" >/dev/null; then
    require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_ready": true'
    require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_device_selected": true'
    require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_input_selected": true'
    drm_launch_ready_artifact=true
  else
    require_contains "$artifact_root/launch-readiness/manifest.json" '"drm_blocked_expected": true'
  fi

  require_contains "$artifact_root/session-launch/manifest.json" '"desktop_exec": "backlit-session --backend=drm --activate-systemd"'
  require_contains "$artifact_root/session-launch/manifest.json" '"headless_session_launch_ready": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"headless_backend_launch_plan": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"session_systemd_units": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"session_systemd_target": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"session_systemd_launch_plan": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"drm_session_checked": true'
  require_contains "$artifact_root/session-launch/manifest.json" '"drm_backend_launch_plan": true'

  if grep '"drm_session_expected_ready": true' "$artifact_root/session-launch/manifest.json" >/dev/null; then
    require_contains "$artifact_root/session-launch/manifest.json" '"drm_session_ready": true'
    require_contains "$artifact_root/session-launch/manifest.json" '"drm_device_selected": true'
    require_contains "$artifact_root/session-launch/manifest.json" '"drm_input_selected": true'
  else
    require_contains "$artifact_root/session-launch/manifest.json" '"drm_session_blocked_expected": true'
  fi

  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"name": "backlit-drm-session-smoke"'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_backend_launch_plan": true'
  if grep '"drm_session_smoke_ready": true' "$artifact_root/drm-session-smoke/manifest.json" >/dev/null; then
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_session_clean_exit": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_device_selected": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_input_selected": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"settings_service": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"notification_service": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"workspace_switch": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"snap": true'
	    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_desktop_launch": true'
	    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_desktop_managed_window": true'
	    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_demo_client": true'
	    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_demo_app_id_preserved": true'
	    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"input_broker_ready": true'
    drm_session_smoke_ready_artifact=true
  else
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_session_smoke_blocked_expected": true'
  fi

  require_contains "$artifact_root/session-replay/manifest.json" '"frame_count": 9'
  require_contains "$artifact_root/session-replay/manifest.json" '"launcher_overlay_frame": true'
  require_contains "$artifact_root/session-replay/manifest.json" '"app_switcher_overlay_frame": true'
  require_contains "$artifact_root/session-replay/manifest.json" '"workspace_switch": true'
  require_contains "$artifact_root/launch-performance/manifest.json" '"startup_budget": true'
  require_contains "$artifact_root/launch-performance/manifest.json" '"terminal_launch_budget": true'
  require_contains "$artifact_root/launch-performance/manifest.json" '"shell_ready_budget": true'
  require_contains "$artifact_root/resource-budget/manifest.json" '"name": "backlit-resource-budget"'
  if grep '"resource_budget_checked": true' "$artifact_root/resource-budget/manifest.json" >/dev/null; then
    require_contains "$artifact_root/resource-budget/manifest.json" '"idle_cpu_budget": true'
    require_contains "$artifact_root/resource-budget/manifest.json" '"idle_rss_budget": true'
  else
    require_contains "$artifact_root/resource-budget/manifest.json" '"resource_budget_blocked_expected": true'
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
    compositor_socket_artifact=true
  else
    require_contains "$artifact_root/compositor-socket/manifest.json" '"socket_blocked_expected": true'
    require_contains "$artifact_root/compositor-socket/manifest.json" '"socket_permission_denied": true'
  fi
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"runtime_backend_contract": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"runtime_backend": "headless-compositor"'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"runtime_trait": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"surface_policy_preview": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"targeted_surface_damage": true'
  require_contains "$artifact_root/compositor-runtime/manifest.json" '"client_disconnect_cleanup": true'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"name": "backlit-smithay-compositor-runtime"'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_compositor_runtime":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_runtime_trait":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_scripted_client":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_core_protocol_globals":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_wayland_client":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_event_loop_runtime":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_service_ready":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_event_loop_service_socket":'
  if grep '"checked": true' "$artifact_root/smithay-compositor-runtime/manifest.json" >/dev/null; then
    if grep '"drm_launch_ready": true' "$artifact_root/smithay-compositor-runtime/manifest.json" >/dev/null; then
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_compositor_runtime": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_runtime_trait": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_scripted_client": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_core_protocol_globals": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_wayland_client": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_event_loop_runtime": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_service_ready": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_service_socket": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_service_socket_runtime_trait": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_event_loop_service_socket": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_demo_client_socket_lifecycle": true'
      smithay_compositor_runtime_artifact=true
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
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libinput_component": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_libseat_session_component": true'
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_calloop_component": true'
    if grep '"drm_launch_ready": true' "$artifact_root/smithay-runtime-probe/manifest.json" >/dev/null; then
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_runtime_probe": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_runtime_bootstrap": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_drm_node_resolved": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_renderer_node_selected": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_wayland_display_bootstrap": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_wayland_socket_bootstrap": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_wayland_client_inserted": true'
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"smithay_calloop_dispatch_bootstrap": true'
      smithay_runtime_probe_artifact=true
    else
      require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"expected_blocked": true'
    fi
  else
    require_contains "$artifact_root/smithay-runtime-probe/manifest.json" '"expected_blocked": true'
  fi
  require_contains "$artifact_root/launcher-desktop-discovery/manifest.json" '"fixture_desktop_discovery": true'
  require_contains "$artifact_root/launcher-desktop-discovery/manifest.json" '"fixture_desktop_spawn": true'
  require_contains "$artifact_root/launcher-desktop-discovery/manifest.json" '"fixture_desktop_exec_args": 2'

  require_contains "$artifact_root/debian-package-install/manifest.json" '"package_install_checked": true'
  if grep '"debs_extracted": true' "$artifact_root/debian-package-install/manifest.json" >/dev/null; then
    require_contains "$artifact_root/debian-package-install/manifest.json" '"dpkg_root_install": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_exec_from_extracted_debs": true'
	    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_services_from_extracted_debs": true'
	    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_compositor_demo_client_from_extracted_debs": true'
	    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_compositor_demo_app_id_from_extracted_debs": true'
	    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_desktop_launch_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_desktop_managed_window_from_extracted_debs": true'
    require_contains "$artifact_root/debian-package-install/manifest.json" '"session_replay_from_extracted_debs": true'
    debian_package_install_replay_artifact=true
  else
    require_contains "$artifact_root/debian-package-install/manifest.json" '"install_blocked_expected": true'
  fi

  require_contains "$artifact_root/debian-system-install/manifest.json" '"system_install_checked": true'
  if grep '"system_install_performed": true' "$artifact_root/debian-system-install/manifest.json" >/dev/null; then
    require_contains "$artifact_root/debian-system-install/manifest.json" '"actual_system_dpkg_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"usr_bin_session_launch": true'
	    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_services_from_system_install": true'
	    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_compositor_demo_client_from_system_install": true'
	    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_compositor_demo_app_id_from_system_install": true'
	    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_desktop_launch_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_desktop_managed_window_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"session_replay_from_system_install": true'
    require_contains "$artifact_root/debian-system-install/manifest.json" '"packages_purged_after_verification": true'
    debian_system_install_replay_artifact=true
  else
    require_contains "$artifact_root/debian-system-install/manifest.json" '"install_blocked_expected": true'
  fi

  if [ -f "$artifact_root/nested-wayland/manifest.json" ]; then
    require_contains "$artifact_root/nested-wayland/manifest.json" '"wayland_preflight_ready": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"launcher_terminal_wayland_spawn": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_clean_exit": true'
    nested_wayland_artifact=true
  fi
fi

cat > "$manifest" <<EOF
{
  "name": "backlit-mvp1-contract",
  "passed": true,
  "artifact_manifests_checked": $artifact_manifests_checked,
  "artifacts": {
    "architecture": "docs/architecture/mvp-1.md",
    "launch_readiness_verifier": "scripts/verify-launch-readiness.sh",
    "session_launch_verifier": "scripts/verify-session-launch.sh",
    "drm_session_smoke_verifier": "scripts/verify-drm-session-smoke.sh",
    "linux_e2e_verifier": "scripts/verify-linux-e2e.sh"
  },
  "checks": {
    "mvp1_scope_documented": true,
    "launch_readiness_contract": true,
    "desktop_session_entry": true,
    "systemd_launch_plan": true,
    "backend_launch_plan_contract": true,
    "drm_session_smoke_contract": true,
    "session_replay_contract": true,
    "desktop_entry_launch_contract": true,
    "session_desktop_launch_contract": true,
    "session_desktop_managed_window_contract": true,
    "package_install_contract": true,
    "resource_budget_contract": true,
    "compositor_runtime_trait_contract": true,
    "smithay_runtime_probe_contract": true,
    "smithay_runtime_probe_artifact": $smithay_runtime_probe_artifact,
    "smithay_compositor_runtime_contract": true,
    "smithay_compositor_runtime_artifact": $smithay_compositor_runtime_artifact,
    "compositor_socket_contract": true,
    "compositor_socket_artifact": $compositor_socket_artifact,
    "drm_launch_ready_artifact": $drm_launch_ready_artifact,
    "drm_session_smoke_ready_artifact": $drm_session_smoke_ready_artifact,
    "debian_package_install_replay_artifact": $debian_package_install_replay_artifact,
    "debian_system_install_replay_artifact": $debian_system_install_replay_artifact,
    "nested_wayland_artifact": $nested_wayland_artifact
  }
}
EOF

grep '"passed": true' "$manifest" >/dev/null

printf 'Backlit MVP 1 contract verification passed. Artifacts: %s\n' "$out_dir"
