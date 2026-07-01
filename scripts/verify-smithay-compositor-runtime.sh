#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/smithay-compositor-runtime}"
mkdir -p "$out_dir"

log="$out_dir/smithay-compositor-runtime.jsonl"
err="$out_dir/smithay-compositor-runtime.stderr"

fail() {
  echo "Smithay compositor runtime verification failed: $*" >&2
  exit 1
}

require_contains() {
  file="$1"
  value="$2"
  grep -F -- "$value" "$file" >/dev/null || fail "missing text in $file: $value"
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
    "compositor_stderr": "$err"
  },
  "checks": {
    "smithay_compositor_runtime": false,
    "smithay_runtime_trait": false,
    "smithay_scripted_client": false,
    "smithay_service_ready": false,
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

set +e
target/debug/backlit-compositor \
  --backend=drm \
  --runtime=smithay \
  --scripted-client > "$log" 2> "$err"
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
require_contains "$log" '"event":"compositor.scripted_client"'
require_contains "$log" '"passed":true'
require_contains "$log" '"runtime_backend":"smithay-compositor-runtime"'
require_contains "$log" '"runtime_trait":true'
require_contains "$log" '"client_connected":true'
require_contains "$log" '"surfaces_after_map":2'
require_contains "$log" '"targeted_damage_ok":true'
require_contains "$log" '"clean_disconnect":true'
require_contains "$log" '"event":"compositor.ready"'
require_contains "$log" '"ready":true'
require_contains "$log" '"bootstrap_client_connected":true'
require_contains "$log" '"bootstrap_surface_presented":true'

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-smithay-compositor-runtime",
  "passed": true,
  "checked": true,
  "expected_blocked": false,
  "artifacts": {
    "compositor_log": "$log",
    "compositor_stderr": "$err"
  },
  "checks": {
    "smithay_compositor_runtime": true,
    "smithay_runtime_trait": true,
    "smithay_scripted_client": true,
    "smithay_service_ready": true,
    "drm_launch_ready": $drm_launch_ready
  }
}
EOF

printf 'Backlit Smithay compositor runtime verification passed. Artifacts: %s\n' "$out_dir"
