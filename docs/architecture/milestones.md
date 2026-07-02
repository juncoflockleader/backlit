# Backlit Milestones

This roadmap starts from the current verified state: Backlit can launch in the Parallels Ubuntu E2E environment, run Smithay readiness probes, prove dedicated DRM first-present through the packaged session path, render real generated `wl_shm` client pixels into a Backlit frame through the `--smithay-real-shm-frame` proof path, and render pixels from an installed Wayland app through the `--smithay-real-app-e2e` proof path.

The next milestones move those proof paths from focused verifiers into the normal compositor loop, then add lifecycle, input, and production rendering behavior.

## Current Status

| Milestone | Status | Exit evidence |
| --- | --- | --- |
| 1. Live Surface Snapshots | Complete | `scripts/verify-smithay-live-surface-snapshots.sh` and Linux/Parallels E2E manifests include copied real `wl_shm` snapshots. |
| 2. Real Clients In The Normal Frame Loop | Complete | `--runtime=smithay --scripted-client` emits normal-frame live snapshot evidence and exports a Backlit frame composed from real generated `wl_shm` pixels; MVP-complete passed on commit `4ce6c97`. |
| 3. Surface Lifecycle | Complete | `--runtime=smithay --scripted-client` emits generated-client resize, unmap, close, destroy, disconnect, and Backlit policy cleanup evidence; Parallels Linux E2E passed on commit `b931ddb`. |
| 4. Input To Real Clients | In progress | Keyboard and pointer events reach real generated clients while compositor shortcut filtering remains active. |
| 5. Real App E2E | Complete | Parallels exports a Backlit frame containing pixels from `/usr/bin/weston-simple-shm`, with server-side SHM capture and Backlit frame sample verification. |
| 6. GPU Texture Compositing | Pending | Real client buffers render through the GPU path with SHM CPU upload as fallback. |
| 7. MVP 1 Closure | Pending | MVP contract and complete gates require real client/rendering evidence. |

The active implementation target is **Milestone 4: Input To Real Clients**. Milestone 3 extended the real-client path through xdg configure/ack, resize, unmap, close, and disconnect policy cleanup; Milestone 4 now routes Smithay seat keyboard and pointer events into real generated Wayland clients.

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

### Current Slice

- `SmithayCompositorRuntime::present()` can report live snapshot-backed frames when a real generated client has committed pixels and no mock backend surfaces are present.
- `backlit-compositor --backend=drm --runtime=smithay --scripted-client --scripted-client-preview <path>` writes a normal runtime frame with the live snapshot composited through Backlit policy geometry.
- `scripts/verify-smithay-compositor-runtime.sh` requires `smithay_normal_runtime_live_snapshot_frame`, `smithay_normal_runtime_real_pixels`, and sample-verified real pixels before passing on launch-ready Linux.
- Verified on commit `4ce6c97` with Parallels Linux E2E, dedicated DRM E2E, and MVP-complete.

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

### Current Slice

- `SmithayCompositorRuntime::run_surface_lifecycle_capture()` keeps a generated xdg client connected after the initial real `wl_shm` map, sends a compositor resize configure, waits for the client ack and resized 420x300 SHM commit, records the null-buffer unmap, sends xdg close, observes client close handling, destroys the xdg objects, and dispatches client disconnect cleanup.
- `backlit-compositor --backend=drm --runtime=smithay --scripted-client` now emits `real_surface_lifecycle`, `real_surface_resize_committed`, `real_surface_unmapped`, `real_surface_close_received`, `real_surface_client_disconnected`, and Backlit policy cleanup fields from the normal Smithay scripted runtime event.
- `scripts/verify-smithay-compositor-runtime.sh`, Linux E2E, Parallels export, MVP 1 contract, and MVP-complete require Smithay surface lifecycle manifest keys before launch-ready evidence can pass.

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
- `./scripts/verify-smithay-compositor-runtime.sh`
- `./scripts/verify-drm-session-smoke.sh`
- `./scripts/verify-linux-e2e.sh`
- `./scripts/verify-parallels-mvp-e2e.sh`

### Current Slice

- `SmithayCompositorRuntime::run_real_input_capture()` maps two generated real `wl_shm` xdg clients, focuses the active SHM toplevel through Smithay keyboard and pointer handles, sends pointer enter/motion/button events and keyboard enter/key events, then maps a second client and verifies new input routes to the second client without adding button/key events to the first.
- The generated client records `wl_pointer` enter, leave, motion, button, and frame events plus `wl_keyboard` keymap, enter, leave, key, modifiers, and repeat-info events.
- The Smithay keyboard filter now proves ordinary keys are forwarded to the focused real client while a compositor-reserved shortcut probe is intercepted and not forwarded.
- `scripts/verify-smithay-compositor-runtime.sh`, Linux E2E, Parallels export, MVP 1 contract, and MVP-complete require `smithay_real_client_input`, `smithay_real_pointer_input`, `smithay_real_keyboard_input`, `smithay_real_input_focus_routing`, and `smithay_shortcut_filter_preserved`.

### Dependencies

- Milestone 3 surface lifecycle.
- Existing Smithay seat handles and libinput event-loop readiness.

## Milestone 5: Real App E2E

### Goal

Launch a real installed Wayland application inside Backlit and verify that it renders through the compositor with preserved metadata and visible pixels.

### Scope

- Start Backlit in the Parallels Ubuntu environment.
- Launch a known installed Wayland client, preferring `weston-simple-shm` for the first deterministic slice.
- Accept the external client's Wayland connection through the Smithay runtime.
- Capture committed external-client `wl_shm` pixels from Smithay-owned buffer state.
- CPU-compose those pixels into a Backlit frame using `WindowPolicy` geometry.
- Capture a Backlit frame containing the real app.
- Verify app process launch, Wayland client connection, title/app id when available, window geometry, nonblank app pixels, frame sample pixels, and clean shutdown or bounded forced teardown after capture.

### Acceptance

- Parallels Linux E2E exports a screenshot containing a real installed Wayland app rendered by Backlit.
- Manifest proves the app was not a mock or generated-only test surface.
- Manifest proves server-side SHM pixels were copied from the external client and composited into the exported Backlit frame.
- Dedicated DRM E2E remains green.
- Verified on commit `410a2cb` with `scripts/verify-parallels-linux-e2e.sh`, `scripts/verify-parallels-dedicated-drm-e2e.sh`, and `scripts/verify-mvp-complete.sh`.

### Verification

- `cargo test --workspace`
- `./scripts/verify-smithay-real-app-e2e.sh`
- `./scripts/verify-linux-e2e.sh`
- `./scripts/verify-parallels-dedicated-drm-e2e.sh`
- `./scripts/verify-parallels-mvp-e2e.sh`

### Dependencies

- Milestone 1 live surface snapshots.
- Existing real-SHM frame composition proof.
- Stable app launch environment inside Ubuntu.
- Milestone 4 input is not required for the first deterministic `weston-simple-shm` slice, but is required before interactive real apps are considered complete.

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
