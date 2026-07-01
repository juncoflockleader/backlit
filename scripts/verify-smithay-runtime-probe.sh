#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/smithay-runtime-probe}"
mkdir -p "$out_dir"

log="$out_dir/smithay-runtime-probe.jsonl"

fail() {
  echo "Smithay runtime probe verification failed: $*" >&2
  exit 1
}

require_contains() {
  file="$1"
  value="$2"
  grep -F "$value" "$file" >/dev/null || fail "missing text in $file: $value"
}

require_matches() {
  file="$1"
  value="$2"
  grep -E "$value" "$file" >/dev/null || fail "missing pattern in $file: $value"
}

extract_u64() {
  file="$1"
  key="$2"
  sed -n "s/.*\"$key\":\([0-9][0-9]*\).*/\1/p" "$file" | tail -n 1
}

if [ "$(uname -s)" != "Linux" ]; then
  cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-smithay-runtime-probe",
  "passed": true,
  "checked": false,
  "expected_blocked": true,
  "reason": "non-linux-host",
  "checks": {
    "smithay_dependency_compiled": false,
    "smithay_backend_feature": false,
    "smithay_drm_component": false,
    "smithay_gbm_allocator_component": false,
    "smithay_egl_display_component": false,
    "smithay_gles_renderer_component": false,
    "smithay_kms_card_opened": false,
    "smithay_kms_device_created": false,
    "smithay_kms_event_source_inserted": false,
    "smithay_kms_event_loop_dispatched": false,
    "smithay_kms_atomic_modesetting": false,
    "smithay_kms_crtc_count": 0,
    "smithay_kms_connector_count": 0,
    "smithay_kms_connected_connector_count": 0,
    "smithay_kms_mode_count": 0,
    "smithay_kms_primary_plane_count": 0,
    "smithay_kms_cursor_plane_count": 0,
    "smithay_kms_overlay_plane_count": 0,
    "smithay_kms_scanout_plan_ready": false,
    "smithay_kms_scanout_connector_id": 0,
    "smithay_kms_scanout_crtc_id": 0,
    "smithay_kms_scanout_primary_plane_id": 0,
    "smithay_kms_scanout_mode_width": 0,
    "smithay_kms_scanout_mode_height": 0,
    "smithay_kms_scanout_mode_refresh_hz": 0,
    "smithay_kms_scanout_mode_preferred": false,
    "smithay_renderer_node_opened": false,
    "smithay_gbm_device_created": false,
    "smithay_gbm_allocator_created": false,
    "smithay_egl_display_created": false,
    "smithay_egl_context_created": false,
    "smithay_gles_renderer_created": false,
    "smithay_offscreen_buffer_created": false,
    "smithay_offscreen_frame_rendered": false,
    "smithay_offscreen_frame_copied": false,
    "smithay_offscreen_pixel_verified": false,
    "smithay_offscreen_render_width": 0,
    "smithay_offscreen_render_height": 0,
    "smithay_offscreen_render_pixels": 0,
    "smithay_offscreen_sample_red": 0,
    "smithay_offscreen_sample_green": 0,
    "smithay_offscreen_sample_blue": 0,
    "smithay_offscreen_sample_alpha": 0,
    "smithay_drm_node_resolved": false,
    "smithay_renderer_node_selected": false,
    "smithay_libseat_session_created": false,
    "smithay_libseat_event_source_inserted": false,
    "smithay_libseat_event_loop_dispatched": false,
    "smithay_libinput_context_created": false,
    "smithay_libinput_seat_assigned": false,
    "smithay_libinput_backend_created": false,
    "smithay_libinput_event_source_inserted": false,
    "smithay_libinput_event_loop_dispatched": false,
    "smithay_libinput_component": false,
    "smithay_libseat_session_component": false,
    "smithay_calloop_component": false,
    "smithay_wayland_display_bootstrap": false,
    "smithay_wayland_socket_bootstrap": false,
    "smithay_wayland_client_inserted": false,
    "smithay_calloop_dispatch_bootstrap": false,
    "drm_launch_ready": false,
    "smithay_runtime_probe": false,
    "smithay_runtime_bootstrap": false
  }
}
EOF
  printf 'Backlit Smithay runtime probe skipped as expected: non-linux-host. Artifacts: %s\n' "$out_dir"
  exit 0
