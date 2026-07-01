# Hardware Lab

Track real machines here. VM behavior is useful for development, but final performance and compatibility decisions need physical hardware.

For the MVP 1 DRM handoff, run this from a seat-owner TTY or display-manager Backlit session:

```bash
BACKLIT_REQUIRE_DEDICATED_DRM_SESSION=1 \
BACKLIT_REQUIRE_DRM_MASTER_PRESENT=1 \
  ./scripts/verify-dedicated-drm-session.sh target/dedicated-drm-session-acceptance
```

Parallels E2E exports the same command as `dedicated-drm-handoff.sh` next to the dedicated-session manifest, but it is expected to remain DRM-master blocked inside the nested desktop.

Suggested first matrix:

- low-end Intel laptop;
- midrange AMD laptop;
- low-resource VM;
- HiDPI laptop;
- NVIDIA machine after MVP 2.
