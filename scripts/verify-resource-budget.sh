#!/usr/bin/env sh
set -eu

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

out_dir="${1:-target/resource-budget}"
mkdir -p "$out_dir"

compositor_log="$out_dir/compositor.jsonl"
compositor_err="$out_dir/compositor.stderr"
shell_log="$out_dir/shell.jsonl"
shell_err="$out_dir/shell.stderr"

idle_probe_ms="${BACKLIT_RESOURCE_IDLE_PROBE_MS:-4000}"
warmup_ms="${BACKLIT_RESOURCE_WARMUP_MS:-300}"
sample_ms="${BACKLIT_RESOURCE_SAMPLE_MS:-3000}"
compositor_cpu_budget_percent="${BACKLIT_COMPOSITOR_IDLE_CPU_BUDGET_PERCENT:-0.5}"
combined_rss_budget_kib="${BACKLIT_IDLE_RSS_BUDGET_KIB:-256000}"

fail() {
  echo "Backlit resource budget verification failed: $*" >&2
  exit 1
}

sleep_ms() {
  ms="$1"
  seconds="$(awk -v ms="$ms" 'BEGIN { printf "%.3f", ms / 1000 }')"
  sleep "$seconds"
}

float_le() {
  value="$1"
  budget="$2"
  awk -v value="$value" -v budget="$budget" 'BEGIN { exit !(value <= budget) }'
}

proc_ticks() {
  pid="$1"
  test -r "/proc/$pid/stat" || fail "missing /proc stat for pid $pid"
  stat_after_comm="$(sed 's/^[^)]*) //' "/proc/$pid/stat")"
  set -- $stat_after_comm
  printf '%s\n' "$(( ${12} + ${13} ))"
}

rss_kib() {
  pid="$1"
  test -r "/proc/$pid/status" || fail "missing /proc status for pid $pid"
  awk '/^VmRSS:/ { print $2; found = 1 } END { if (!found) print 0 }' "/proc/$pid/status"
}

cpu_percent_for_delta() {
  delta_ticks="$1"
  ticks_per_second="$2"
  measured_ms="$3"
  awk \
    -v delta_ticks="$delta_ticks" \
    -v ticks_per_second="$ticks_per_second" \
    -v measured_ms="$measured_ms" \
    'BEGIN { printf "%.3f", ((delta_ticks / ticks_per_second) / (measured_ms / 1000)) * 100 }'
}

cleanup() {
  for pid in ${compositor_pid:-} ${shell_pid:-}; do
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
}

if [ "$(uname -s)" != "Linux" ]; then
  cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-resource-budget",
  "passed": true,
  "target_os": "$(uname -s)",
  "resource_budget_checked": false,
  "resource_budget_blocked_expected": true,
  "checks": {
    "linux_procfs_required": true,
    "idle_cpu_budget": false,
    "idle_rss_budget": false
  }
}
EOF

  printf 'Backlit resource budget verification skipped on non-Linux host. Artifacts: %s\n' "$out_dir"
  exit 0
fi

minimum_probe_ms=$((warmup_ms + sample_ms + 200))
if [ "$idle_probe_ms" -lt "$minimum_probe_ms" ]; then
  fail "idle probe ${idle_probe_ms}ms is too short for warmup ${warmup_ms}ms and sample ${sample_ms}ms"
fi

cargo build -p backlit-compositor -p backlit-shell

target/debug/backlit-compositor \
  --backend=headless \
  --socket=backlit-resource-budget \
  --idle-probe-ms="$idle_probe_ms" > "$compositor_log" 2> "$compositor_err" &
compositor_pid="$!"

target/debug/backlit-shell \
  --component=all \
  --socket=backlit-resource-budget \
  --verify \
  --idle-probe-ms="$idle_probe_ms" > "$shell_log" 2> "$shell_err" &
shell_pid="$!"

trap cleanup EXIT HUP INT TERM

sleep_ms "$warmup_ms"

kill -0 "$compositor_pid" 2>/dev/null || fail "compositor exited before idle sampling"
kill -0 "$shell_pid" 2>/dev/null || fail "shell exited before idle sampling"

ticks_per_second="$(getconf CLK_TCK)"
compositor_ticks_before="$(proc_ticks "$compositor_pid")"
shell_ticks_before="$(proc_ticks "$shell_pid")"
compositor_rss_kib="$(rss_kib "$compositor_pid")"
shell_rss_kib="$(rss_kib "$shell_pid")"

sleep_ms "$sample_ms"

