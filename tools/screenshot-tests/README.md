# Screenshot Tests

MVP 0 renders deterministic PPM screenshots through `backlit-session` and `backlit-demo-client`.

```bash
./scripts/verify-gui-smoke.sh
```

The verifier checks that expected GUI regions are present and writes artifacts to `target/gui-smoke/`. This is the first golden screenshot path; nested Wayland and real compositor captures should build on this instead of replacing it.

The default 800x520 render is also checked against checksum `5635038614353063225` so accidental visual drift fails the smoke verifier.
