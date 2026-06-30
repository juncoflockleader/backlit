# Performance Tools

Backlit treats "fast" as something measured.

Initial metrics to collect:

- compositor startup after session launch;
- shell ready after session launch;
- compositor idle CPU;
- compositor and shell idle RSS;
- pointer-to-frame latency;
- dropped frames during window drag;
- terminal launch latency;
- fullscreen direct scanout success.

Current MVP 0 smoke command:

```bash
cargo run -p backlit-perf -- --verify
```

This is intentionally narrow: it checks deterministic GUI render time and headless backend present time so regressions are visible before real frame pacing instrumentation exists.