fi

cargo build -p backlit-compositor-backend --features smithay-backend

target/debug/backlit-compositor-backend \
  --backend=drm \
  --verify-smithay-runtime > "$log"

require_contains "$log" '"event":"backend.smithay_runtime_probe"'
require_contains "$log" '"event":"backend.smithay_runtime_bootstrap"'
require_contains "$log" '"feature_enabled":true'
require_contains "$log" '"compiled":true'
require_contains "$log" '"runtime_backend":"smithay-drm-probe"'
require_contains "$log" '"runtime_backend":"smithay-drm-bootstrap"'
require_contains "$log" '"display_driver":"smithay-drm-kms"'
require_contains "$log" '"input_driver":"smithay-libinput"'
require_contains "$log" '"session_driver":"smithay-libseat-logind"'
require_contains "$log" '"event_loop":"calloop"'
require_contains "$log" '"component_count":8'
require_contains "$log" '"gbm_allocator_component":true'
require_contains "$log" '"egl_display_component":true'
require_contains "$log" '"gles_renderer_component":true'

launch_ready=false
expected_blocked=true
smithay_runtime_probe=false
smithay_runtime_bootstrap=false
smithay_wayland_socket_bootstrap=false
smithay_wayland_client_inserted=false
smithay_drm_node_resolved=false
smithay_kms_card_opened=false
smithay_kms_device_created=false
smithay_kms_event_source_inserted=false
smithay_kms_event_loop_dispatched=false
smithay_kms_atomic_modesetting=false
smithay_kms_crtc_count=0
smithay_kms_connector_count=0
smithay_kms_connected_connector_count=0
smithay_kms_mode_count=0
smithay_kms_primary_plane_count=0
smithay_kms_cursor_plane_count=0
smithay_kms_overlay_plane_count=0
smithay_kms_scanout_plan_ready=false
smithay_kms_scanout_connector_id=0
smithay_kms_scanout_crtc_id=0
smithay_kms_scanout_primary_plane_id=0
smithay_kms_scanout_mode_width=0
smithay_kms_scanout_mode_height=0
smithay_kms_scanout_mode_refresh_hz=0
smithay_kms_scanout_mode_preferred=false
smithay_renderer_node_selected=false
smithay_renderer_node_opened=false
smithay_gbm_device_created=false
smithay_gbm_allocator_created=false
smithay_egl_display_created=false
smithay_egl_context_created=false
smithay_gles_renderer_created=false
smithay_offscreen_buffer_created=false
smithay_offscreen_frame_rendered=false
smithay_offscreen_frame_copied=false
smithay_offscreen_pixel_verified=false
smithay_offscreen_render_width=0
smithay_offscreen_render_height=0
smithay_offscreen_render_pixels=0
smithay_offscreen_sample_red=0
smithay_offscreen_sample_green=0
smithay_offscreen_sample_blue=0
smithay_offscreen_sample_alpha=0
smithay_libseat_session_created=false
smithay_libseat_event_source_inserted=false
smithay_libseat_event_loop_dispatched=false
smithay_libinput_context_created=false
smithay_libinput_seat_assigned=false
smithay_libinput_backend_created=false
smithay_libinput_event_source_inserted=false
smithay_libinput_event_loop_dispatched=false
if grep -F '"event":"backend.preflight","backend":"drm","ready":true' "$log" >/dev/null; then
  launch_ready=true
  expected_blocked=false
  require_contains "$log" '"launch_ready":true'
  require_contains "$log" '"passed":true'
  require_contains "$log" '"drm_card_selected":true'
  require_contains "$log" '"drm_node_resolved":true'
  require_contains "$log" '"drm_node_type":"primary"'
  require_contains "$log" '"drm_node_primary_path":"/dev/dri/card'
  require_contains "$log" '"drm_node_render_path":"/dev/dri/renderD'
  require_contains "$log" '"kms_card_opened":true'
  require_contains "$log" '"kms_device_created":true'
  require_contains "$log" '"kms_event_source_inserted":true'
  require_contains "$log" '"kms_event_loop_dispatched":true'
  require_matches "$log" '"kms_atomic_modesetting":(true|false)'
  require_matches "$log" '"kms_crtc_count":[1-9][0-9]*'
  require_matches "$log" '"kms_connector_count":[1-9][0-9]*'
  require_matches "$log" '"kms_connected_connector_count":[1-9][0-9]*'
  require_matches "$log" '"kms_mode_count":[1-9][0-9]*'
  require_matches "$log" '"kms_primary_plane_count":[1-9][0-9]*'
  require_matches "$log" '"kms_cursor_plane_count":[0-9][0-9]*'
  require_matches "$log" '"kms_overlay_plane_count":[0-9][0-9]*'
  require_contains "$log" '"kms_scanout_plan_ready":true'
  require_matches "$log" '"kms_scanout_connector_id":[1-9][0-9]*'
  require_matches "$log" '"kms_scanout_connector_name":"[^"]+"'
  require_matches "$log" '"kms_scanout_crtc_id":[1-9][0-9]*'
  require_matches "$log" '"kms_scanout_primary_plane_id":[1-9][0-9]*'
  require_matches "$log" '"kms_scanout_mode_width":[1-9][0-9]*'
  require_matches "$log" '"kms_scanout_mode_height":[1-9][0-9]*'
  require_matches "$log" '"kms_scanout_mode_refresh_hz":[1-9][0-9]*'
  require_matches "$log" '"kms_scanout_mode_preferred":(true|false)'
  require_contains "$log" '"kms_resource_failure":""'
  require_contains "$log" '"renderer_node_selected":true'
  require_contains "$log" '"renderer_node_path":"/dev/dri/renderD'
  require_contains "$log" '"renderer_node_opened":true'
  require_contains "$log" '"gbm_device_created":true'
  require_contains "$log" '"gbm_allocator_created":true'
  require_contains "$log" '"egl_display_created":true'
  require_contains "$log" '"egl_context_created":true'
  require_contains "$log" '"gles_renderer_created":true'
  require_contains "$log" '"offscreen_buffer_created":true'
  require_contains "$log" '"offscreen_frame_rendered":true'
  require_contains "$log" '"offscreen_frame_copied":true'
  require_contains "$log" '"offscreen_pixel_verified":true'
  require_contains "$log" '"offscreen_render_width":16'
  require_contains "$log" '"offscreen_render_height":16'
  require_contains "$log" '"offscreen_render_pixels":256'
  require_contains "$log" '"offscreen_sample_red":255'
  require_contains "$log" '"offscreen_sample_green":0'
  require_contains "$log" '"offscreen_sample_blue":0'
  require_contains "$log" '"offscreen_sample_alpha":255'
  require_contains "$log" '"renderer_runtime_failure":""'
  require_contains "$log" '"input_event_selected":true'
  require_contains "$log" '"uses_logind":true'
  require_contains "$log" '"uses_libseat":true'
  require_contains "$log" '"uses_libinput":true'
  require_contains "$log" '"libseat_session_created":true'
  require_contains "$log" '"libseat_session_seat":"seat'
  require_contains "$log" '"libseat_event_source_inserted":true'
  require_contains "$log" '"libseat_event_loop_dispatched":true'
  require_contains "$log" '"libinput_context_created":true'
  require_contains "$log" '"libinput_seat_assigned":true'
  require_contains "$log" '"libinput_backend_created":true'
  require_contains "$log" '"libinput_event_source_inserted":true'
  require_contains "$log" '"libinput_event_loop_dispatched":true'
  require_contains "$log" '"input_runtime_failure":""'
  require_contains "$log" '"display_created":true'
  require_contains "$log" '"display_handle_created":true'
  require_contains "$log" '"listening_socket_bound":true'
  require_contains "$log" '"socket_name":"backlit-smithay-bootstrap-'
  require_contains "$log" '"socket_connect_succeeded":true'
  require_contains "$log" '"socket_accept_succeeded":true'
  require_contains "$log" '"client_inserted":true'
  require_contains "$log" '"display_clients_dispatched":true'
  require_contains "$log" '"display_clients_flushed":true'
  require_contains "$log" '"event_loop_created":true'
  require_contains "$log" '"event_loop_dispatched":true'
  require_contains "$log" '"failure":""'
  smithay_runtime_probe=true
  smithay_runtime_bootstrap=true
  smithay_wayland_socket_bootstrap=true
  smithay_wayland_client_inserted=true
  smithay_drm_node_resolved=true
  smithay_kms_card_opened=true
  smithay_kms_device_created=true
  smithay_kms_event_source_inserted=true
  smithay_kms_event_loop_dispatched=true
  if grep -F '"kms_atomic_modesetting":true' "$log" >/dev/null; then
    smithay_kms_atomic_modesetting=true
  fi
  smithay_kms_crtc_count="$(extract_u64 "$log" kms_crtc_count)"
  smithay_kms_connector_count="$(extract_u64 "$log" kms_connector_count)"
  smithay_kms_connected_connector_count="$(extract_u64 "$log" kms_connected_connector_count)"
  smithay_kms_mode_count="$(extract_u64 "$log" kms_mode_count)"
  smithay_kms_primary_plane_count="$(extract_u64 "$log" kms_primary_plane_count)"
  smithay_kms_cursor_plane_count="$(extract_u64 "$log" kms_cursor_plane_count)"
  smithay_kms_overlay_plane_count="$(extract_u64 "$log" kms_overlay_plane_count)"
  smithay_kms_scanout_plan_ready=true
  smithay_kms_scanout_connector_id="$(extract_u64 "$log" kms_scanout_connector_id)"
  smithay_kms_scanout_crtc_id="$(extract_u64 "$log" kms_scanout_crtc_id)"
  smithay_kms_scanout_primary_plane_id="$(extract_u64 "$log" kms_scanout_primary_plane_id)"
  smithay_kms_scanout_mode_width="$(extract_u64 "$log" kms_scanout_mode_width)"
  smithay_kms_scanout_mode_height="$(extract_u64 "$log" kms_scanout_mode_height)"
  smithay_kms_scanout_mode_refresh_hz="$(extract_u64 "$log" kms_scanout_mode_refresh_hz)"
  if grep -F '"kms_scanout_mode_preferred":true' "$log" >/dev/null; then
    smithay_kms_scanout_mode_preferred=true
  fi
  smithay_renderer_node_selected=true
  smithay_renderer_node_opened=true
  smithay_gbm_device_created=true
  smithay_gbm_allocator_created=true
  smithay_egl_display_created=true
  smithay_egl_context_created=true
  smithay_gles_renderer_created=true
  smithay_offscreen_buffer_created=true
  smithay_offscreen_frame_rendered=true
  smithay_offscreen_frame_copied=true
  smithay_offscreen_pixel_verified=true
  smithay_offscreen_render_width=16
  smithay_offscreen_render_height=16
  smithay_offscreen_render_pixels=256
  smithay_offscreen_sample_red=255
  smithay_offscreen_sample_green=0
  smithay_offscreen_sample_blue=0
  smithay_offscreen_sample_alpha=255
  smithay_libseat_session_created=true
  smithay_libseat_event_source_inserted=true
  smithay_libseat_event_loop_dispatched=true
  smithay_libinput_context_created=true
  smithay_libinput_seat_assigned=true
  smithay_libinput_backend_created=true
  smithay_libinput_event_source_inserted=true
  smithay_libinput_event_loop_dispatched=true
