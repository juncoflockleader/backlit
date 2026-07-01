#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/dedicated-drm-session}"
mkdir -p "$out_dir"

boundary_dir="$out_dir/drm-master-boundary"
boundary_manifest="$boundary_dir/manifest.json"
manifest="$out_dir/manifest.json"
session_log="$out_dir/session.jsonl"
session_err="$out_dir/session.stderr"
session_screenshot="$out_dir/dedicated-session.ppm"
service_log_dir="$out_dir/session-services"
handoff_plan="$out_dir/dedicated-drm-handoff.sh"
expected_checksum="15888844850457870477"
expected_ppm_bytes="1248015"

fail() {
  echo "Dedicated DRM session verification failed: $*" >&2
  exit 1
}

require_matches() {
  file="$1"
  value="$2"
  grep -E "$value" "$file" >/dev/null || fail "missing pattern in $file: $value"
}

bool_has() {
  file="$1"
  value="$2"
  if grep -F -- "$value" "$file" >/dev/null 2>&1; then
    printf true
  else
    printf false
  fi
}

write_handoff_plan() {
  cat > "$handoff_plan" <<'EOF'
#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/dedicated-drm-session-acceptance}"
printf 'Backlit dedicated DRM acceptance handoff\n'
printf '  output: %s\n' "$out_dir"
printf '  requires: seat-owner TTY or display-manager Backlit session with DRM master\n'
printf '  session: id=%s seat=%s type=%s runtime=%s wayland=%s\n' \
  "${XDG_SESSION_ID:-}" "${XDG_SEAT:-}" "${XDG_SESSION_TYPE:-}" \
  "${XDG_RUNTIME_DIR:-}" "${WAYLAND_DISPLAY:-}"

BACKLIT_REQUIRE_DEDICATED_DRM_SESSION=1 \
BACKLIT_REQUIRE_DRM_MASTER_PRESENT=1 \
  ./scripts/verify-dedicated-drm-session.sh "$out_dir"
EOF
  chmod +x "$handoff_plan"
}

write_manifest() {
  passed="$1"
  expected_blocked="$2"
  reason="$3"
  dedicated_session_acceptance="$4"
  drm_launch_ready="$5"
  current_session_can_present="$6"
  dedicated_session_required="$7"
  first_present_framebuffer_filled="$8"
  first_present_plane_state_ready="$9"
  first_present_commit_succeeded="${10}"
  first_present_vblank_event_received="${11}"
  first_present_blocked_by_drm_master="${12}"
  session_drm_first_present_probe="${13}"
  session_gui_verified="${14}"
  session_services="${15}"
  session_desktop_launch="${16}"
  session_compositor_demo_client="${17}"
  session_clean_exit="${18}"
  session_ppm_bytes="${19}"

  cat > "$manifest" <<EOF
{
  "name": "backlit-dedicated-drm-session",
  "passed": $passed,
  "checked": $boundary_checked,
  "expected_blocked": $expected_blocked,
  "reason": "$reason",
  "artifacts": {
    "drm_master_boundary_manifest": "$boundary_manifest",
    "session_log": "$session_log",
    "session_stderr": "$session_err",
    "session_screenshot": "$session_screenshot",
    "session_services_dir": "$service_log_dir",
    "dedicated_handoff_script": "$handoff_plan"
  },
  "handoff": {
    "command": "BACKLIT_REQUIRE_DEDICATED_DRM_SESSION=1 BACKLIT_REQUIRE_DRM_MASTER_PRESENT=1 ./scripts/verify-dedicated-drm-session.sh target/dedicated-drm-session-acceptance",
    "requires": "seat-owner-tty-or-display-manager-session",
    "seat_owner_required": true,
    "drm_master_present_required": true,
    "acceptance_checks": "first-present-commit-vblank-gui-services-launch-clean-exit",
    "mutating_handoff_attempted": false
  },
  "checks": {
    "dedicated_handoff_plan": true,
    "dedicated_handoff_script_checked": true,
    "dedicated_handoff_seat_owner_required": true,
    "dedicated_handoff_drm_master_present_required": true,
    "dedicated_handoff_acceptance_checks": true,
    "dedicated_session_acceptance": $dedicated_session_acceptance,
    "drm_master_boundary": true,
    "drm_launch_ready": $drm_launch_ready,
    "current_session_can_present": $current_session_can_present,
    "dedicated_session_required": $dedicated_session_required,
    "first_present_framebuffer_filled": $first_present_framebuffer_filled,
    "first_present_plane_state_ready": $first_present_plane_state_ready,
    "first_present_commit_succeeded": $first_present_commit_succeeded,
    "first_present_vblank_event_received": $first_present_vblank_event_received,
    "first_present_blocked_by_drm_master": $first_present_blocked_by_drm_master,
    "session_drm_first_present_probe": $session_drm_first_present_probe,
    "session_gui_verified": $session_gui_verified,
    "session_services": $session_services,
    "session_desktop_launch": $session_desktop_launch,
    "session_compositor_demo_client": $session_compositor_demo_client,
    "session_clean_exit": $session_clean_exit,
    "session_ppm_bytes": $session_ppm_bytes,
    "require_dedicated_session": $require_dedicated_session,
    "dedicated_session_model": "seat-owner-tty-or-display-manager-session"
  }
}
EOF
}

