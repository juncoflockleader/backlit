#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/ci-contract}"
workflow=".github/workflows/linux-e2e.yml"
mkdir -p "$out_dir"

fail() {
  echo "CI contract verification failed: $*" >&2
  exit 1
}

require_file() {
  test -f "$1" || fail "missing file $1"
}

require_contains() {
  file="$1"
  value="$2"
  grep -F -- "$value" "$file" >/dev/null || fail "missing text in $file: $value"
}

require_file "$workflow"

require_contains "$workflow" "name: Linux E2E"
require_contains "$workflow" "pull_request:"
require_contains "$workflow" "branches:"
require_contains "$workflow" "- main"
require_contains "$workflow" "runs-on: ubuntu-24.04"
require_contains "$workflow" "timeout-minutes: 45"
require_contains "$workflow" "uses: actions/checkout@v4"
require_contains "$workflow" "sudo ./scripts/bootstrap-ubuntu.sh"
require_contains "$workflow" "rustup component add rustfmt clippy"
require_contains "$workflow" "./scripts/verify-linux-e2e.sh target/linux-e2e-ci"
require_contains "$workflow" "uses: actions/upload-artifact@v4"
require_contains "$workflow" "path: target/linux-e2e-ci"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-ci-contract",
  "passed": true,
  "artifacts": {
    "workflow": "$workflow"
  },
  "checks": {
    "push_main": true,
    "pull_request": true,
    "ubuntu_runner": true,
    "bootstrap_dependencies": true,
    "rustfmt_clippy": true,
    "linux_e2e_gate": true,
    "artifact_upload": true
  }
}
EOF

printf 'Backlit CI contract verification passed. Artifacts: %s\n' "$out_dir"
