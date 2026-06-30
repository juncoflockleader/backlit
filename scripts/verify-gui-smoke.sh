#!/usr/bin/env sh
set -eu

out_dir="${1:-target/gui-smoke}"
mkdir -p "$out_dir"

cargo run -p backlit-compositor -- --backend=headless --smoke-test > "$out_dir/compositor.jsonl"
cargo run -p backlit-shell -- --component=panel --socket=backlit-0 > "$out_dir/shell.jsonl"
cargo run -p backlit-session -- \
  --backend=headless \
  --socket=backlit-0 \
  --screenshot="$out_dir/backlit-session.ppm" \
  --verify > "$out_dir/session.jsonl"
cargo run -p backlit-demo-client -- \
  --output="$out_dir/demo-client.ppm" \
  --verify > "$out_dir/demo-client.jsonl"

grep '"event":"session.verified"' "$out_dir/session.jsonl" >/dev/null
grep '"passed":true' "$out_dir/session.jsonl" >/dev/null
grep '"event":"demo_client.verified"' "$out_dir/demo-client.jsonl" >/dev/null
grep '"passed":true' "$out_dir/demo-client.jsonl" >/dev/null
test -s "$out_dir/backlit-session.ppm"
test -s "$out_dir/demo-client.ppm"

printf 'Backlit GUI smoke verification passed. Artifacts: %s\n' "$out_dir"

