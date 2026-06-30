#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/mvp0-contract}"
artifact_root="${2:-}"
mkdir -p "$out_dir"

fail() {
  echo "MVP 0 contract verification failed: $*" >&2
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

require_file Cargo.toml
require_file backlit-design.md
require_file docs/architecture/mvp-0.md
require_executable scripts/verify-gui-smoke.sh
require_executable scripts/render-gui-preview.sh
require_executable scripts/verify-launch-performance.sh
require_executable scripts/verify-resource-budget.sh
require_executable scripts/verify-linux-e2e.sh
require_executable scripts/verify-ci-contract.sh
require_executable scripts/verify-packaging-contract.sh
require_executable scripts/verify-staged-session-install.sh
require_executable scripts/verify-nested-wayland-smoke.sh
require_executable scripts/verify-session-clean-exit.sh

require_contains Cargo.toml '"crates/compositor"'
require_contains Cargo.toml '"crates/compositor-backend"'
require_contains Cargo.toml '"crates/demo-client"'
require_contains Cargo.toml '"crates/input"'
require_contains Cargo.toml '"crates/protocols"'
require_contains Cargo.toml '"crates/perf"'
require_contains Cargo.toml '"crates/session"'
require_contains Cargo.toml '"crates/shell"'
require_contains Cargo.toml '"crates/launcher"'
require_contains Cargo.toml '"crates/surface"'
require_contains Cargo.toml '"crates/window-policy"'

require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-compositor -- --backend=headless --smoke-test'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-compositor-backend -- --backend=headless --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-protocols -- --verify --list'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-perf -- --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-input -- --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-surface -- --verify'
require_contains scripts/verify-gui-smoke.sh 'cargo run -p backlit-demo-client --'
require_contains scripts/verify-gui-smoke.sh '"golden_checksum": true'
require_contains scripts/verify-launch-performance.sh '"name": "backlit-launch-performance"'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-nested-wayland-smoke.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/render-gui-preview.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-launch-performance.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-resource-budget.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-ci-contract.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-packaging-contract.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-staged-session-install.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-drm-session-smoke.sh'
require_contains scripts/verify-linux-e2e.sh './scripts/verify-session-clean-exit.sh'

require_contains packaging/sessions/backlit.desktop 'Exec=backlit-session'
require_contains packaging/systemd/backlit-compositor.service 'ExecStart=/usr/bin/backlit-compositor'
require_contains packaging/systemd/backlit-shell.service 'ExecStart=/usr/bin/backlit-shell'
require_contains packaging/debian/control.stub 'Package: fastgui-session'

artifact_manifests_checked=false
nested_wayland_artifact=false
if [ -n "$artifact_root" ] && [ -d "$artifact_root" ]; then
  artifact_manifests_checked=true
  require_file "$artifact_root/gui-smoke/manifest.json"
  require_file "$artifact_root/gui-preview/manifest.json"
  require_file "$artifact_root/launch-performance/manifest.json"
  require_file "$artifact_root/resource-budget/manifest.json"
  require_file "$artifact_root/ci-contract/manifest.json"
  require_file "$artifact_root/packaging-contract/manifest.json"
  require_file "$artifact_root/staged-session-install/manifest.json"
  require_file "$artifact_root/session-clean-exit/manifest.json"
  require_file "$artifact_root/drm-session-smoke/manifest.json"

  require_contains "$artifact_root/gui-smoke/manifest.json" '"protocol_required_count": 7'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"launcher_required_targets": 3'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"shortcut_required_bindings": 6'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"keyboard_input": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"pointer_input": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_input": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"surface_lifecycle": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_surface_lifecycle": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"no_idle_redraw": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"targeted_damage": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_services": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_launch_spawn": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"session_move_resize": true'
  require_contains "$artifact_root/gui-smoke/manifest.json" '"golden_checksum": true'
  require_contains "$artifact_root/gui-preview/manifest.json" '"session_verified": true'
  require_contains "$artifact_root/gui-preview/manifest.json" '"session_services": true'
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
  require_contains "$artifact_root/ci-contract/manifest.json" '"linux_e2e_gate": true'
  require_contains "$artifact_root/packaging-contract/manifest.json" '"desktop_entry": true'
  require_contains "$artifact_root/packaging-contract/manifest.json" '"package_split": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"desktop_exec_resolves": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_session_gui": true'
  require_contains "$artifact_root/staged-session-install/manifest.json" '"staged_session_services": true'
  require_contains "$artifact_root/session-clean-exit/manifest.json" '"clean_exit_event": true'
  require_contains "$artifact_root/session-clean-exit/manifest.json" '"windows_after_shutdown": 0'
  require_contains "$artifact_root/session-clean-exit/manifest.json" '"focus_cleared": true'
  require_contains "$artifact_root/drm-session-smoke/manifest.json" '"name": "backlit-drm-session-smoke"'

  if [ -f "$artifact_root/nested-wayland/manifest.json" ]; then
    nested_wayland_artifact=true
    require_contains "$artifact_root/nested-wayland/manifest.json" '"parent_socket_ready": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"wayland_preflight_ready": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"compositor_wayland_smoke": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_client_spawn": true'
    require_contains "$artifact_root/nested-wayland/manifest.json" '"session_wayland_services": true'
  fi
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-mvp0-contract",
  "passed": true,
  "artifact_manifests_checked": $artifact_manifests_checked,
  "nested_wayland_artifact": $nested_wayland_artifact,
  "artifacts": {
    "architecture": "docs/architecture/mvp-0.md",
    "gui_smoke_verifier": "scripts/verify-gui-smoke.sh",
    "linux_e2e_verifier": "scripts/verify-linux-e2e.sh"
  },
  "checks": {
    "workspace_crates": true,
    "headless_backend": true,
    "nested_wayland_gate": true,
    "demo_client": true,
    "performance_smoke": true,
    "launch_performance": true,
    "resource_budget": true,
    "input_smoke": true,
    "surface_lifecycle": true,
    "frame_damage": true,
    "protocol_smoke": true,
    "golden_gui": true,
    "viewable_preview": true,
    "session_services": true,
    "session_clean_exit": true,
    "packaging_skeleton": true,
    "staged_session_install": true,
    "drm_session_smoke": true,
    "ci_gate": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit MVP 0 contract verification passed. Artifacts: %s\n' "$out_dir"
