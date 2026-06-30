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

The current automated regression check is:

```bash
cargo run -p backlit-perf -- --verify
```

It measures the deterministic headless GUI render path and the in-memory headless backend present path. This does not prove real compositor latency, but it catches early regressions while nested Wayland and DRM/KMS backends are still being built.
