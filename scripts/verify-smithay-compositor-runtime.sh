#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/smithay-compositor-runtime}"
mkdir -p "$out_dir"

log="$out_dir/smithay-compositor-runtime.jsonl"
err="$out_dir/smithay-compositor-runtime.stderr"
client_smoke_log="$out_dir/smithay-wayland-client-smoke.jsonl"
client_smoke_err="$out_dir/smithay-wayland-client-smoke.stderr"
first_present_log="$out_dir/smithay-first-present-probe.jsonl"
first_present_err="$out_dir/smithay-first-present-probe.stderr"
service_log="$out_dir/smithay-compositor-service.jsonl"
service_err="$out_dir/smithay-compositor-service.stderr"
normal_frame_ppm="$out_dir/smithay-normal-runtime-real-client-frame.ppm"
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

require_matches() {
  file="$1"
  value="$2"
  grep -E "$value" "$file" >/dev/null || fail "missing pattern in $file: $value"
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
    "client_smoke_log": "$client_smoke_log",
    "client_smoke_stderr": "$client_smoke_err",
    "first_present_log": "$first_present_log",
    "first_present_stderr": "$first_present_err",
    "service_log": "$service_log",
    "service_stderr": "$service_err",
    "normal_frame_ppm": "$normal_frame_ppm",
    "first_demo_client_log": "$first_demo_client_log",
    "demo_client_log": "$demo_client_log"
  },
  "checks": {
    "smithay_compositor_runtime": false,
    "smithay_runtime_trait": false,
    "smithay_runtime_launch_plan": false,
    "smithay_scripted_client": false,
    "smithay_core_protocol_globals": false,
    "smithay_mvp_protocol_globals": false,
    "smithay_seat_global": false,
    "smithay_keyboard_pointer_capabilities": false,
    "smithay_input_sources": false,
    "smithay_input_event_loop": false,
    "smithay_input_seat_handles": false,
    "smithay_input_seat_dispatch": false,
    "smithay_input_event_classification": false,
    "smithay_real_wayland_client": false,
    "smithay_real_wayland_metadata": false,
    "smithay_real_shm_buffer": false,
    "smithay_normal_runtime_live_snapshot_frame": false,
    "smithay_normal_runtime_real_pixels": false,
    "smithay_real_surface_lifecycle": false,
    "smithay_real_xdg_resize_commit": false,
    "smithay_real_xdg_unmap_cleanup": false,
    "smithay_real_xdg_close_disconnect": false,
    "smithay_policy_lifecycle_cleanup": false,
    "smithay_real_client_input": false,
    "smithay_real_pointer_input": false,
    "smithay_real_keyboard_input": false,
    "smithay_real_input_focus_routing": false,
    "smithay_shortcut_filter_preserved": false,
    "smithay_real_wayland_policy_window": false,
    "smithay_event_loop_runtime": false,
    "smithay_drm_first_present_probe": false,
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
  --scripted-client \
  --scripted-client-preview "$normal_frame_ppm" > "$log" 2> "$err"
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
require_contains "$log" '"event":"compositor.backend_launch_plan"'
require_contains "$log" '"implementation":"smithay-compositor-runtime"'
require_contains "$log" '"event":"compositor.scripted_client"'
require_contains "$log" '"passed":true'
require_contains "$log" '"runtime_backend":"smithay-compositor-runtime"'
require_contains "$log" '"runtime_trait":true'
require_line_contains_all "$log" \
  '"event":"compositor.scripted_client"' \
  '"smithay_protocol_globals":10' \
  '"input_sources_ready":true' \
  '"input_source_count":2' \
  '"input_seat_ready":true' \
  '"input_keyboard_handle_ready":true' \
  '"input_pointer_handle_ready":true' \
  '"input_seat_dispatch_count":' \
  '"input_keyboard_dispatch_count":' \
  '"input_pointer_dispatch_count":' \
  '"input_event_count":' \
  '"input_keyboard_event_count":' \
  '"input_pointer_event_count":' \
  '"input_special_event_count":'
require_matches "$log" '"inserted_wayland_clients":([2-9]|[1-9][0-9]+)'
require_matches "$log" '"wayland_dispatch_count":([8-9]|[1-9][0-9]+)'
require_matches "$log" '"calloop_dispatch_count":([8-9]|[1-9][0-9]+)'
require_matches "$log" '"input_event_loop_dispatch_count":([8-9]|[1-9][0-9]+)'
require_contains "$log" '"client_connected":true'
require_contains "$log" '"surfaces_after_map":2'
require_contains "$log" '"targeted_damage_ok":true'
require_contains "$log" '"clean_disconnect":true'
require_line_contains_all "$log" \
  '"event":"compositor.scripted_client"' \
  '"normal_frame_uses_live_snapshot":true' \
  '"normal_frame_real_wayland_client":true' \
  '"normal_frame_live_snapshot_presented":true' \
  '"normal_frame_policy_window_from_snapshot":true' \
  '"normal_frame_policy_geometry_preserved":true' \
  '"normal_frame_pixels_composited":true' \
  '"normal_frame_samples_verified":true' \
  '"normal_frame_ppm_written":true' \
  '"normal_frame_surface_count":1' \
  '"normal_frame_damaged_surfaces":1' \
  '"normal_frame_presented_pixels":76800' \
  '"normal_frame_snapshot_width":320' \
  '"normal_frame_snapshot_height":240' \
  '"normal_frame_snapshot_pixel_count":76800' \
  '"normal_frame_composited_pixels":76800'
require_matches "$log" '"normal_frame_client_count":[1-9][0-9]*'
require_matches "$log" '"normal_frame_ppm_bytes":[1-9][0-9]*'
require_matches "$log" '"normal_frame_checksum":[1-9][0-9]*'
require_line_contains_all "$log" \
  '"event":"compositor.scripted_client"' \
  '"real_surface_lifecycle":true' \
  '"real_surface_configure_acked":true' \
  '"real_surface_resize_configured":true' \
  '"real_surface_resize_committed":true' \
  '"real_surface_unmapped":true' \
  '"real_surface_close_sent":true' \
  '"real_surface_close_received":true' \
  '"real_surface_destroyed":true' \
  '"real_surface_client_disconnected":true' \
  '"real_surface_policy_window_mapped":true' \
  '"real_surface_policy_window_resized":true' \
  '"real_surface_policy_focus_preserved":true' \
  '"real_surface_policy_window_removed_after_unmap":true' \
  '"real_surface_policy_no_stale_windows_after_disconnect":true' \
  '"real_surface_initial_width":320' \
  '"real_surface_initial_height":240' \
  '"real_surface_resized_width":420' \
  '"real_surface_resized_height":300' \
  '"real_surface_resized_pixel_count":126000'
require_matches "$log" '"real_surface_configure_sent_count":([2-9]|[1-9][0-9]+)'
require_matches "$log" '"real_surface_configure_ack_count":([2-9]|[1-9][0-9]+)'
require_matches "$log" '"real_surface_unmap_count":[1-9][0-9]*'
require_matches "$log" '"real_surface_toplevel_destroyed_count":[1-9][0-9]*'
require_matches "$log" '"real_surface_resized_checksum":[1-9][0-9]*'
require_line_contains_all "$log" \
  '"event":"compositor.scripted_client"' \
  '"real_input_to_clients":true' \
  '"real_input_pointer_entered":true' \
  '"real_input_pointer_motion":true' \
  '"real_input_pointer_button":true' \
  '"real_input_keyboard_entered":true' \
  '"real_input_keyboard_key":true' \
  '"real_input_focus_routed_to_second_client":true' \
  '"real_input_shortcut_filter_preserved":true'
require_matches "$log" '"real_input_primary_pointer_button_events":([2-9]|[1-9][0-9]+)'
require_matches "$log" '"real_input_secondary_pointer_button_events":([2-9]|[1-9][0-9]+)'
require_matches "$log" '"real_input_primary_key_events":([2-9]|[1-9][0-9]+)'
require_matches "$log" '"real_input_secondary_key_events":([2-9]|[1-9][0-9]+)'
require_matches "$log" '"real_input_keyboard_focus_set_count":([4-9]|[1-9][0-9]+)'
require_matches "$log" '"real_input_pointer_focus_set_count":([2-9]|[1-9][0-9]+)'
require_matches "$log" '"real_input_shortcut_intercept_count":([2-9]|[1-9][0-9]+)'
require_matches "$log" '"real_input_forwarded_key_count":([6-9]|[1-9][0-9]+)'
test -s "$normal_frame_ppm" || fail "missing Smithay normal runtime real client frame $normal_frame_ppm"
require_contains "$log" '"event":"compositor.ready"'
require_contains "$log" '"ready":true'
require_line_contains_all "$log" \
  '"event":"compositor.ready"' \
  '"inserted_wayland_clients":1' \
  '"smithay_protocol_globals":10' \
  '"wayland_dispatch_count":1' \
  '"calloop_dispatch_count":1' \
  '"input_sources_ready":true' \
  '"input_source_count":2' \
  '"input_event_loop_dispatch_count":1' \
  '"input_seat_ready":true' \
  '"input_keyboard_handle_ready":true' \
  '"input_pointer_handle_ready":true' \
  '"input_seat_dispatch_count":' \
  '"input_keyboard_dispatch_count":' \
  '"input_pointer_dispatch_count":' \
  '"input_event_count":' \
  '"input_keyboard_event_count":' \
  '"input_pointer_event_count":' \
  '"input_special_event_count":'
require_contains "$log" '"bootstrap_client_connected":true'
require_contains "$log" '"bootstrap_surface_presented":true'

set +e
target/debug/backlit-compositor \
  --backend=drm \
  --runtime=smithay \
  --drm-first-present-probe > "$first_present_log" 2> "$first_present_err"
first_present_status=$?
set -e

if [ "$first_present_status" -ne 0 ]; then
  cat "$first_present_log" >&2 || true
  cat "$first_present_err" >&2 || true
  fail "DRM first-present probe exited with status $first_present_status on a launch-ready host"
fi

require_contains "$first_present_log" '"event":"compositor.start"'
require_contains "$first_present_log" '"drm_first_present_probe":true'
require_contains "$first_present_log" '"event":"compositor.backend_launch_plan"'
require_contains "$first_present_log" '"implementation":"smithay-compositor-runtime"'
require_contains "$first_present_log" '"event":"compositor.drm_first_present_probe"'
require_contains "$first_present_log" '"passed":true'
require_contains "$first_present_log" '"runtime_backend":"smithay-drm-probe"'
require_contains "$first_present_log" '"feature_enabled":true'
require_contains "$first_present_log" '"compiled":true'
require_contains "$first_present_log" '"launch_ready":true'
require_contains "$first_present_log" '"drm_card_selected":true'
require_contains "$first_present_log" '"drm_node_resolved":true'
require_contains "$first_present_log" '"kms_scanout_plan_ready":true'
require_contains "$first_present_log" '"kms_surface_created":true'
require_contains "$first_present_log" '"kms_framebuffer_created":true'
require_contains "$first_present_log" '"kms_framebuffer_added":true'
require_contains "$first_present_log" '"kms_first_present_framebuffer_filled":true'
require_contains "$first_present_log" '"kms_first_present_plane_state_ready":true'
require_matches "$first_present_log" '"kms_framebuffer_test_state_(succeeded|permission_denied)":true'
require_matches "$first_present_log" '"kms_first_present_(commit_succeeded|blocked_by_drm_master)":true'
if grep -F '"kms_first_present_commit_succeeded":true' "$first_present_log" >/dev/null; then
  require_contains "$first_present_log" '"kms_first_present_vblank_event_received":true'
else
  require_contains "$first_present_log" '"kms_first_present_blocked_by_drm_master":true'
  require_contains "$first_present_log" '"kms_framebuffer_test_state_permission_denied":true'
fi
require_contains "$first_present_log" '"kms_first_present_failure":""'
require_matches "$first_present_log" '"libinput_event_count":[0-9][0-9]*'
require_matches "$first_present_log" '"libinput_keyboard_event_count":[0-9][0-9]*'
require_matches "$first_present_log" '"libinput_pointer_event_count":[0-9][0-9]*'
require_matches "$first_present_log" '"libinput_special_event_count":[0-9][0-9]*'
require_contains "$first_present_log" '"event":"compositor.exit"'

set +e
target/debug/backlit-compositor \
  --backend=drm \
  --runtime=smithay \
  --smithay-client-smoke > "$client_smoke_log" 2> "$client_smoke_err"
client_smoke_status=$?
set -e

if [ "$client_smoke_status" -ne 0 ]; then
  cat "$client_smoke_log" >&2 || true
  cat "$client_smoke_err" >&2 || true
  fail "Smithay Wayland client smoke exited with status $client_smoke_status on a launch-ready host"
fi

require_contains "$client_smoke_log" '"event":"compositor.smithay_client_smoke"'
require_contains "$client_smoke_log" '"event":"compositor.backend_launch_plan"'
require_contains "$client_smoke_log" '"implementation":"smithay-compositor-runtime"'
require_contains "$client_smoke_log" '"passed":true'
require_contains "$client_smoke_log" '"runtime_backend":"smithay-compositor-runtime"'
require_line_contains_all "$client_smoke_log" \
  '"event":"compositor.smithay_client_smoke"' \
  '"smithay_protocol_globals":10' \
  '"registry_global_count":10' \
  '"registry_announced":true' \
  '"mvp_protocol_globals":7' \
  '"mvp_protocol_globals_announced":true' \
  '"wl_output_bound":true' \
  '"xdg_output_manager_bound":true' \
  '"viewporter_bound":true' \
  '"presentation_bound":true' \
  '"linux_dmabuf_bound":true' \
  '"linux_dmabuf_version":5' \
  '"linux_dmabuf_version_at_least_4":true' \
  '"seat_global_announced":true' \
  '"seat_bound":true' \
  '"seat_name_observed":true' \
  '"seat_keyboard_capability":true' \
  '"seat_pointer_capability":true' \
  '"keyboard_bound":true' \
  '"pointer_bound":true' \
  '"input_sources_ready":true' \
  '"input_source_count":2' \
  '"input_seat_ready":true' \
  '"input_keyboard_handle_ready":true' \
  '"input_pointer_handle_ready":true' \
  '"input_seat_dispatch_count":' \
  '"input_keyboard_dispatch_count":' \
  '"input_pointer_dispatch_count":' \
  '"input_event_count":' \
  '"input_keyboard_event_count":' \
  '"input_pointer_event_count":' \
  '"input_special_event_count":' \
  '"compositor_bound":true' \
  '"shm_bound":true' \
  '"shm_buffer_created":true' \
  '"shm_buffer_attached":true' \
  '"xdg_wm_base_bound":true' \
  '"surface_created":true' \
  '"xdg_toplevel_created":true' \
  '"configure_received":true' \
  '"configure_acked":true' \
  '"surface_committed":true' \
  '"surface_commit_count":2' \
  '"xdg_toplevel_count":1' \
  '"title_changed_count":1' \
  '"app_id_changed_count":1' \
  '"observed_title":"Backlit Smithay smoke"' \
  '"observed_app_id":"org.backlit.SmithaySmoke"' \
  '"title_matched":true' \
  '"app_id_matched":true' \
  '"shm_buffer_commit_count":1' \
  '"shm_buffer_width":320' \
  '"shm_buffer_height":240' \
  '"shm_buffer_pixels":76800' \
  '"policy_window_mapped":true' \
  '"policy_app_id_preserved":true' \
  '"policy_focused_after_map":true' \
  '"policy_geometry_preserved":true' \
  '"policy_windows":1' \
  '"policy_backend_surface_presented":true' \
  '"policy_presented_pixels":76800'
require_matches "$client_smoke_log" '"input_event_loop_dispatch_count":[3-9][0-9]*'

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
require_contains "$service_log" '"event":"compositor.backend_launch_plan"'
require_contains "$service_log" '"implementation":"smithay-compositor-runtime"'
require_contains "$service_log" '"event":"compositor.ready"'
require_contains "$service_log" '"runtime_backend":"smithay-compositor-runtime"'
require_contains "$service_log" '"ready":true'
require_line_contains_all "$service_log" \
  '"event":"compositor.ready"' \
  '"inserted_wayland_clients":1' \
  '"smithay_protocol_globals":10' \
  '"wayland_dispatch_count":1' \
  '"calloop_dispatch_count":1' \
  '"input_sources_ready":true' \
  '"input_source_count":2' \
  '"input_event_loop_dispatch_count":1' \
  '"input_seat_ready":true' \
  '"input_keyboard_handle_ready":true' \
  '"input_pointer_handle_ready":true' \
  '"input_seat_dispatch_count":' \
  '"input_keyboard_dispatch_count":' \
  '"input_pointer_dispatch_count":' \
  '"input_event_count":' \
  '"input_keyboard_event_count":' \
  '"input_pointer_event_count":' \
  '"input_special_event_count":'
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
  '"smithay_protocol_globals":10' \
  '"wayland_dispatch_count":1' \
  '"calloop_dispatch_count":1' \
  '"input_sources_ready":true' \
  '"input_source_count":2' \
  '"input_event_loop_dispatch_count":1' \
  '"input_seat_ready":true' \
  '"input_keyboard_handle_ready":true' \
  '"input_pointer_handle_ready":true' \
  '"input_seat_dispatch_count":' \
  '"input_keyboard_dispatch_count":' \
  '"input_pointer_dispatch_count":' \
  '"input_event_count":' \
  '"input_keyboard_event_count":' \
  '"input_pointer_event_count":' \
  '"input_special_event_count":' \
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
  '"smithay_protocol_globals":10' \
  '"wayland_dispatch_count":2' \
  '"calloop_dispatch_count":2' \
  '"input_sources_ready":true' \
  '"input_source_count":2' \
  '"input_event_loop_dispatch_count":2'
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
  '"smithay_protocol_globals":10' \
  '"wayland_dispatch_count":8' \
  '"calloop_dispatch_count":8' \
  '"input_sources_ready":true' \
  '"input_source_count":2' \
  '"input_event_loop_dispatch_count":8' \
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
    "client_smoke_log": "$client_smoke_log",
    "client_smoke_stderr": "$client_smoke_err",
    "first_present_log": "$first_present_log",
    "first_present_stderr": "$first_present_err",
    "service_log": "$service_log",
    "service_stderr": "$service_err",
    "normal_frame_ppm": "$normal_frame_ppm",
    "first_demo_client_log": "$first_demo_client_log",
    "demo_client_log": "$demo_client_log"
  },
  "checks": {
    "smithay_compositor_runtime": true,
    "smithay_runtime_trait": true,
    "smithay_runtime_launch_plan": true,
    "smithay_scripted_client": true,
    "smithay_core_protocol_globals": true,
    "smithay_mvp_protocol_globals": true,
    "smithay_seat_global": true,
    "smithay_keyboard_pointer_capabilities": true,
    "smithay_input_sources": true,
    "smithay_input_event_loop": true,
    "smithay_input_seat_handles": true,
    "smithay_input_seat_dispatch": true,
    "smithay_input_event_classification": true,
    "smithay_real_wayland_client": true,
    "smithay_real_wayland_metadata": true,
    "smithay_real_shm_buffer": true,
    "smithay_normal_runtime_live_snapshot_frame": true,
    "smithay_normal_runtime_real_pixels": true,
    "smithay_real_surface_lifecycle": true,
    "smithay_real_xdg_resize_commit": true,
    "smithay_real_xdg_unmap_cleanup": true,
    "smithay_real_xdg_close_disconnect": true,
    "smithay_policy_lifecycle_cleanup": true,
    "smithay_real_client_input": true,
    "smithay_real_pointer_input": true,
    "smithay_real_keyboard_input": true,
    "smithay_real_input_focus_routing": true,
    "smithay_shortcut_filter_preserved": true,
    "smithay_real_wayland_policy_window": true,
    "smithay_event_loop_runtime": true,
    "smithay_drm_first_present_probe": true,
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
