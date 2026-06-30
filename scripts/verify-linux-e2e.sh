#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/linux-e2e}"
smoke_dir="$out_dir/gui-smoke"
packaging_dir="$out_dir/packaging-contract"
mkdir -p "$out_dir"

commit="$(git rev-parse --short HEAD 2>/dev/null || printf unknown)"
branch="$(git status --short --branch 2>/dev/null | sed -n '1p' || printf unknown)"
rustc_version="$(rustc -V)"
cargo_version="$(cargo -V)"

cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
./scripts/verify-gui-smoke.sh "$smoke_dir"
./scripts/verify-packaging-contract.sh "$packaging_dir"

nested_wayland=false
nested_wayland_manifest=""
if [ "$(uname -s)" = "Linux" ]; then
  ./scripts/verify-nested-wayland-smoke.sh "$out_dir/nested-wayland"
  nested_wayland=true
  nested_wayland_manifest="$out_dir/nested-wayland/manifest.json"
fi

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
    "packaging_contract_manifest": "$packaging_dir/manifest.json",
    "nested_wayland_manifest": "$nested_wayland_manifest"
  },
  "checks": {
    "fmt": true,
    "tests": true,
    "clippy": true,
    "gui_smoke": true,
    "packaging_contract": true,
    "nested_wayland": $nested_wayland
  }
}
EOF

printf 'Backlit Linux E2E verification passed. Artifacts: %s\n' "$out_dir"