compositor_ticks_after="$(proc_ticks "$compositor_pid")"
shell_ticks_after="$(proc_ticks "$shell_pid")"

set +e
wait "$compositor_pid"
compositor_status="$?"
wait "$shell_pid"
shell_status="$?"
set -e
trap - EXIT HUP INT TERM

test "$compositor_status" -eq 0 || fail "compositor idle probe failed with status $compositor_status"
test "$shell_status" -eq 0 || fail "shell idle probe failed with status $shell_status"

grep '"event":"compositor.ready"' "$compositor_log" >/dev/null
grep '"ready":true' "$compositor_log" >/dev/null
grep '"accepting_clients":true' "$compositor_log" >/dev/null
grep '"bootstrap_client_connected":true' "$compositor_log" >/dev/null
grep '"bootstrap_surface_presented":true' "$compositor_log" >/dev/null
grep '"presented_pixels":1' "$compositor_log" >/dev/null
grep '"event":"compositor.idle_probe_start"' "$compositor_log" >/dev/null
grep '"event":"compositor.idle_probe_complete"' "$compositor_log" >/dev/null
grep '"event":"compositor.exit"' "$compositor_log" >/dev/null
grep '"event":"shell.verified"' "$shell_log" >/dev/null
grep '"event":"shell.idle_probe_start"' "$shell_log" >/dev/null
grep '"event":"shell.idle_probe_complete"' "$shell_log" >/dev/null

compositor_tick_delta=$((compositor_ticks_after - compositor_ticks_before))
shell_tick_delta=$((shell_ticks_after - shell_ticks_before))
combined_tick_delta=$((compositor_tick_delta + shell_tick_delta))
combined_rss_kib=$((compositor_rss_kib + shell_rss_kib))

compositor_idle_cpu_percent="$(cpu_percent_for_delta "$compositor_tick_delta" "$ticks_per_second" "$sample_ms")"
shell_idle_cpu_percent="$(cpu_percent_for_delta "$shell_tick_delta" "$ticks_per_second" "$sample_ms")"
combined_idle_cpu_percent="$(cpu_percent_for_delta "$combined_tick_delta" "$ticks_per_second" "$sample_ms")"

float_le "$compositor_idle_cpu_percent" "$compositor_cpu_budget_percent" \
  || fail "compositor idle CPU ${compositor_idle_cpu_percent}% exceeded budget ${compositor_cpu_budget_percent}%"

if [ "$combined_rss_kib" -gt "$combined_rss_budget_kib" ]; then
  fail "combined compositor+shell RSS ${combined_rss_kib} KiB exceeded budget ${combined_rss_budget_kib} KiB"
fi

cat > "$out_dir/manifest.json" <<EOF
{
  "name": "backlit-resource-budget",
  "passed": true,
  "target_os": "$(uname -s)",
  "resource_budget_checked": true,
  "resource_budget_blocked_expected": false,
  "artifacts": {
    "compositor_log": "$compositor_log",
    "compositor_stderr": "$compositor_err",
    "shell_log": "$shell_log",
    "shell_stderr": "$shell_err"
  },
  "budgets": {
    "compositor_idle_cpu_percent": $compositor_cpu_budget_percent,
    "combined_rss_kib": $combined_rss_budget_kib
  },
  "measurements": {
    "idle_probe_ms": $idle_probe_ms,
    "warmup_ms": $warmup_ms,
    "sample_ms": $sample_ms,
    "ticks_per_second": $ticks_per_second,
    "compositor_tick_delta": $compositor_tick_delta,
    "shell_tick_delta": $shell_tick_delta,
    "compositor_idle_cpu_percent": $compositor_idle_cpu_percent,
    "shell_idle_cpu_percent": $shell_idle_cpu_percent,
    "combined_idle_cpu_percent": $combined_idle_cpu_percent,
    "compositor_rss_kib": $compositor_rss_kib,
    "shell_rss_kib": $shell_rss_kib,
    "combined_rss_kib": $combined_rss_kib
  },
  "checks": {
    "linux_procfs_required": true,
    "compositor_service_ready": true,
    "compositor_accepting_clients": true,
    "compositor_bootstrap_surface": true,
    "compositor_idle_probe": true,
    "shell_idle_probe": true,
    "idle_cpu_budget": true,
    "idle_rss_budget": true
  }
}
EOF

grep '"passed": true' "$out_dir/manifest.json" >/dev/null

printf 'Backlit resource budget verification passed. Artifacts: %s\n' "$out_dir"
