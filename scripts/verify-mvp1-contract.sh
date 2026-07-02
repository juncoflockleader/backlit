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

require_matches() {
  file="$1"
  value="$2"
  grep -E "$value" "$file" >/dev/null || fail "missing pattern in $file: $value"
}

require_file docs/architecture/mvp-1.md
require_file docs/architecture/real-shm-client-pixels.md
require_file docs/runbooks/parallels-ubuntu-readonly.md
require_executable scripts/verify-launch-readiness.sh
require_executable scripts/verify-session-launch.sh
require_executable scripts/verify-drm-session-smoke.sh
require_executable scripts/verify-drm-master-boundary.sh
require_executable scripts/verify-dedicated-drm-session.sh
require_executable scripts/verify-session-replay.sh
require_executable scripts/verify-compositor-socket.sh
require_executable scripts/verify-launcher-desktop-discovery.sh
require_executable scripts/verify-debian-package-install.sh
require_executable scripts/verify-debian-system-install.sh
require_executable scripts/verify-launch-performance.sh
require_executable scripts/verify-resource-budget.sh
require_executable scripts/verify-smithay-runtime-probe.sh
require_executable scripts/verify-smithay-compositor-runtime.sh
require_executable scripts/verify-smithay-real-app-e2e.sh
require_executable scripts/verify-smithay-real-shm-frame.sh
require_executable scripts/verify-nested-wayland-smoke.sh
require_executable scripts/verify-linux-e2e.sh
require_executable scripts/verify-parallels-ubuntu-health.sh
require_executable scripts/verify-parallels-post-repair-readiness.sh
require_executable scripts/verify-parallels-mvp-e2e.sh
require_executable scripts/verify-parallels-dedicated-drm-e2e.sh
require_executable scripts/verify-mvp-complete.sh