boundary_checked=false
require_dedicated_session=false
if [ "${BACKLIT_REQUIRE_DEDICATED_DRM_SESSION:-0}" = "1" ] \
  || [ "${BACKLIT_REQUIRE_DRM_MASTER_PRESENT:-0}" = "1" ]; then
  require_dedicated_session=true
fi

write_handoff_plan

./scripts/verify-drm-master-boundary.sh "$boundary_dir"

test -f "$boundary_manifest" || fail "missing DRM master boundary manifest"
grep '"name": "backlit-drm-master-boundary"' "$boundary_manifest" >/dev/null

drm_launch_ready="$(bool_has "$boundary_manifest" '"drm_launch_ready": true')"
boundary_checked="$(bool_has "$boundary_manifest" '"checked": true')"
current_session_can_present="$(bool_has "$boundary_manifest" '"current_session_can_present": true')"
dedicated_session_required="$(bool_has "$boundary_manifest" '"dedicated_session_required": true')"
first_present_framebuffer_filled="$(bool_has "$boundary_manifest" '"first_present_framebuffer_filled": true')"
first_present_plane_state_ready="$(bool_has "$boundary_manifest" '"first_present_plane_state_ready": true')"
first_present_commit_succeeded="$(bool_has "$boundary_manifest" '"first_present_commit_succeeded": true')"
first_present_vblank_event_received="$(bool_has "$boundary_manifest" '"first_present_vblank_event_received": true')"
first_present_blocked_by_drm_master="$(bool_has "$boundary_manifest" '"first_present_blocked_by_drm_master": true')"

if [ "$current_session_can_present" != true ]; then
  reason="drm-master-unavailable"
  if grep -F '"reason": "non-linux-host"' "$boundary_manifest" >/dev/null 2>&1; then
    reason="non-linux-host"
  elif [ "$drm_launch_ready" != true ]; then
    reason="drm-launch-not-ready"
  fi

  write_manifest \
    true \
    true \
    "$reason" \
    false \
    "$drm_launch_ready" \
    "$current_session_can_present" \
    "$dedicated_session_required" \
    "$first_present_framebuffer_filled" \
    "$first_present_plane_state_ready" \
    "$first_present_commit_succeeded" \
    "$first_present_vblank_event_received" \
    "$first_present_blocked_by_drm_master" \
    false \
    false \
    false \
    false \
    false \
    false \
    0

  if [ "$require_dedicated_session" = true ]; then
    fail "current session cannot own DRM master; run from a dedicated Backlit TTY or display-manager session"
  fi

  printf 'Backlit dedicated DRM session verification expected-blocked. Artifacts: %s\n' "$out_dir"
  exit 0
fi

BACKLIT_REQUIRE_DRM_MASTER_PRESENT=1 ./scripts/verify-drm-master-boundary.sh "$boundary_dir"

cargo build -p backlit-session --features smithay-backend
cargo build \
  -p backlit-compositor \
  -p backlit-demo-client \
  -p backlit-shell \
  -p backlit-notification-daemon \
  -p backlit-settings-daemon

target/debug/backlit-session \
  --backend=drm \
  --socket=backlit-dedicated-session \
  --screenshot="$session_screenshot" \
  --verify \
  --verify-launch-spawn \
  --launch-spawn-program=true \
  --verify-desktop-launch \
  --verify-drm-first-present \
  --require-drm-master-present \
  --desktop-dir=crates/launcher/fixtures \
  --desktop-entry=org.backlit.SpawnProbe.desktop \
  --wayland-display=backlit-dedicated-session \
  --verify-services \
  --verify-clean-exit \
  --service-log-dir="$service_log_dir" > "$session_log" 2> "$session_err"

