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
    "smithay_runtime_probe": false,
    "drm_launch_ready": false
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
require_contains "$log" '"feature_enabled":true'
require_contains "$log" '"compiled":true'
require_contains "$log" '"runtime_backend":"smithay-drm-probe"'
require_contains "$log" '"display_driver":"smithay-drm-kms"'
require_contains "$log" '"input_driver":"smithay-libinput"'
require_contains "$log" '"session_driver":"smithay-libseat-logind"'
require_contains "$log" '"event_loop":"calloop"'
require_contains "$log" '"component_count":5'

launch_ready=false
expected_blocked=true
smithay_runtime_probe=false
if grep -F '"event":"backend.preflight","backend":"drm","ready":true' "$log" >/dev/null; then
  launch_ready=true
  expected_blocked=false
  require_contains "$log" '"launch_ready":true'
  require_contains "$log" '"passed":true'
  require_contains "$log" '"drm_card_selected":true'
  require_contains "$log" '"input_event_selected":true'
  require_contains "$log" '"uses_logind":true'
  require_contains "$log" '"uses_libseat":true'
  require_contains "$log" '"uses_libinput":true'
  smithay_runtime_probe=true
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
    "smithay_libinput_component": true,
    "smithay_libseat_session_component": true,
    "smithay_calloop_component": true,
    "drm_launch_ready": $launch_ready,
    "smithay_runtime_probe": $smithay_runtime_probe
  }
}
EOF

printf 'Backlit Smithay runtime probe verification passed. Artifacts: %s\n' "$out_dir"
