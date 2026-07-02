# Backlit Milestones

This roadmap starts from the current verified state: Backlit can launch in the Parallels Ubuntu E2E environment, run Smithay readiness probes, prove dedicated DRM first-present through the packaged session path, and render real generated `wl_shm` client pixels into a Backlit frame through the `--smithay-real-shm-frame` proof path.

The next milestones move that proof from a verifier slice into the normal compositor loop.

## Milestone 1: Live Surface Snapshots

### Goal

Promote the real-SHM capture path from a one-shot verifier into a durable Smithay-to-Backlit surface snapshot pipeline.

### Scope

- Track Smithay-observed Wayland surfaces across commits.
- Store title, app id, size, buffer format, stride, commit serial, and damage metadata.
- Copy committed `wl_shm` pixels into a Backlit-owned snapshot buffer.
- Preserve the existing `--smithay-real-shm-frame` verifier as a focused regression test.

### Acceptance

- `backlit-compositor --backend=drm --runtime=smithay` can maintain at least one live real client snapshot after the client commits a SHM buffer.
- Snapshot metadata maps into `SurfaceManager` and `WindowPolicy` without using mock window data.
- A verifier records real client metadata, commit count, damage region, pixel checksum, and sample colors.

### Verification

- `cargo test --workspace`
- `./scripts/verify-smithay-real-shm-frame.sh`
- New live snapshot verifier, for example `./scripts/verify-smithay-live-surface-snapshots.sh`

### Dependencies

- Existing Smithay runtime bootstrap.
- Existing real-SHM proof path.
- No GPU texture compositing required.

## Milestone 2: Real Clients In The Normal Frame Loop

### Goal

Render live real Wayland client snapshots in the normal Backlit compositor frame loop instead of only in the real-SHM proof command.

### Scope

- Feed real surface snapshots into the runtime frame model.
- Replace mock demo windows with real client surfaces when `--runtime=smithay`.
- Compose client pixels through the existing Backlit frame renderer and `WindowPolicy` geometry.
- Keep mock/demo surfaces available for headless and deterministic development paths.

### Acceptance

- A real generated Wayland client appears in a normal compositor frame.
- Frame output verifies source pixel samples after policy placement.
- Closing the client removes the corresponding managed window and damages the output.
- Existing mock runtime tests continue to pass.

### Verification

- `cargo test --workspace`
- `./scripts/verify-compositor-runtime.sh`
- `./scripts/verify-smithay-compositor-runtime.sh`
- `./scripts/verify-linux-e2e.sh`

### Dependencies

- Milestone 1 live surface snapshots.
- Stable policy geometry for real surfaces.

## Milestone 3: Surface Lifecycle

### Goal

Handle normal xdg-shell client lifecycle events with real windows: map, unmap, resize, focus, close, and destroy.

### Scope

- Track xdg-toplevel configure/ack cycles for real clients.
- Update snapshots when clients resize and commit new buffers.
- Remove Backlit managed windows when surfaces unmap or clients disconnect.
- Preserve stacking and focus order across lifecycle changes.

### Acceptance

- A generated test client can map, resize, commit a new buffer, close, and disconnect.
- Backlit policy state follows each lifecycle transition.
- No stale managed windows remain after client disconnect.

### Verification

- `cargo test --workspace`
- `./scripts/verify-smithay-compositor-runtime.sh`
- `./scripts/verify-compositor-socket.sh`
- `./scripts/verify-linux-e2e.sh`

### Dependencies

- Milestone 2 real clients in the normal frame loop.
- Existing `WindowPolicy` lifecycle behavior.

## Milestone 4: Input To Real Clients

### Goal

Route keyboard and pointer input from the Backlit/Smithay runtime into real Wayland clients.

### Scope

- Connect libinput events to Smithay seat keyboard and pointer handles.
- Maintain pointer focus, keyboard focus, enter/leave, button, motion, and key dispatch.
- Preserve Backlit shell shortcuts such as terminal launch and app switching.
- Add an interactive generated client verifier for click and text input.

### Acceptance

- A real client receives pointer enter/motion/button events.
- A real client receives keyboard enter/key events.
- Backlit compositor-level shortcuts still work.
- Focus changes route input to the expected client.

### Verification

- `cargo test --workspace`
- `backlit-input --verify`
- `./scripts/verify-drm-session-smoke.sh`
- `./scripts/verify-linux-e2e.sh`
- `./scripts/verify-parallels-mvp-e2e.sh`

### Dependencies

- Milestone 3 surface lifecycle.
- Existing Smithay seat handles and libinput event-loop readiness.

## Milestone 5: Real App E2E

### Goal

Launch a real installed Wayland application inside Backlit and verify that it renders through the compositor with preserved metadata and visible pixels.

### Scope

- Start Backlit in the Parallels Ubuntu environment.
- Launch a known Wayland client such as `foot`.
- Capture a Backlit frame containing the real app.
- Verify title/app id, window geometry, nonblank app pixels, and clean shutdown.

### Acceptance

- Parallels Linux E2E exports a screenshot containing a real installed Wayland app rendered by Backlit.
- Manifest proves the app was not a mock or generated-only test surface.
- Dedicated DRM E2E remains green.

### Verification

- `./scripts/verify-linux-e2e.sh`
- `./scripts/verify-parallels-dedicated-drm-e2e.sh`
- `./scripts/verify-parallels-mvp-e2e.sh`

### Dependencies

- Milestone 4 input to real clients.
- Stable app launch environment inside Ubuntu.

## Milestone 6: GPU Texture Compositing

### Goal

Move beyond CPU SHM composition toward real compositor rendering paths suitable for production performance.

### Scope

- Import client buffers into renderer-managed textures where possible.
- Keep SHM CPU upload as a fallback path.
- Integrate damage-aware redraw and presentation timing.
- Preserve direct-scanout eligibility checks for fullscreen surfaces.

### Acceptance

- A real client surface is rendered through the GPU path on launch-ready Linux.
- SHM fallback still works.
- Frame timing and resource-budget checks remain within MVP budgets.

### Verification

- `cargo test --workspace`
- `backlit-perf --verify`
- `./scripts/verify-resource-budget.sh`
- `./scripts/verify-drm-session-smoke.sh`
- `./scripts/verify-parallels-mvp-e2e.sh`

### Dependencies

- Milestone 5 real app E2E.
- Smithay renderer/allocator integration beyond the current first-present probes.

## Milestone 7: MVP 1 Closure

### Goal

Close the MVP 1 contract with real windows, real input, real app launch, package-installed session verification, and exported Parallels evidence.

### Scope

- Require real-client rendering evidence in the MVP 1 contract.
- Require real app launch evidence in Linux E2E.
- Keep dedicated DRM first-present and package installation gates mandatory.
- Refresh development instructions and recovery runbooks.

### Acceptance

- `scripts/verify-mvp1-contract.sh` passes with real client rendering required.
- `scripts/verify-mvp-complete.sh` passes from exported Parallels artifacts.
- The repository documents how to run and inspect the real GUI path.

### Verification

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `./scripts/verify-parallels-mvp-e2e.sh`

### Dependencies

- Milestone 5 real app E2E.
- Milestone 6 GPU texture compositing may remain optional if CPU composition satisfies MVP 1 performance budgets.
