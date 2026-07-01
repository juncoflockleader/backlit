#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/drm-session-smoke}"
mkdir -p "$out_dir"

session_log="$out_dir/session.jsonl"
session_err="$out_dir/session.stderr"
session_screenshot="$out_dir/drm-session.ppm"
service_log_dir="$out_dir/session-services"
expected_checksum="15888844850457870477"
expected_ppm_bytes="1248015"

fail() {
  echo "DRM session smoke verification failed: $*" >&2
  exit 1
}

require_matches() {
  file="$1"
  value="$2"
  grep -E "$value" "$file" >/dev/null || fail "missing pattern in $file: $value"
}

count_matching() {
  dir="$1"
  pattern="$2"

  if [ ! -d "$dir" ]; then
    printf '0'
    return
  fi

  find "$dir" -maxdepth 1 -name "$pattern" -print 2>/dev/null | wc -l | tr -d ' '
}

count_accessible_matching() {
  dir="$1"
  pattern="$2"
  mode="$3"
  count=0

  if [ ! -d "$dir" ]; then
    printf '0'
    return
  fi

  for path in "$dir"/$pattern; do
    if [ ! -e "$path" ]; then
      continue
    fi
    if [ "$mode" = "read" ] && [ -r "$path" ]; then
      count=$((count + 1))
    elif [ "$mode" = "write" ] && [ -w "$path" ]; then
      count=$((count + 1))
    fi
  done

  printf '%s' "$count"
}

runtime_present=false
if [ -n "${XDG_RUNTIME_DIR:-}" ] && [ -d "${XDG_RUNTIME_DIR:-}" ]; then
  runtime_present=true
fi

session_present=false
if [ -n "${XDG_SESSION_ID:-}" ]; then
  session_present=true
fi

session_active=false
session_remote=false
session_state=""
session_seat="${XDG_SEAT:-}"
session_type="${XDG_SESSION_TYPE:-}"
logind_available=false
if command -v loginctl >/dev/null 2>&1; then
  logind_available=true
fi

libseat_available=false
if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists libseat 2>/dev/null; then
  libseat_available=true
fi

libinput_available=false
if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists libinput 2>/dev/null; then
  libinput_available=true
fi

if [ "$session_present" = true ] && [ "$logind_available" = true ]; then
  session_active_value="$(loginctl show-session "$XDG_SESSION_ID" -p Active --value 2>/dev/null || true)"
  session_remote_value="$(loginctl show-session "$XDG_SESSION_ID" -p Remote --value 2>/dev/null || true)"
  session_state="$(loginctl show-session "$XDG_SESSION_ID" -p State --value 2>/dev/null || true)"
  logind_seat="$(loginctl show-session "$XDG_SESSION_ID" -p Seat --value 2>/dev/null || true)"
  logind_type="$(loginctl show-session "$XDG_SESSION_ID" -p Type --value 2>/dev/null || true)"
  if [ "$session_active_value" = "yes" ]; then
    session_active=true
  fi
  if [ "$session_remote_value" = "yes" ]; then
    session_remote=true
  fi
  if [ -n "$logind_seat" ]; then
    session_seat="$logind_seat"
  fi
  if [ -n "$logind_type" ]; then
    session_type="$logind_type"
  fi
fi

session_local=false
if [ "$session_active" = true ] \
  && [ "$session_remote" = false ] \
  && [ -n "$session_seat" ] \
  && [ -n "$session_type" ] \
  && [ "$session_type" != "unspecified" ]; then
  session_local=true
fi

runtime_owned_by_user=false
if [ "$(uname -s)" = "Linux" ] && [ "$runtime_present" = true ]; then
  runtime_owner_uid="$(stat -c '%u' "$XDG_RUNTIME_DIR" 2>/dev/null || printf unknown)"
  current_uid="$(id -u)"
  if [ "$runtime_owner_uid" = "$current_uid" ]; then
    runtime_owned_by_user=true
  fi
fi

drm_card_nodes="$(count_matching /dev/dri 'card*')"
drm_render_nodes="$(count_matching /dev/dri 'renderD*')"
input_event_nodes="$(count_matching /dev/input 'event*')"
drm_card_readable="$(count_accessible_matching /dev/dri 'card*' read)"
drm_card_writable="$(count_accessible_matching /dev/dri 'card*' write)"
drm_render_readable="$(count_accessible_matching /dev/dri 'renderD*' read)"
drm_render_writable="$(count_accessible_matching /dev/dri 'renderD*' write)"
input_event_readable="$(count_accessible_matching /dev/input 'event*' read)"
drm_node_count=$((drm_card_nodes + drm_render_nodes))

drm_card_access_ready=false
if [ "$drm_card_nodes" -gt 0 ] \
  && [ "$drm_card_readable" -gt 0 ] \
  && [ "$drm_card_writable" -gt 0 ]; then
  drm_card_access_ready=true
