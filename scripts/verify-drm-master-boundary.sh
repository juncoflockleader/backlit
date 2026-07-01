#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/drm-master-boundary}"
mkdir -p "$out_dir"

manifest="$out_dir/manifest.json"
probe_dir="$out_dir/smithay-runtime-probe"
probe_manifest="$probe_dir/manifest.json"
session_desktop="packaging/sessions/backlit.desktop"
compositor_service="packaging/systemd/backlit-compositor.service"

fail() {
  echo "DRM master boundary verification failed: $*" >&2
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

session_exec="$(sed -n 's/^Exec=//p' "$session_desktop")"
compositor_exec="$(sed -n 's/^ExecStart=//p' "$compositor_service")"

test "$session_exec" = "backlit-session --backend=drm --activate-systemd" \
  || fail "unexpected session Exec=$session_exec"
test "$compositor_exec" = "/usr/bin/backlit-compositor --backend=drm --socket=backlit-0 --serve" \
  || fail "unexpected compositor ExecStart=$compositor_exec"

session_id="${XDG_SESSION_ID:-}"
session_type="${XDG_SESSION_TYPE:-}"
session_seat="${XDG_SEAT:-}"
if [ -n "$session_id" ] && command -v loginctl >/dev/null 2>&1; then
  logind_type="$(loginctl show-session "$session_id" -p Type --value 2>/dev/null || true)"
  logind_seat="$(loginctl show-session "$session_id" -p Seat --value 2>/dev/null || true)"
  if [ -n "$logind_type" ]; then
    session_type="$logind_type"
  fi
  if [ -n "$logind_seat" ]; then
    session_seat="$logind_seat"
  fi
fi

if [ "$(uname -s)" != "Linux" ]; then
  cat > "$manifest" <<EOF
{
  "name": "backlit-drm-master-boundary",
  "passed": true,
  "checked": false,
  "expected_blocked": true,
  "reason": "non-linux-host",
  "artifacts": {
    "session_desktop": "$session_desktop",
    "compositor_service": "$compositor_service"
  },
  "checks": {
    "session_entry_drm": true,
    "compositor_service_drm": true,
    "drm_master_boundary_checked": false,
    "drm_launch_ready": false,
    "first_present_framebuffer_filled": false,
    "first_present_plane_state_ready": false,
    "first_present_commit_succeeded": false,
    "first_present_vblank_event_received": false,
    "first_present_blocked_by_drm_master": false,
    "drm_master_boundary_observed": false,
    "dedicated_session_required": false,
    "current_session_can_present": false,
    "mutating_handoff_attempted": false,
    "require_drm_master_present": false,
    "session_id": "$session_id",
    "seat": "$session_seat",
    "session_type": "$session_type",
    "dedicated_session_model": "seat-owner-tty-or-display-manager-session"
  }
}
EOF
  printf 'Backlit DRM master boundary skipped as expected: non-linux-host. Artifacts: %s\n' "$out_dir"
  exit 0
fi

./scripts/verify-smithay-runtime-probe.sh "$probe_dir"

require_contains "$probe_manifest" '"name": "backlit-smithay-runtime-probe"'

checked=false
expected_blocked=true
drm_launch_ready=false
first_present_framebuffer_filled=false
first_present_plane_state_ready=false
first_present_commit_succeeded=false
first_present_vblank_event_received=false
first_present_blocked_by_drm_master=false
drm_master_boundary_observed=false
dedicated_session_required=false
current_session_can_present=false
require_drm_master_present=false

if [ "${BACKLIT_REQUIRE_DRM_MASTER_PRESENT:-0}" = "1" ]; then
  require_drm_master_present=true
fi

if grep -F '"checked": true' "$probe_manifest" >/dev/null; then
  checked=true
fi

if grep -F '"drm_launch_ready": true' "$probe_manifest" >/dev/null; then
  drm_launch_ready=true
  expected_blocked=false
  require_contains "$probe_manifest" '"smithay_runtime_probe": true'
  require_contains "$probe_manifest" '"smithay_kms_first_present_framebuffer_filled": true'
  require_contains "$probe_manifest" '"smithay_kms_first_present_plane_state_ready": true'
  require_matches "$probe_manifest" '"smithay_kms_first_present_commit_succeeded": (true|false)'
  require_matches "$probe_manifest" '"smithay_kms_first_present_vblank_event_received": (true|false)'
  require_matches "$probe_manifest" '"smithay_kms_first_present_blocked_by_drm_master": (true|false)'
  require_matches "$probe_manifest" '"smithay_kms_first_present_(commit_succeeded|blocked_by_drm_master)": true'

  first_present_framebuffer_filled=true
  first_present_plane_state_ready=true

  if grep -F '"smithay_kms_first_present_commit_succeeded": true' "$probe_manifest" >/dev/null; then
    require_contains "$probe_manifest" '"smithay_kms_first_present_vblank_event_received": true'
    first_present_commit_succeeded=true
    first_present_vblank_event_received=true
    current_session_can_present=true
  elif grep -F '"smithay_kms_first_present_blocked_by_drm_master": true' "$probe_manifest" >/dev/null; then
    require_contains "$probe_manifest" '"smithay_kms_framebuffer_test_state_permission_denied": true'
    first_present_blocked_by_drm_master=true
    drm_master_boundary_observed=true
    dedicated_session_required=true
  else
    fail "Smithay first-present probe neither committed nor recorded DRM-master denial"
  fi
fi

if [ "$require_drm_master_present" = true ] \
  && [ "$current_session_can_present" != true ]; then
  fail "first present did not commit; run Backlit from a dedicated DRM-master session"
fi

cat > "$manifest" <<EOF
{
  "name": "backlit-drm-master-boundary",
  "passed": true,
  "checked": $checked,
  "expected_blocked": $expected_blocked,
  "artifacts": {
    "session_desktop": "$session_desktop",
    "compositor_service": "$compositor_service",
    "smithay_runtime_probe_manifest": "$probe_manifest"
  },
  "checks": {
    "session_entry_drm": true,
    "compositor_service_drm": true,
    "drm_master_boundary_checked": $checked,
    "drm_launch_ready": $drm_launch_ready,
    "first_present_framebuffer_filled": $first_present_framebuffer_filled,
    "first_present_plane_state_ready": $first_present_plane_state_ready,
    "first_present_commit_succeeded": $first_present_commit_succeeded,
    "first_present_vblank_event_received": $first_present_vblank_event_received,
    "first_present_blocked_by_drm_master": $first_present_blocked_by_drm_master,
    "drm_master_boundary_observed": $drm_master_boundary_observed,
    "dedicated_session_required": $dedicated_session_required,
    "current_session_can_present": $current_session_can_present,
    "mutating_handoff_attempted": false,
    "require_drm_master_present": $require_drm_master_present,
    "session_id": "$session_id",
    "seat": "$session_seat",
    "session_type": "$session_type",
    "dedicated_session_model": "seat-owner-tty-or-display-manager-session"
  }
}
EOF

grep '"passed": true' "$manifest" >/dev/null

printf 'Backlit DRM master boundary verification passed. Artifacts: %s\n' "$out_dir"
