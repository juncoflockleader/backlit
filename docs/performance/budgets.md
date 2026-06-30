# Performance Budgets

Initial MVP targets:

| Metric | MVP target | Stretch target |
| --- | ---: | ---: |
| Compositor startup after session launch | < 500 ms | < 250 ms |
| Shell ready after session launch | < 2 s | < 1 s |
| Compositor idle CPU | < 0.5% | near zero |
| Compositor + shell idle RSS | < 250 MB | < 150 MB |
| Pointer-to-frame latency at 60 Hz | < 16 ms p99 | < 8 ms p95 |
| Dropped frames during window drag | < 1% | near zero |
| Terminal launch after hotkey | < 300 ms | < 150 ms |

Any change to compositor startup, input, rendering, app launch, shell startup, or session services should carry a benchmark or regression check.

## MVP 0 Smoke Check

The render/present regression check is:

```bash
cargo run -p backlit-perf -- --verify
```

It measures the deterministic headless GUI render path, the in-memory headless backend present path, no-redraw idle frames, targeted surface damage, direct-scanout eligibility, and a 60-frame drag pacing loop. This does not prove real compositor latency, but it catches early regressions while nested Wayland and DRM/KMS backends are still being built.

The launch-path regression check is:

```bash
./scripts/verify-launch-performance.sh
```

It builds the session, compositor, and shell binaries, runs `backlit-session` directly, and verifies the current MVP budgets for GUI readiness after session launch, shell-ready service probes after launch, and terminal hotkey spawn time. The Linux E2E gate includes this verifier and publishes `target/linux-e2e/launch-performance/manifest.json`.

The Linux resource-budget regression check is:

```bash
./scripts/verify-resource-budget.sh
```

It runs bounded idle probes for the compositor and shell, samples `/proc`, and verifies compositor idle CPU stays under 0.5% while combined compositor+shell RSS stays under 250 MB. Non-Linux hosts record an expected skip; use Parallels Ubuntu or another Linux host for the real budget proof.