fi

input_requires_logind_broker=false
if [ "$input_event_nodes" -gt 0 ] && [ "$input_event_readable" -eq 0 ]; then
  input_requires_logind_broker=true
fi

input_broker_ready=false
input_broker_mode="missing"
if [ "$input_event_nodes" -gt 0 ] && [ "$input_event_readable" -gt 0 ]; then
  input_broker_ready=true
  input_broker_mode="direct"
elif [ "$input_requires_logind_broker" = true ] \
  && [ "$logind_available" = true ] \
  && [ "$session_local" = true ] \
  && [ "$libseat_available" = true ] \
  && [ "$libinput_available" = true ]; then
  input_broker_ready=true
  input_broker_mode="logind-libseat"
fi

drm_expected_ready=false
if [ "$(uname -s)" = "Linux" ] \
  && [ "$runtime_present" = true ] \
  && [ "$runtime_owned_by_user" = true ] \
  && [ "$session_present" = true ] \
  && [ "$session_local" = true ] \
  && [ "$drm_card_access_ready" = true ] \
  && [ "$input_broker_ready" = true ]; then
  drm_expected_ready=true
fi

drm_session_smoke_ready=false
drm_session_smoke_blocked_expected=false
drm_session_clean_exit=false
session_drm_first_present_probe=false
session_first_present_commit_succeeded=false
session_first_present_vblank_event_received=false
session_first_present_blocked_by_drm_master=false