require_contains docs/architecture/mvp-1.md 'MVP 1 is the bare graphical session'
require_contains docs/architecture/mvp-1.md 'DRM/KMS backend'
require_contains docs/architecture/mvp-1.md 'libinput keyboard and pointer support'
require_contains docs/architecture/mvp-1.md 'Wayland app windows'
require_contains docs/architecture/mvp-1.md 'terminal hotkey'
require_contains docs/architecture/mvp-1.md 'app switcher'
require_contains docs/architecture/mvp-1.md 'clean exit'
require_contains docs/architecture/mvp-1.md 'managed desktop-launch window'
require_contains docs/architecture/mvp-1.md 'verify-mvp-complete.sh'
require_contains docs/architecture/mvp-1.md 'does not claim the real DRM compositor loop is complete'
require_contains docs/architecture/mvp-1.md 'parallels-ubuntu-health/manifest.json'
require_contains docs/architecture/mvp-1.md 'docs/runbooks/parallels-ubuntu-readonly.md'
require_contains docs/architecture/real-shm-client-pixels.md 'real `wl_shm` pixels in a Backlit-rendered frame'
require_contains docs/architecture/real-shm-client-pixels.md 'not full GPU texture compositing'
require_contains docs/architecture/real-shm-client-pixels.md 'scripts/verify-smithay-real-shm-frame.sh'
require_contains docs/runbooks/parallels-ubuntu-readonly.md 'guest-root-read-only'
require_contains docs/runbooks/parallels-ubuntu-readonly.md 'Do not run `fsck` against a'
require_contains docs/runbooks/parallels-ubuntu-readonly.md 'mounted root filesystem'
require_contains docs/runbooks/parallels-ubuntu-readonly.md './scripts/verify-parallels-linux-e2e.sh target/linux-e2e-parallels'
require_contains docs/runbooks/parallels-ubuntu-readonly.md './scripts/verify-mvp-complete.sh target/mvp-complete target/linux-e2e-parallels target/parallels-dedicated-drm-e2e'
require_contains scripts/verify-launch-readiness.sh '"drm_expected_ready"'
require_contains scripts/verify-launch-readiness.sh '"drm_card_access_ready"'
require_contains scripts/verify-launch-readiness.sh '"input_broker_ready"'
require_contains scripts/verify-launch-readiness.sh '"backend.launch_plan"'
require_contains scripts/verify-launch-readiness.sh '"drm_launch_plan": true'
require_contains scripts/verify-session-launch.sh 'backlit-session --backend=drm --activate-systemd'
require_contains scripts/verify-session-launch.sh '"session_systemd_launch_plan"'
require_contains scripts/verify-session-launch.sh '"session.backend_launch_plan"'
require_contains scripts/verify-session-launch.sh '"drm_backend_launch_plan": true'
require_contains scripts/verify-session-launch.sh '"drm_smithay_launch_plan": true'
require_contains scripts/verify-session-launch.sh '"implementation":"smithay-compositor-runtime"'
require_contains scripts/verify-drm-session-smoke.sh '--backend=drm'
require_contains scripts/verify-drm-session-smoke.sh '"drm_session_smoke_ready"'
require_contains scripts/verify-drm-session-smoke.sh '"drm_session_clean_exit"'
require_contains scripts/verify-drm-session-smoke.sh '"session.backend_launch_plan"'
require_contains scripts/verify-drm-session-smoke.sh '"drm_backend_launch_plan": true'
require_contains scripts/verify-drm-session-smoke.sh '"drm_smithay_launch_plan": true'
require_contains scripts/verify-drm-session-smoke.sh '"implementation":"smithay-compositor-runtime"'
require_contains scripts/verify-drm-session-smoke.sh '--verify-desktop-launch'
require_contains scripts/verify-drm-session-smoke.sh '--verify-drm-first-present'
require_contains scripts/verify-drm-session-smoke.sh '"session_drm_first_present_probe": $session_drm_first_present_probe'
require_contains scripts/verify-drm-session-smoke.sh '"session_desktop_launch": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_desktop_managed_window": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_demo_client": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_demo_app_id_preserved": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_runtime": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_protocol_globals": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_input_sources": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_input_event_loop": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_input_seat_handles": $drm_session_smoke_ready'
require_contains scripts/verify-drm-session-smoke.sh '"session_compositor_smithay_input_seat_dispatch": $drm_session_smoke_ready'
require_contains scripts/verify-nested-wayland-smoke.sh '--verify-desktop-launch'
require_contains scripts/verify-nested-wayland-smoke.sh '"session_wayland_desktop_launch": true'
require_contains scripts/verify-nested-wayland-smoke.sh '"session_wayland_desktop_managed_window": true'
require_contains scripts/verify-nested-wayland-smoke.sh '"session_wayland_demo_app_id_preserved": $session_compositor_demo_client'
require_contains scripts/verify-drm-master-boundary.sh '"name": "backlit-drm-master-boundary"'
require_contains scripts/verify-drm-master-boundary.sh '"session_entry_drm": true'
require_contains scripts/verify-drm-master-boundary.sh '"compositor_service_drm": true'
require_contains scripts/verify-drm-master-boundary.sh '"compositor_service_smithay_runtime": true'
require_contains scripts/verify-drm-master-boundary.sh '"dedicated_session_required": $dedicated_session_required'
require_contains scripts/verify-drm-master-boundary.sh '"current_session_can_present": $current_session_can_present'
require_contains scripts/verify-dedicated-drm-session.sh '"name": "backlit-dedicated-drm-session"'
require_contains scripts/verify-dedicated-drm-session.sh 'BACKLIT_DEDICATED_DRM_SESSION_BIN'
require_contains scripts/verify-dedicated-drm-session.sh '"session_binary": "$session_bin"'
require_contains scripts/verify-dedicated-drm-session.sh '"system_session_binary": $system_session_binary'
require_contains scripts/verify-dedicated-drm-session.sh '--require-drm-master-present'
require_contains scripts/verify-dedicated-drm-session.sh '"implementation":"smithay-compositor-runtime"'
require_contains scripts/verify-dedicated-drm-session.sh 'cargo build -p backlit-compositor --features smithay-backend'
require_contains scripts/verify-dedicated-drm-session.sh '"dedicated_handoff_plan": true'
require_contains scripts/verify-dedicated-drm-session.sh '"dedicated_handoff_script_checked": true'
require_contains scripts/verify-dedicated-drm-session.sh '"dedicated_handoff_seat_owner_required": true'
require_contains scripts/verify-dedicated-drm-session.sh '"dedicated_handoff_drm_master_present_required": true'
require_contains scripts/verify-dedicated-drm-session.sh '"dedicated_handoff_acceptance_checks": true'
require_contains scripts/verify-dedicated-drm-session.sh '"dedicated_session_acceptance": $dedicated_session_acceptance'
require_contains scripts/verify-dedicated-drm-session.sh '"first_present_commit_succeeded": $first_present_commit_succeeded'
require_contains scripts/verify-dedicated-drm-session.sh '"first_present_vblank_event_received": $first_present_vblank_event_received'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh 'systemd-run'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh 'PAMName=login'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh 'TTYPath='
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh 'BACKLIT_DEDICATED_DRM_SESSION_BIN=/usr/bin/backlit-session'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh 'BACKLIT_REQUIRE_DEDICATED_DRM_SESSION=1'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh '"system_package_dedicated_drm": true'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh '"system_session_binary": true'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh '"debs_built": true'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh '"dedicated_session_acceptance": true'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh 'system-dpkg-install.log'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh 'system-dpkg-purge.log'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh 'fastgui-core'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh 'verify-parallels-ubuntu-health.sh'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh '"parallels_ubuntu_health": true'
require_contains scripts/verify-parallels-dedicated-drm-e2e.sh '"guest_root_filesystem_writable": true'
require_contains scripts/verify-parallels-linux-e2e.sh 'check_guest_writable'
require_contains scripts/verify-parallels-linux-e2e.sh 'verify-parallels-ubuntu-health.sh'
require_contains scripts/verify-parallels-linux-e2e.sh '"parallels_ubuntu_health": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"guest_root_filesystem_writable": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"launch_performance": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"startup_budget": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"terminal_launch_budget": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"shell_ready_budget": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"resource_budget": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"resource_budget_checked": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"idle_cpu_budget": true'
require_contains scripts/verify-parallels-linux-e2e.sh '"idle_rss_budget": true'
require_contains scripts/verify-parallels-ubuntu-health.sh '"name": "backlit-parallels-ubuntu-health"'
require_contains scripts/verify-parallels-ubuntu-health.sh '"root_filesystem_writable": $(json_bool "$root_filesystem_writable")'
require_contains scripts/verify-parallels-ubuntu-health.sh '"tmp_writable": $(json_bool "$tmp_writable")'
require_contains scripts/verify-parallels-ubuntu-health.sh 'guest-root-read-only'
require_contains scripts/verify-parallels-ubuntu-health.sh 'verify-parallels-linux-e2e.sh'
require_contains scripts/verify-parallels-ubuntu-health.sh 'verify-parallels-dedicated-drm-e2e.sh'
require_contains scripts/verify-parallels-ubuntu-health.sh 'docs/runbooks/parallels-ubuntu-readonly.md'
require_contains scripts/verify-parallels-post-repair-readiness.sh '"name": "backlit-parallels-post-repair-readiness"'
require_contains scripts/verify-parallels-post-repair-readiness.sh '"ready_for_parallels_mvp_e2e": $(json_bool "$passed")'
require_contains scripts/verify-parallels-post-repair-readiness.sh './scripts/verify-parallels-ubuntu-health.sh "$dir"'
require_contains scripts/verify-parallels-post-repair-readiness.sh './scripts/verify-parallels-mvp-e2e.sh'
require_contains scripts/verify-parallels-post-repair-readiness.sh 'docs/runbooks/parallels-ubuntu-readonly.md'
require_contains scripts/verify-parallels-mvp-e2e.sh '"name": "backlit-parallels-mvp-e2e"'
require_contains scripts/verify-parallels-mvp-e2e.sh '"source_tree_ready": $(json_bool "$source_tree_ready")'
require_contains scripts/verify-parallels-mvp-e2e.sh './scripts/render-gui-preview.sh "$local_gui_dir"'
require_contains scripts/verify-parallels-mvp-e2e.sh '"local_gui_preview": $(json_bool "$local_gui_preview_passed")'
require_contains scripts/verify-parallels-mvp-e2e.sh 'local-gui-preview-failed'
require_contains scripts/verify-parallels-mvp-e2e.sh '"normal_reason": $(json_string "$normal_health_reason")'
require_contains scripts/verify-parallels-mvp-e2e.sh '"normal_root_mount": $(json_string "$normal_health_root_mount")'
require_contains scripts/verify-parallels-mvp-e2e.sh 'refresh_health_summary'
require_contains scripts/verify-parallels-mvp-e2e.sh 'dirty-worktree'
require_contains scripts/verify-parallels-mvp-e2e.sh 'unpushed-commit'
require_contains scripts/verify-parallels-mvp-e2e.sh './scripts/verify-parallels-ubuntu-health.sh "$normal_health_dir"'
require_contains scripts/verify-parallels-mvp-e2e.sh './scripts/verify-parallels-linux-e2e.sh "$normal_e2e_dir"'
require_contains scripts/verify-parallels-mvp-e2e.sh './scripts/verify-parallels-dedicated-drm-e2e.sh "$dedicated_e2e_dir"'
require_contains scripts/verify-parallels-mvp-e2e.sh './scripts/verify-mvp-complete.sh "$mvp_complete_dir" "$normal_e2e_dir" "$dedicated_e2e_dir"'
require_contains scripts/verify-parallels-mvp-e2e.sh 'parallels-health-failed'
require_contains scripts/verify-mvp-complete.sh '"name": "backlit-mvp-complete"'
require_contains scripts/verify-mvp-complete.sh '"source_tree_ready": $source_tree_ready'
require_contains scripts/verify-mvp-complete.sh '"worktree_clean": $worktree_clean'
require_contains scripts/verify-mvp-complete.sh '"pushed_commit": $pushed_commit'
require_contains scripts/verify-mvp-complete.sh 'dirty-worktree'
require_contains scripts/verify-mvp-complete.sh 'unpushed-commit'
require_contains scripts/verify-mvp-complete.sh '"actual_system_dpkg_install": true'
require_contains scripts/verify-mvp-complete.sh '"launch_performance_evidence": $launch_performance_evidence'
require_contains scripts/verify-mvp-complete.sh '"startup_budget": true'
require_contains scripts/verify-mvp-complete.sh '"resource_budget": true'
require_contains scripts/verify-mvp-complete.sh '"idle_cpu_budget": true'
require_contains scripts/verify-mvp-complete.sh '"parallels_health_evidence": $parallels_health_evidence'
require_contains scripts/verify-mvp-complete.sh 'missing-parallels-linux-health-manifest'
require_contains scripts/verify-mvp-complete.sh 'missing-parallels-dedicated-health-manifest'
require_contains scripts/verify-mvp-complete.sh '"root_filesystem_writable": true'
require_contains scripts/verify-mvp-complete.sh '"tmp_writable": true'
require_contains scripts/verify-mvp-complete.sh '"system_package_dedicated_drm": true'
require_contains scripts/verify-mvp-complete.sh 'system-dpkg-install.log'
require_contains scripts/verify-mvp-complete.sh 'system-dpkg-purge.log'
require_contains scripts/verify-mvp-complete.sh '"session_binary": "/usr/bin/backlit-session"'
require_contains scripts/verify-mvp-complete.sh '"package_installed_dedicated_drm": $package_installed_dedicated_drm'
require_contains scripts/verify-mvp-complete.sh '"preview_evidence": $preview_evidence'
require_contains scripts/verify-mvp-complete.sh '"semantic_gui_evidence": $semantic_gui_evidence'
require_contains scripts/verify-mvp-complete.sh '"gui_smoke_session_desktop_managed_window": true'
require_contains scripts/verify-mvp-complete.sh '"debian_system_install_desktop_managed_window": true'
require_contains scripts/verify-mvp-complete.sh '"nested_wayland_desktop_managed_window": true'
require_contains scripts/verify-mvp-complete.sh 'require_png_file'
require_contains scripts/verify-mvp-complete.sh '89504e470d0a1a0a'
require_contains scripts/verify-mvp-complete.sh '"png_written": true'
require_contains scripts/verify-mvp-complete.sh '"preview_format": "png"'
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
require_contains scripts/verify-smithay-runtime-probe.sh '"smithay_libinput_pointer_event_count": $smithay_libinput_pointer_event_count'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-smithay-runtime-probe.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-smithay-compositor-runtime.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-smithay-live-surface-snapshots.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-smithay-real-app-e2e.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-smithay-real-shm-frame.sh'
require_contains scripts/verify-smithay-live-surface-snapshots.sh '--smithay-live-surface-snapshots'
require_contains scripts/verify-smithay-live-surface-snapshots.sh '"live_snapshot_pixels_copied": true'
require_contains scripts/verify-smithay-live-surface-snapshots.sh '"live_snapshot_damage_recorded": true'
require_contains scripts/verify-smithay-live-surface-snapshots.sh '"policy_window_from_live_snapshot": true'
require_contains crates/compositor/src/main.rs '"compositor.smithay_live_surface_snapshots"'
require_contains crates/compositor-backend/src/lib.rs 'run_live_surface_snapshot_capture'
require_contains scripts/verify-smithay-real-app-e2e.sh '--smithay-real-app-e2e'
require_contains scripts/verify-smithay-real-app-e2e.sh '"real_installed_app": true'
require_contains scripts/verify-smithay-real-app-e2e.sh '"real_app_shm_pixels_captured": true'
require_contains scripts/verify-smithay-real-app-e2e.sh '"real_app_pixels_composited": true'
require_contains scripts/verify-smithay-real-app-e2e.sh '"real_app_frame_samples_verified": true'
require_contains scripts/verify-smithay-real-app-e2e.sh '"policy_window_from_real_app": true'
require_contains crates/compositor/src/main.rs '"compositor.smithay_real_app_e2e"'
require_contains crates/compositor/src/main.rs 'smithay_real_app_frame_output'
require_contains crates/compositor-backend/src/lib.rs 'run_real_app_e2e_capture'
require_contains scripts/verify-smithay-real-shm-frame.sh '--smithay-real-shm-frame'
require_contains scripts/verify-smithay-real-shm-frame.sh '"real_shm_pixels_captured": true'
require_contains scripts/verify-smithay-real-shm-frame.sh '"real_shm_pixels_composited": true'
require_contains scripts/verify-smithay-real-shm-frame.sh '"real_client_pixel_samples_verified": true'
require_contains crates/compositor/src/main.rs '"compositor.smithay_real_shm_frame"'
require_contains crates/compositor/src/main.rs 'smithay_real_shm_frame_output'
require_contains crates/compositor-backend/src/lib.rs 'run_real_shm_frame_capture'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-drm-master-boundary.sh'
require_contains scripts/verify-smithay-compositor-runtime.sh '--features smithay-backend'
require_contains scripts/verify-smithay-compositor-runtime.sh '--runtime=smithay'
require_contains scripts/verify-smithay-compositor-runtime.sh '--drm-first-present-probe'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_compositor_runtime": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_runtime_launch_plan": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_core_protocol_globals": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_mvp_protocol_globals": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"mvp_protocol_globals_announced":true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"linux_dmabuf_version_at_least_4":true'
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
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_normal_runtime_live_snapshot_frame": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_normal_runtime_real_pixels": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_surface_lifecycle": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_xdg_resize_commit": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_xdg_unmap_cleanup": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_xdg_close_disconnect": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_policy_lifecycle_cleanup": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_real_wayland_policy_window": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_drm_first_present_probe": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_service_socket": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_service_socket_runtime_trait": true'
require_contains scripts/verify-smithay-compositor-runtime.sh '"smithay_demo_client_socket_lifecycle": true'
require_contains crates/compositor/src/main.rs '"compositor.drm_first_present_probe"'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-drm-session-smoke.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-dedicated-drm-session.sh'
require_contains scripts/verify-linux-e2e.sh '"dedicated_drm_handoff": true'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-mvp1-contract.sh'

