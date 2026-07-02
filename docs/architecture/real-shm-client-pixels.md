# Real SHM Client Pixels

## Summary

This milestone proves that Backlit can accept a real Wayland `wl_shm` client surface through the Smithay runtime, capture the committed ARGB/XRGB pixel payload, and render those client pixels into a Backlit frame using the normal `WindowPolicy` geometry.

The slice is intentionally CPU-composited. It does not replace the dedicated DRM/KMS first-present path and does not claim GPU texture compositing.

## Scope

- Generate a deterministic real Wayland client inside the Smithay runtime.
- Bind the MVP protocol globals: `wl_compositor`, `wl_shm`, `xdg_wm_base`, `wl_output`, `xdg_output_manager`, `wp_viewporter`, `wp_presentation`, and `zwp_linux_dmabuf`.
- Create and commit an ARGB8888 `wl_shm` buffer with deterministic sample colors.
- Observe the Smithay-side surface commit, title, app id, and committed buffer dimensions.
- Copy the committed SHM backing bytes into a Backlit-owned pixel frame after Smithay has observed the commit.
- Map the observed client metadata into a managed Backlit toplevel through `SurfaceManager` and `WindowPolicy`.
- CPU-compose the captured client pixels at the policy window geometry.
- Write a PPM artifact and verify source and frame sample pixels.

## Public Interfaces

- `backlit-compositor --smithay-real-shm-frame`
  Runs the real SHM client frame capture and verification path.
- `backlit-compositor --smithay-real-shm-frame-output <path>`
  Writes the composed Backlit frame to a custom PPM path and implies `--smithay-real-shm-frame`.
- `scripts/verify-smithay-real-shm-frame.sh [out-dir]`
  Builds the Smithay compositor binary, runs the real SHM frame path on Linux, and writes a manifest.

## Verification

The verifier checks:

- Smithay runtime startup and backend launch preflight.
- Real Wayland client protocol smoke success.
- Smithay-observed title, app id, surface commit, and SHM dimensions.
- Captured client pixel count, stride, format, and deterministic sample colors.
- Backlit `WindowPolicy` mapping and preserved app id.
- CPU composition of all client pixels into the Backlit frame.
- Frame sample pixels match the source client SHM samples.
- PPM frame artifact exists and has nontrivial size.

## Assumptions

- This milestone proves real `wl_shm` pixels in a Backlit-rendered frame, not full GPU texture compositing.
- DRM first-present remains covered by the existing dedicated DRM E2E path.
- The generated client keeps the SHM file alive long enough for the runtime to copy the committed bytes after Smithay has observed the commit.