if [ "$drm_expected_ready" = true ]; then
  cargo build -p backlit-session --features smithay-backend
  cargo build -p backlit-compositor --features smithay-backend
  cargo build \
    -p backlit-demo-client \
    -p backlit-shell \
    -p backlit-notification-daemon \
    -p backlit-settings-daemon
  target/debug/backlit-session \
    --backend=drm \
    --socket=backlit-drm-smoke \
    --screenshot="$session_screenshot" \
    --verify \
    --verify-launch-spawn \
    --launch-spawn-program=true \
    --verify-desktop-launch \
    --verify-drm-first-present \
    --desktop-dir=crates/launcher/fixtures \
    --desktop-entry=org.backlit.SpawnProbe.desktop \
    --wayland-display=backlit-drm-smoke \
    --verify-services \
    --verify-clean-exit \
    --service-log-dir="$service_log_dir" > "$session_log" 2> "$session_err"

  grep '"event":"session.launch"' "$session_log" >/dev/null
  grep '"verify_drm_first_present":true' "$session_log" >/dev/null
  grep '"event":"session.backend_preflight"' "$session_log" >/dev/null
  grep '"backend":"drm"' "$session_log" >/dev/null
  grep '"ready":true' "$session_log" >/dev/null
  grep '"event":"session.backend_launch_plan"' "$session_log" >/dev/null
  grep '"implementation":"pre-smithay-policy-harness"' "$session_log" >/dev/null
  grep '"display_driver":"drm-kms"' "$session_log" >/dev/null
  grep '"uses_drm":true' "$session_log" >/dev/null
  grep '"uses_libinput":true' "$session_log" >/dev/null
  grep '"drm_card_selected":true' "$session_log" >/dev/null
  grep '"input_event_selected":true' "$session_log" >/dev/null
  grep '"xdg_runtime_dir_owned_by_user":true' "$session_log" >/dev/null
  grep '"logind_session_verified":true' "$session_log" >/dev/null
  grep '"session_active":true' "$session_log" >/dev/null
  grep '"session_remote":false' "$session_log" >/dev/null
  grep '"drm_card_access_ready":true' "$session_log" >/dev/null
  grep '"input_broker_ready":true' "$session_log" >/dev/null
  grep '"input_broker_mode":"' "$session_log" >/dev/null
  grep '"event":"session.drm_first_present_probe"' "$session_log" >/dev/null
  grep '"runtime_backend":"smithay-drm-probe"' "$session_log" >/dev/null
  grep '"feature_enabled":true' "$session_log" >/dev/null
  grep '"compiled":true' "$session_log" >/dev/null
  grep '"launch_ready":true' "$session_log" >/dev/null
  grep '"drm_card_selected":true' "$session_log" >/dev/null
  grep '"drm_node_resolved":true' "$session_log" >/dev/null
  grep '"kms_scanout_plan_ready":true' "$session_log" >/dev/null
  grep '"kms_surface_created":true' "$session_log" >/dev/null
  grep '"kms_framebuffer_created":true' "$session_log" >/dev/null
  grep '"kms_framebuffer_added":true' "$session_log" >/dev/null
  grep '"kms_first_present_framebuffer_filled":true' "$session_log" >/dev/null
  grep '"kms_first_present_plane_state_ready":true' "$session_log" >/dev/null
  require_matches "$session_log" '"kms_framebuffer_test_state_(succeeded|permission_denied)":true'
  require_matches "$session_log" '"kms_first_present_(commit_succeeded|blocked_by_drm_master)":true'
  if grep -F '"kms_first_present_commit_succeeded":true' "$session_log" >/dev/null; then
    grep '"kms_first_present_vblank_event_received":true' "$session_log" >/dev/null
    session_first_present_commit_succeeded=true
    session_first_present_vblank_event_received=true
  else
    grep '"kms_first_present_blocked_by_drm_master":true' "$session_log" >/dev/null
    grep '"kms_framebuffer_test_state_permission_denied":true' "$session_log" >/dev/null
    session_first_present_blocked_by_drm_master=true
  fi
  grep '"kms_first_present_failure":""' "$session_log" >/dev/null
  grep '"event":"session.gui_ready"' "$session_log" >/dev/null
  grep '"event":"session.verified"' "$session_log" >/dev/null
  grep '"event":"session.launch_spawn"' "$session_log" >/dev/null
  grep '"event":"session.desktop_launch"' "$session_log" >/dev/null
  grep '"event":"session.services_verified"' "$session_log" >/dev/null
  grep '"event":"session.clean_exit"' "$session_log" >/dev/null
  grep '"passed":true' "$session_log" >/dev/null
  grep '"golden_ok":true' "$session_log" >/dev/null
  grep '"policy_windows":3' "$session_log" >/dev/null
  grep '"visible_windows":3' "$session_log" >/dev/null
  grep '"focused_window_visible":true' "$session_log" >/dev/null
  grep '"focused_title_bar_ok":true' "$session_log" >/dev/null
  grep '"workspace_indicator_ok":true' "$session_log" >/dev/null
  grep '"spawned":true' "$session_log" >/dev/null
  grep '"exit_success":true' "$session_log" >/dev/null
  grep '"wayland_display_set":true' "$session_log" >/dev/null
  grep '"entry_selector":"org.backlit.SpawnProbe.desktop"' "$session_log" >/dev/null
  grep '"entry_resolved":true' "$session_log" >/dev/null
  grep '"entry_program":"sh"' "$session_log" >/dev/null
  grep '"entry_arg_count":2' "$session_log" >/dev/null
  grep '"program_resolved":true' "$session_log" >/dev/null
  grep '"managed_window_mapped":true' "$session_log" >/dev/null
  grep '"managed_window_app_id":"org.backlit.SpawnProbe.desktop"' "$session_log" >/dev/null
  grep '"managed_windows_after_launch":4' "$session_log" >/dev/null
  grep '"focused_launched_window":true' "$session_log" >/dev/null
  grep '"compositor_ready":true' "$session_log" >/dev/null
  grep '"compositor_runtime":"smithay"' "$session_log" >/dev/null
  grep '"compositor_runtime_backend":"smithay-compositor-runtime"' "$session_log" >/dev/null
  grep '"compositor_runtime_backend_ok":true' "$session_log" >/dev/null
  grep '"compositor_smithay_runtime":true' "$session_log" >/dev/null
  grep '"compositor_smithay_protocol_globals":true' "$session_log" >/dev/null
  grep '"compositor_smithay_input_sources":true' "$session_log" >/dev/null
  grep '"compositor_smithay_input_event_loop":true' "$session_log" >/dev/null
  grep '"compositor_smithay_input_seat_handles":true' "$session_log" >/dev/null
  grep '"compositor_smithay_input_seat_dispatch":true' "$session_log" >/dev/null
  grep '"compositor_service_socket_bound":true' "$session_log" >/dev/null
  grep '"compositor_demo_client_resolved":true' "$session_log" >/dev/null
  grep '"compositor_demo_client_exit_ok":true' "$session_log" >/dev/null
  grep '"compositor_demo_client_connected":true' "$session_log" >/dev/null
  grep '"compositor_demo_surface_mapped":true' "$session_log" >/dev/null
  grep '"compositor_demo_app_id_preserved":true' "$session_log" >/dev/null
  grep '"compositor_service_socket_cleanup":true' "$session_log" >/dev/null
  grep '"shell_ready":true' "$session_log" >/dev/null
  grep '"notification_ready":true' "$session_log" >/dev/null
  grep '"settings_ready":true' "$session_log" >/dev/null
  grep '"children_exited_cleanly":true' "$session_log" >/dev/null
  grep '"workspace_switch_ok":true' "$session_log" >/dev/null
  grep '"snap_left_ok":true' "$session_log" >/dev/null
  grep '"snap_right_ok":true' "$session_log" >/dev/null
  grep '"windows_before_shutdown":3' "$session_log" >/dev/null
  grep '"windows_closed":3' "$session_log" >/dev/null
  grep '"windows_after_shutdown":0' "$session_log" >/dev/null
  grep '"focus_cleared":true' "$session_log" >/dev/null
  grep "\"checksum\":$expected_checksum" "$session_log" >/dev/null
  test -s "$session_screenshot"

  session_ppm_bytes="$(wc -c < "$session_screenshot" | tr -d ' ')"
  test "$session_ppm_bytes" = "$expected_ppm_bytes"
  drm_session_smoke_ready=true
  drm_session_clean_exit=true
  session_drm_first_present_probe=true
