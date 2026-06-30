#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/linux-e2e}"
smoke_dir="$out_dir/gui-smoke"
mkdir -p "$out_dir"

commit="$(git rev-parse --short HEAD 2>/dev/null || printf unknown)"
branch="$(git status --short --branch 2>/dev/null | sed -n '1p' || printf unknown)"
rustc_version="$(rustc -V)"
cargo_version="$(cargo -V)"

cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
./scripts/verify-gui-smoke.sh "$smoke_dir"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-linux-e2e",
  "passed": true,
  "commit": "$commit",
  "branch": "$branch",
  "rustc": "$rustc_version",
  "cargo": "$cargo_version",
  "artifacts": {
    "gui_smoke_manifest": "$smoke_dir/manifest.json"
  },
  "checks": {
    "fmt": true,
    "tests": true,
    "clippy": true,
    "gui_smoke": true
  }
}
EOF

printf 'Backlit Linux E2E verification passed. Artifacts: %s\n' "$out_dir"
