#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/linux-e2e}"
smoke_dir="$out_dir/gui-smoke"
preview_dir="$out_dir/gui-preview"
ci_contract_dir="$out_dir/ci-contract"
packaging_dir="$out_dir/packaging-contract"
staged_install_dir="$out_dir/staged-session-install"
launch_readiness_dir="$out_dir/launch-readiness"
session_launch_dir="$out_dir/session-launch"
drm_session_smoke_dir="$out_dir/drm-session-smoke"
mvp0_contract_dir="$out_dir/mvp0-contract"
mkdir -p "$out_dir"

commit="$(git rev-parse --short HEAD 2>/dev/null || printf unknown)"
branch="$(git status --short --branch 2>/dev/null | sed -n '1p' || printf unknown)"
rustc_version="$(rustc -V)"
cargo_version="$(cargo -V)"

cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
./scripts/verify-gui-smoke.sh "$smoke_dir"
./scripts/render-gui-preview.sh "$preview_dir"
./scripts/verify-ci-contract.sh "$ci_contract_dir"
./scripts/verify-packaging-contract.sh "$packaging_dir"
./scripts/verify-staged-session-install.sh "$staged_install_dir"
./scripts/verify-launch-readiness.sh "$launch_readiness_dir"
./scripts/verify-session-launch.sh "$session_launch_dir"
./scripts/verify-drm-session-smoke.sh "$drm_session_smoke_dir"

nested_wayland=false
nested_wayland_manifest=""
if [ "$(uname -s)" = "Linux" ]; then
  ./scripts/verify-nested-wayland-smoke.sh "$out_dir/nested-wayland"
  nested_wayland=true
  nested_wayland_manifest="$out_dir/nested-wayland/manifest.json"
fi

./scripts/verify-mvp0-contract.sh "$mvp0_contract_dir" "$out_dir"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-linux-e2e",
  "passed": true,
  "commit": "$commit",
  "branch": "$branch",
  "rustc": "$rustc_version",
  "cargo": "$cargo_version",
  "artifacts": {
    "gui_smoke_manifest": "$smoke_dir/manifest.json",
    "gui_preview_manifest": "$preview_dir/manifest.json",
    "ci_contract_manifest": "$ci_contract_dir/manifest.json",
    "packaging_contract_manifest": "$packaging_dir/manifest.json",
    "staged_session_install_manifest": "$staged_install_dir/manifest.json",
    "launch_readiness_manifest": "$launch_readiness_dir/manifest.json",
    "session_launch_manifest": "$session_launch_dir/manifest.json",
    "drm_session_smoke_manifest": "$drm_session_smoke_dir/manifest.json",
    "mvp0_contract_manifest": "$mvp0_contract_dir/manifest.json",
    "nested_wayland_manifest": "$nested_wayland_manifest"
  },
  "checks": {
    "fmt": true,
    "tests": true,
    "clippy": true,
    "gui_smoke": true,
    "gui_preview": true,
    "ci_contract": true,
    "packaging_contract": true,
    "staged_session_install": true,
    "launch_readiness": true,
    "session_launch": true,
    "drm_session_smoke": true,
    "mvp0_contract": true,
    "nested_wayland": $nested_wayland
  }
}
EOF

printf 'Backlit Linux E2E verification passed. Artifacts: %s\n' "$out_dir"