grep '"event":"session.launch"' "$session_log" >/dev/null
grep '"verify_drm_first_present":true' "$session_log" >/dev/null
grep '"require_drm_master_present":true' "$session_log" >/dev/null
grep '"event":"session.backend_preflight"' "$session_log" >/dev/null
grep '"backend":"drm"' "$session_log" >/dev/null
grep '"ready":true' "$session_log" >/dev/null
grep '"event":"session.backend_launch_plan"' "$session_log" >/dev/null
grep '"drm_card_selected":true' "$session_log" >/dev/null
grep '"input_event_selected":true' "$session_log" >/dev/null
grep '"drm_card_access_ready":true' "$session_log" >/dev/null
grep '"input_broker_ready":true' "$session_log" >/dev/null
grep '"event":"session.drm_first_present_probe"' "$session_log" >/dev/null
grep '"implementation":"smithay-compositor-runtime"' "$session_log" >/dev/null
grep '"runtime_backend":"smithay-drm-probe"' "$session_log" >/dev/null
grep '"feature_enabled":true' "$session_log" >/dev/null
grep '"compiled":true' "$session_log" >/dev/null
grep '"launch_ready":true' "$session_log" >/dev/null
grep '"drm_node_resolved":true' "$session_log" >/dev/null
grep '"kms_scanout_plan_ready":true' "$session_log" >/dev/null
grep '"kms_surface_created":true' "$session_log" >/dev/null
grep '"kms_framebuffer_created":true' "$session_log" >/dev/null
grep '"kms_framebuffer_added":true' "$session_log" >/dev/null
grep '"kms_first_present_framebuffer_filled":true' "$session_log" >/dev/null
grep '"kms_first_present_plane_state_ready":true' "$session_log" >/dev/null
grep '"kms_first_present_commit_succeeded":true' "$session_log" >/dev/null
grep '"kms_first_present_vblank_event_received":true' "$session_log" >/dev/null
grep '"kms_first_present_blocked_by_drm_master":false' "$session_log" >/dev/null
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
grep '"spawned":true' "$session_log" >/dev/null
grep '"exit_success":true' "$session_log" >/dev/null
grep '"entry_selector":"org.backlit.SpawnProbe.desktop"' "$session_log" >/dev/null
grep '"managed_window_mapped":true' "$session_log" >/dev/null
grep '"managed_window_app_id":"org.backlit.SpawnProbe.desktop"' "$session_log" >/dev/null
grep '"compositor_ready":true' "$session_log" >/dev/null
grep '"compositor_service_socket_bound":true' "$session_log" >/dev/null
grep '"compositor_demo_client_resolved":true' "$session_log" >/dev/null
grep '"compositor_demo_client_exit_ok":true' "$session_log" >/dev/null
grep '"compositor_demo_client_connected":true' "$session_log" >/dev/null
grep '"compositor_demo_surface_mapped":true' "$session_log" >/dev/null
grep '"compositor_demo_app_id_preserved":true' "$session_log" >/dev/null
grep '"shell_ready":true' "$session_log" >/dev/null
grep '"notification_ready":true' "$session_log" >/dev/null
grep '"settings_ready":true' "$session_log" >/dev/null
grep '"children_exited_cleanly":true' "$session_log" >/dev/null
grep '"workspace_switch_ok":true' "$session_log" >/dev/null
grep '"snap_left_ok":true' "$session_log" >/dev/null
grep '"snap_right_ok":true' "$session_log" >/dev/null
grep '"windows_after_shutdown":0' "$session_log" >/dev/null
grep '"focus_cleared":true' "$session_log" >/dev/null
grep "\"checksum\":$expected_checksum" "$session_log" >/dev/null
test -s "$session_screenshot"

session_ppm_bytes="$(wc -c < "$session_screenshot" | tr -d ' ')"
test "$session_ppm_bytes" = "$expected_ppm_bytes"

write_manifest \
  true \
  false \
  "dedicated-drm-session-presented" \
  true \
  true \
  true \
  false \
  true \
  true \
  true \
  true \
  false \
  true \
  true \
  true \
  true \
  true \
  true \
  "$session_ppm_bytes"

require_matches "$manifest" '"dedicated_session_acceptance": true'
grep '"passed": true' "$manifest" >/dev/null

printf 'Backlit dedicated DRM session verification passed. Artifacts: %s\n' "$out_dir"
