# Parallels Ubuntu Read-Only Root Recovery

Backlit's Parallels E2E runners require the Ubuntu guest root filesystem and
`/tmp` to be writable. If `scripts/verify-parallels-ubuntu-health.sh` reports
`guest-root-read-only`, the runner stops before uploading scripts, installing
packages, switching TTYs, or running the long E2E gates.

## Evidence

The health verifier writes a manifest before exiting:

```bash
./scripts/verify-parallels-ubuntu-health.sh target/parallels-ubuntu-health
```

Relevant fields:

- `reason: "guest-root-read-only"` means the root mount flags are read-only.
- `root_mount` should show the affected root device, for example `/dev/sda2 / ext4 ro,relatime 0 0`.
- `root_filesystem_writable: false` means root cannot write under `/root`.
- `tmp_writable: false` means root cannot write under `/tmp`.
- `e2e_ready: false` means the Parallels E2E runners must not continue.

## Recovery Order

1. Take a Parallels snapshot or otherwise preserve the current VM state.
2. Prefer a normal Parallels UI shutdown and start. Avoid forced stop unless the
   guest will not shut down cleanly.
3. Re-run the health verifier:

   ```bash
   ./scripts/verify-parallels-ubuntu-health.sh target/parallels-ubuntu-health-after-restart
   ```

4. If the verifier passes, run the true E2E gates:

   ```bash
   ./scripts/verify-parallels-linux-e2e.sh target/linux-e2e-parallels
   ./scripts/verify-parallels-dedicated-drm-e2e.sh target/parallels-dedicated-drm-e2e
   ./scripts/verify-mvp-complete.sh target/mvp-complete target/linux-e2e-parallels target/parallels-dedicated-drm-e2e
   ```

5. If the root filesystem is still read-only, repair it offline from Ubuntu
   recovery mode or a live installer environment. Do not run `fsck` against a
   mounted root filesystem. The current observed root device is recorded in the
   health manifest; for example, if it reports `/dev/sda2`, the offline repair
   command is:

   ```bash
   sudo fsck -f /dev/sda2
   ```

6. Boot the repaired VM normally and run the health verifier again before any
   E2E runner.

## Completion Criteria

The recovery is complete only when both normal and dedicated Parallels E2E
outputs contain a passing `parallels-ubuntu-health/manifest.json` with:

- `passed: true`
- `e2e_ready: true`
- `root_filesystem_writable: true`
- `tmp_writable: true`

The MVP completion gate also requires those health manifests before it accepts
launch-performance, resource-budget, GUI preview, package-install, and
dedicated DRM evidence.