else
  set +e
  cargo run -p backlit-session -- \
    --backend=drm \
    --socket=backlit-drm-smoke \
    --preflight-only > "$session_log" 2> "$session_err"
  session_status="$?"
  set -e

  test "$session_status" -ne 0 || fail "DRM session preflight unexpectedly passed"
  grep '"event":"session.backend_preflight"' "$session_log" >/dev/null
  grep '"backend":"drm"' "$session_log" >/dev/null
  grep '"ready":false' "$session_log" >/dev/null
  grep '"event":"session.backend_launch_plan"' "$session_log" >/dev/null
  grep '"implementation":"pre-smithay-policy-harness"' "$session_log" >/dev/null
  grep '"event":"session.launch_ready"' "$session_log" >/dev/null
  grep '"passed":false' "$session_log" >/dev/null
  drm_session_smoke_blocked_expected=true
  session_ppm_bytes=0
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-drm-session-smoke",
  "passed": true,
  "target_os": "$(uname -s)",
  "artifacts": {
    "session_log": "$session_log",
    "session_stderr": "$session_err",
    "session_screenshot": "$session_screenshot",
    "session_services_dir": "$service_log_dir"
  },
  "checks": {
    "drm_expected_ready": $drm_expected_ready,
    "drm_session_smoke_ready": $drm_session_smoke_ready,
    "drm_backend_launch_plan": true,
    "drm_device_selected": $drm_session_smoke_ready,
    "drm_input_selected": $drm_session_smoke_ready,
    "drm_session_smoke_blocked_expected": $drm_session_smoke_blocked_expected,
    "drm_session_clean_exit": $drm_session_clean_exit,
    "session_drm_first_present_probe": $session_drm_first_present_probe,
    "session_first_present_commit_succeeded": $session_first_present_commit_succeeded,
    "session_first_present_vblank_event_received": $session_first_present_vblank_event_received,
    "session_first_present_blocked_by_drm_master": $session_first_present_blocked_by_drm_master,
    "settings_service": $drm_session_smoke_ready,
    "session_compositor_smithay_runtime": $drm_session_smoke_ready,
    "session_compositor_smithay_protocol_globals": $drm_session_smoke_ready,
    "session_compositor_smithay_input_sources": $drm_session_smoke_ready,
    "session_compositor_smithay_input_event_loop": $drm_session_smoke_ready,
    "session_compositor_smithay_input_seat_handles": $drm_session_smoke_ready,
    "session_compositor_smithay_input_seat_dispatch": $drm_session_smoke_ready,
    "session_compositor_demo_client": $drm_session_smoke_ready,
    "session_compositor_demo_app_id_preserved": $drm_session_smoke_ready,
    "session_desktop_launch": $drm_session_smoke_ready,
    "session_desktop_managed_window": $drm_session_smoke_ready,
    "notification_service": $drm_session_smoke_ready,
    "workspace_switch": $drm_session_smoke_ready,
    "snap": $drm_session_smoke_ready,
    "xdg_runtime_dir_present": $runtime_present,
    "xdg_runtime_dir_owned_by_user": $runtime_owned_by_user,
    "session_present": $session_present,
    "session_active": $session_active,
    "session_remote": $session_remote,
    "session_local": $session_local,
    "session_state": "$session_state",
    "seat": "$session_seat",
    "session_type": "$session_type",
    "logind_available": $logind_available,
    "libseat_available": $libseat_available,
    "libinput_available": $libinput_available,
    "drm_card_nodes": $drm_card_nodes,
    "drm_render_nodes": $drm_render_nodes,
    "input_event_nodes": $input_event_nodes,
    "drm_card_readable": $drm_card_readable,
    "drm_card_writable": $drm_card_writable,
    "drm_render_readable": $drm_render_readable,
    "drm_render_writable": $drm_render_writable,
    "input_event_readable": $input_event_readable,
    "drm_card_access_ready": $drm_card_access_ready,
    "input_requires_logind_broker": $input_requires_logind_broker,
    "input_broker_ready": $input_broker_ready,
    "input_broker_mode": "$input_broker_mode",
    "session_ppm_bytes": $session_ppm_bytes
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit DRM session smoke verification passed. Artifacts: %s\n' "$out_dir"