else
  require_contains "$log" '"event":"backend.preflight","backend":"drm","ready":false'
  require_contains "$log" '"launch_ready":false'
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-smithay-runtime-probe",
  "passed": true,
  "checked": true,
  "expected_blocked": $expected_blocked,
  "artifacts": {
    "log": "$log"
  },
  "checks": {
    "smithay_dependency_compiled": true,
    "smithay_backend_feature": true,
    "smithay_drm_component": true,
    "smithay_gbm_allocator_component": true,
    "smithay_egl_display_component": true,
    "smithay_gles_renderer_component": true,
    "smithay_kms_card_opened": $smithay_kms_card_opened,
    "smithay_kms_device_created": $smithay_kms_device_created,
    "smithay_kms_event_source_inserted": $smithay_kms_event_source_inserted,
    "smithay_kms_event_loop_dispatched": $smithay_kms_event_loop_dispatched,
    "smithay_kms_atomic_modesetting": $smithay_kms_atomic_modesetting,
    "smithay_kms_crtc_count": $smithay_kms_crtc_count,
    "smithay_kms_connector_count": $smithay_kms_connector_count,
    "smithay_kms_connected_connector_count": $smithay_kms_connected_connector_count,
    "smithay_kms_mode_count": $smithay_kms_mode_count,
    "smithay_kms_primary_plane_count": $smithay_kms_primary_plane_count,
    "smithay_kms_cursor_plane_count": $smithay_kms_cursor_plane_count,
    "smithay_kms_overlay_plane_count": $smithay_kms_overlay_plane_count,
    "smithay_kms_scanout_plan_ready": $smithay_kms_scanout_plan_ready,
    "smithay_kms_scanout_connector_id": $smithay_kms_scanout_connector_id,
    "smithay_kms_scanout_crtc_id": $smithay_kms_scanout_crtc_id,
    "smithay_kms_scanout_primary_plane_id": $smithay_kms_scanout_primary_plane_id,
    "smithay_kms_scanout_mode_width": $smithay_kms_scanout_mode_width,
    "smithay_kms_scanout_mode_height": $smithay_kms_scanout_mode_height,
    "smithay_kms_scanout_mode_refresh_hz": $smithay_kms_scanout_mode_refresh_hz,
    "smithay_kms_scanout_mode_preferred": $smithay_kms_scanout_mode_preferred,
    "smithay_renderer_node_opened": $smithay_renderer_node_opened,
    "smithay_gbm_device_created": $smithay_gbm_device_created,
    "smithay_gbm_allocator_created": $smithay_gbm_allocator_created,
    "smithay_egl_display_created": $smithay_egl_display_created,
    "smithay_egl_context_created": $smithay_egl_context_created,
    "smithay_gles_renderer_created": $smithay_gles_renderer_created,
    "smithay_offscreen_buffer_created": $smithay_offscreen_buffer_created,
    "smithay_offscreen_frame_rendered": $smithay_offscreen_frame_rendered,
    "smithay_offscreen_frame_copied": $smithay_offscreen_frame_copied,
    "smithay_offscreen_pixel_verified": $smithay_offscreen_pixel_verified,
    "smithay_offscreen_render_width": $smithay_offscreen_render_width,
    "smithay_offscreen_render_height": $smithay_offscreen_render_height,
    "smithay_offscreen_render_pixels": $smithay_offscreen_render_pixels,
    "smithay_offscreen_sample_red": $smithay_offscreen_sample_red,
    "smithay_offscreen_sample_green": $smithay_offscreen_sample_green,
    "smithay_offscreen_sample_blue": $smithay_offscreen_sample_blue,
    "smithay_offscreen_sample_alpha": $smithay_offscreen_sample_alpha,
    "smithay_drm_node_resolved": $smithay_drm_node_resolved,
    "smithay_renderer_node_selected": $smithay_renderer_node_selected,
    "smithay_libseat_session_created": $smithay_libseat_session_created,
    "smithay_libseat_event_source_inserted": $smithay_libseat_event_source_inserted,
    "smithay_libseat_event_loop_dispatched": $smithay_libseat_event_loop_dispatched,
    "smithay_libinput_context_created": $smithay_libinput_context_created,
    "smithay_libinput_seat_assigned": $smithay_libinput_seat_assigned,
    "smithay_libinput_backend_created": $smithay_libinput_backend_created,
    "smithay_libinput_event_source_inserted": $smithay_libinput_event_source_inserted,
    "smithay_libinput_event_loop_dispatched": $smithay_libinput_event_loop_dispatched,
    "smithay_libinput_component": true,
    "smithay_libseat_session_component": true,
    "smithay_calloop_component": true,
    "smithay_wayland_display_bootstrap": $smithay_runtime_bootstrap,
    "smithay_wayland_socket_bootstrap": $smithay_wayland_socket_bootstrap,
    "smithay_wayland_client_inserted": $smithay_wayland_client_inserted,
    "smithay_calloop_dispatch_bootstrap": $smithay_runtime_bootstrap,
    "drm_launch_ready": $launch_ready,
    "smithay_runtime_probe": $smithay_runtime_probe,
    "smithay_runtime_bootstrap": $smithay_runtime_bootstrap
  }
}
EOF

printf 'Backlit Smithay runtime probe verification passed. Artifacts: %s\n' "$out_dir"
