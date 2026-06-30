#!/usr/bin/env sh
set -eu

out_dir="${1:-target/gui-smoke}"
mkdir -p "$out_dir"

expected_checksum="5635038614353063225"
expected_width="800"
expected_height="520"
expected_ppm_bytes="1248015"

cargo run -p backlit-compositor -- --backend=headless --smoke-test > "$out_dir/compositor.jsonl"
cargo run -p backlit-compositor-backend -- --backend=headless --verify > "$out_dir/backend-preflight.jsonl"
cargo run -p backlit-protocols -- --verify --list > "$out_dir/protocols.jsonl"
cargo run -p backlit-perf -- --verify > "$out_dir/perf.jsonl"
cargo run -p backlit-shell -- --component=all --socket=backlit-0 --verify > "$out_dir/shell.jsonl"
cargo run -p backlit-launcher -- --verify --list --target=terminal > "$out_dir/launcher.jsonl"
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
grep '"golden_ok":true' "$out_dir/session.jsonl" >/dev/null
grep "\"checksum\":$expected_checksum" "$out_dir/session.jsonl" >/dev/null
grep '"event":"backend.preflight"' "$out_dir/backend-preflight.jsonl" >/dev/null
grep '"ready":true' "$out_dir/backend-preflight.jsonl" >/dev/null
grep '"event":"protocol.smoke"' "$out_dir/protocols.jsonl" >/dev/null
grep '"required_protocols":7' "$out_dir/protocols.jsonl" >/dev/null
grep '"event":"perf.smoke"' "$out_dir/perf.jsonl" >/dev/null
grep '"passed":true' "$out_dir/perf.jsonl" >/dev/null
grep '"golden_ok":true' "$out_dir/perf.jsonl" >/dev/null
grep '"event":"shell.verified"' "$out_dir/shell.jsonl" >/dev/null
grep '"required_components":4' "$out_dir/shell.jsonl" >/dev/null
grep '"event":"launcher.verified"' "$out_dir/launcher.jsonl" >/dev/null
grep '"required_targets":3' "$out_dir/launcher.jsonl" >/dev/null
grep '"target":"terminal"' "$out_dir/launcher.jsonl" >/dev/null
grep '"event":"demo_client.verified"' "$out_dir/demo-client.jsonl" >/dev/null
grep '"passed":true' "$out_dir/demo-client.jsonl" >/dev/null
grep '"golden_ok":true' "$out_dir/demo-client.jsonl" >/dev/null
test -s "$out_dir/backlit-session.ppm"
test -s "$out_dir/demo-client.ppm"

session_ppm_bytes="$(wc -c < "$out_dir/backlit-session.ppm" | tr -d ' ')"
demo_ppm_bytes="$(wc -c < "$out_dir/demo-client.ppm" | tr -d ' ')"
test "$session_ppm_bytes" = "$expected_ppm_bytes"
test "$demo_ppm_bytes" = "$expected_ppm_bytes"

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-gui-smoke",
  "passed": true,
  "backend": "headless",
  "socket": "backlit-0",
  "width": $expected_width,
  "height": $expected_height,
  "checksum": $expected_checksum,
  "expected_ppm_bytes": $expected_ppm_bytes,
  "artifacts": {
    "compositor_log": "$out_dir/compositor.jsonl",
    "backend_preflight_log": "$out_dir/backend-preflight.jsonl",
    "protocols_log": "$out_dir/protocols.jsonl",
    "perf_log": "$out_dir/perf.jsonl",
    "shell_log": "$out_dir/shell.jsonl",
    "launcher_log": "$out_dir/launcher.jsonl",
    "session_log": "$out_dir/session.jsonl",
    "demo_client_log": "$out_dir/demo-client.jsonl",
    "session_screenshot": "$out_dir/backlit-session.ppm",
    "demo_client_screenshot": "$out_dir/demo-client.ppm"
  },
  "checks": {
    "protocol_required_count": 7,
    "shell_required_components": 4,
    "launcher_required_targets": 3,
    "session_ppm_bytes": $session_ppm_bytes,
    "demo_ppm_bytes": $demo_ppm_bytes,
    "golden_checksum": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit GUI smoke verification passed. Artifacts: %s\n' "$out_dir"