artifact_manifests_checked=false
drm_launch_ready_artifact=false
drm_session_smoke_ready_artifact=false
drm_master_boundary_artifact=false
dedicated_drm_session_artifact=false
debian_package_install_replay_artifact=false
debian_system_install_replay_artifact=false
nested_wayland_artifact=false
compositor_socket_artifact=false
smithay_runtime_probe_artifact=false
smithay_compositor_runtime_artifact=false
smithay_live_surface_snapshots_artifact=false
smithay_real_app_e2e_artifact=false
smithay_real_shm_frame_artifact=false

if [ -n "$artifact_root" ] && [ -d "$artifact_root" ]; then
  artifact_manifests_checked=true

  require_file "$artifact_root/launch-readiness/manifest.json"
  require_file "$artifact_root/session-launch/manifest.json"
  require_file "$artifact_root/drm-session-smoke/manifest.json"
  require_file "$artifact_root/drm-master-boundary/manifest.json"
  require_file "$artifact_root/dedicated-drm-session/manifest.json"
  require_file "$artifact_root/session-replay/manifest.json"
  require_file "$artifact_root/launch-performance/manifest.json"
  require_file "$artifact_root/resource-budget/manifest.json"
  require_file "$artifact_root/compositor-runtime/manifest.json"
  require_file "$artifact_root/compositor-socket/manifest.json"
  require_file "$artifact_root/smithay-runtime-probe/manifest.json"
  require_file "$artifact_root/smithay-compositor-runtime/manifest.json"
  require_file "$artifact_root/smithay-live-surface-snapshots/manifest.json"
  require_file "$artifact_root/smithay-real-app-e2e/manifest.json"
  require_file "$artifact_root/smithay-real-shm-frame/manifest.json"
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
  require_contains "$artifact_root/session-launch/manifest.json" '"drm_smithay_launch_plan": true'

  if grep '"drm_session_expected_ready": true' "$artifact_root/session-launch/manifest.json" >/dev/null; then
    require_contains "$artifact_root/session-launch/manifest.json" '"drm_session_ready": true'
    require_contains "$artifact_root/session-launch/manifest.json" '"drm_device_selected": true'
    require_contains "$artifact_root/session-launch/manifest.json" '"drm_input_selected": true'
  else
    require_contains "$artifact_root/session-launch/manifest.json" '"drm_session_blocked_expected": true'
  fi

  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"name": "backlit-drm-session-smoke"'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_backend_launch_plan": true'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_smithay_launch_plan": true'
  if grep '"drm_session_smoke_ready": true' "$artifact_root/drm-session-smoke/manifest.json" >/dev/null; then
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_session_clean_exit": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_device_selected": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_input_selected": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_drm_first_present_probe": true'
    require_matches "$artifact_root/drm-session-smoke/manifest.json" '"session_first_present_(commit_succeeded|blocked_by_drm_master)": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"settings_service": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"notification_service": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"workspace_switch": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"snap": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_desktop_launch": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_desktop_managed_window": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_runtime": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_protocol_globals": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_input_sources": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_input_event_loop": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_input_seat_handles": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_smithay_input_seat_dispatch": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_demo_client": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"session_compositor_demo_app_id_preserved": true'
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"input_broker_ready": true'
    drm_session_smoke_ready_artifact=true
  else
    require_contains "$artifact_root/drm-session-smoke/manifest.json" '"drm_session_smoke_blocked_expected": true'
  fi

  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"name": "backlit-drm-master-boundary"'
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"session_entry_drm": true'
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"compositor_service_drm": true'
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"compositor_service_smithay_runtime": true'
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"mutating_handoff_attempted": false'
  require_contains "$artifact_root/drm-master-boundary/manifest.json" '"dedicated_session_model": "seat-owner-tty-or-display-manager-session"'
  if grep '"drm_launch_ready": true' "$artifact_root/drm-master-boundary/manifest.json" >/dev/null; then
    require_contains "$artifact_root/drm-master-boundary/manifest.json" '"drm_master_boundary_checked": true'
    require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_framebuffer_filled": true'
    require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_plane_state_ready": true'
    if grep '"current_session_can_present": true' "$artifact_root/drm-master-boundary/manifest.json" >/dev/null; then
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_commit_succeeded": true'
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_vblank_event_received": true'
    else
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"first_present_blocked_by_drm_master": true'
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"drm_master_boundary_observed": true'
      require_contains "$artifact_root/drm-master-boundary/manifest.json" '"dedicated_session_required": true'
    fi
    drm_master_boundary_artifact=true
  else
    require_contains "$artifact_root/drm-master-boundary/manifest.json" '"expected_blocked": true'
  fi

  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"name": "backlit-dedicated-drm-session"'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"drm_master_boundary": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_handoff_plan": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_handoff_script":'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"seat_owner_required": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"drm_master_present_required": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"acceptance_checks": "first-present-commit-vblank-gui-services-launch-clean-exit"'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_handoff_script_checked": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_handoff_seat_owner_required": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_handoff_drm_master_present_required": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_handoff_acceptance_checks": true'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"mutating_handoff_attempted": false'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_session_model": "seat-owner-tty-or-display-manager-session"'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"session_binary":'
  require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"system_session_binary":'
  if grep '"expected_blocked": false' "$artifact_root/dedicated-drm-session/manifest.json" >/dev/null; then
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_session_acceptance": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"first_present_commit_succeeded": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"first_present_vblank_event_received": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"session_drm_first_present_probe": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"session_gui_verified": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"session_services": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"session_clean_exit": true'
  else
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"expected_blocked": true'
    require_contains "$artifact_root/dedicated-drm-session/manifest.json" '"dedicated_session_acceptance": false'
    require_matches "$artifact_root/dedicated-drm-session/manifest.json" '"reason": "(drm-master-unavailable|drm-launch-not-ready|non-linux-host)"'
  fi
  dedicated_drm_session_artifact=true

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
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_runtime_launch_plan":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_scripted_client":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_core_protocol_globals":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_mvp_protocol_globals":'
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
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_normal_runtime_live_snapshot_frame":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_normal_runtime_real_pixels":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_surface_lifecycle":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_xdg_resize_commit":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_xdg_unmap_cleanup":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_xdg_close_disconnect":'
  require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_policy_lifecycle_cleanup":'
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
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_mvp_protocol_globals": true'
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
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_normal_runtime_live_snapshot_frame": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_normal_runtime_real_pixels": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_surface_lifecycle": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_xdg_resize_commit": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_xdg_unmap_cleanup": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_xdg_close_disconnect": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_policy_lifecycle_cleanup": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_real_wayland_policy_window": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_event_loop_runtime": true'
      require_contains "$artifact_root/smithay-compositor-runtime/manifest.json" '"smithay_drm_first_present_probe": true'
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
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"name": "backlit-smithay-live-surface-snapshots"'
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"smithay_live_surface_snapshots":'
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"real_wayland_client":'
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_pipeline":'
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_persisted":'
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_metadata_preserved":'
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_pixels_copied":'
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_damage_recorded":'
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_samples_verified":'
  require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"policy_window_from_live_snapshot":'
  if grep '"checked": true' "$artifact_root/smithay-live-surface-snapshots/manifest.json" >/dev/null; then
    if grep '"drm_launch_ready": true' "$artifact_root/smithay-live-surface-snapshots/manifest.json" >/dev/null; then
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"smithay_live_surface_snapshots": true'
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"real_wayland_client": true'
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_pipeline": true'
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_persisted": true'
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_metadata_preserved": true'
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_pixels_copied": true'
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_damage_recorded": true'
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"live_snapshot_samples_verified": true'
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"policy_window_from_live_snapshot": true'
      smithay_live_surface_snapshots_artifact=true
    else
      require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"expected_blocked": true'
    fi
  else
    require_contains "$artifact_root/smithay-live-surface-snapshots/manifest.json" '"expected_blocked": true'
  fi
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"name": "backlit-smithay-real-app-e2e"'
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"smithay_real_app_e2e":'
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_installed_app":'
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_wayland_client_connected":'
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_metadata_observed":'
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_shm_pixels_captured":'
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_pixels_composited":'
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_frame_samples_verified":'
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"policy_window_from_real_app":'
  require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"frame_ppm_written":'
  if grep '"checked": true' "$artifact_root/smithay-real-app-e2e/manifest.json" >/dev/null; then
    if grep '"drm_launch_ready": true' "$artifact_root/smithay-real-app-e2e/manifest.json" >/dev/null; then
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"smithay_real_app_e2e": true'
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_installed_app": true'
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_wayland_client_connected": true'
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_metadata_observed": true'
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_shm_pixels_captured": true'
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_pixels_composited": true'
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"real_app_frame_samples_verified": true'
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"policy_window_from_real_app": true'
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"frame_ppm_written": true'
      smithay_real_app_e2e_artifact=true
    else
      require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"expected_blocked": true'
    fi
  else
    require_contains "$artifact_root/smithay-real-app-e2e/manifest.json" '"expected_blocked": true'
  fi
  require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"name": "backlit-smithay-real-shm-frame"'
  require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"smithay_real_shm_frame":'
  require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_wayland_client":'
  require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_wayland_metadata":'
  require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_shm_pixels_captured":'
  require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_shm_pixels_composited":'
  require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_client_pixel_samples_verified":'
  require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"policy_window_from_real_surface":'
  require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"frame_ppm_written":'
  if grep '"checked": true' "$artifact_root/smithay-real-shm-frame/manifest.json" >/dev/null; then
    if grep '"drm_launch_ready": true' "$artifact_root/smithay-real-shm-frame/manifest.json" >/dev/null; then
      require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"smithay_real_shm_frame": true'
      require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_wayland_client": true'
      require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_wayland_metadata": true'
      require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_shm_pixels_captured": true'
      require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_shm_pixels_composited": true'
      require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"real_client_pixel_samples_verified": true'
      require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"policy_window_from_real_surface": true'
      require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"frame_ppm_written": true'
      smithay_real_shm_frame_artifact=true
    else
      require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"expected_blocked": true'
    fi
  else
    require_contains "$artifact_root/smithay-real-shm-frame/manifest.json" '"expected_blocked": true'
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
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_desktop_launch": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_desktop_managed_window": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_demo_client": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_demo_app_id_preserved": true'
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
    "real_shm_client_pixels_plan": "docs/architecture/real-shm-client-pixels.md",
    "launch_readiness_verifier": "scripts/verify-launch-readiness.sh",
    "session_launch_verifier": "scripts/verify-session-launch.sh",
    "drm_session_smoke_verifier": "scripts/verify-drm-session-smoke.sh",
    "smithay_live_surface_snapshots_verifier": "scripts/verify-smithay-live-surface-snapshots.sh",
    "smithay_real_app_e2e_verifier": "scripts/verify-smithay-real-app-e2e.sh",
    "smithay_real_shm_frame_verifier": "scripts/verify-smithay-real-shm-frame.sh",
    "dedicated_drm_session_verifier": "scripts/verify-dedicated-drm-session.sh",
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
    "smithay_live_surface_snapshots_contract": true,
    "smithay_live_surface_snapshots_artifact": $smithay_live_surface_snapshots_artifact,
    "smithay_real_app_e2e_contract": true,
    "smithay_real_app_e2e_artifact": $smithay_real_app_e2e_artifact,
    "smithay_real_shm_frame_contract": true,
    "smithay_real_shm_frame_artifact": $smithay_real_shm_frame_artifact,
    "compositor_socket_contract": true,
    "compositor_socket_artifact": $compositor_socket_artifact,
    "drm_launch_ready_artifact": $drm_launch_ready_artifact,
    "drm_session_smoke_ready_artifact": $drm_session_smoke_ready_artifact,
    "drm_master_boundary_artifact": $drm_master_boundary_artifact,
    "dedicated_drm_session_artifact": $dedicated_drm_session_artifact,
    "debian_package_install_replay_artifact": $debian_package_install_replay_artifact,
    "debian_system_install_replay_artifact": $debian_system_install_replay_artifact,
    "nested_wayland_artifact": $nested_wayland_artifact
  }
}
EOF

grep '"passed": true' "$manifest" >/dev/null

printf 'Backlit MVP 1 contract verification passed. Artifacts: %s\n' "$out_dir"
