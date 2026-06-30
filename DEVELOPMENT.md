# Backlit Development

Backlit is being scaffolded for MVP 0 from `backlit-design.md`: a testable development harness for a fast Wayland-native Linux desktop.

## Current Direction

- Project identity: `backlit`.
- Rust crate prefix: `backlit-*`.
- Packaging names will follow the design's `fastgui-*` package split later where useful.
- First compositor framework target: Smithay.
- First supported development target: Ubuntu 26.04 LTS.
- Secondary compatibility target: Ubuntu 24.04 LTS.
- macOS is supported for editing, Git, and pure Rust tests only. Real graphics/input validation belongs in Linux VMs and on real Linux hardware.

## Repository Layout

```text
crates/
  common/              Shared config, logging, metrics helpers.
  compositor/          Compositor binary entrypoint.
  compositor-backend/  Backend selection and runtime config.
  input/               Keyboard and pointer event routing smoke checks.
  surface/             xdg-shell-style toplevel lifecycle smoke checks.
  window-policy/       Focus, placement, workspaces, snapping; pure logic.
  shell-protocol/      Private shell/compositor protocol model.
  shell/               Shell host stub for panel/launcher/wallpaper work.
  settings-daemon/     Future power/input/display state daemon.
  portal-backend/      Future xdg-desktop-portal backend.
apps/
  settings/            Future settings UI.
  launcher/            Future launcher UI.
  terminal-wrapper/    Future terminal launch wrapper.
packaging/
  debian/              Debian package metadata skeleton.
  sessions/            Desktop/session entries.
  systemd/             User/system service skeletons.
tools/
  perf/                Benchmarks and metrics tooling.
  screenshot-tests/    Golden screenshot test harness.
  hardware-lab/        Hardware matrix and lab scripts.
docs/
  architecture/        Architecture notes.
  protocols/           Protocol support tracking.
  performance/         Budgets and measurement notes.
```

## Host Prerequisites

The current scaffold has no third-party Rust dependencies, so the workspace can build with the local Rust toolchain:

```bash
rustc --version
cargo --version
cargo test --workspace
```

Recommended local tools:

- Rust 1.76 or newer.
- `rustfmt` and `clippy`.
- Git.
- On Linux: Wayland/Mesa/input development packages listed below.

## Ubuntu Development Setup

On Ubuntu 26.04 or 24.04:

```bash
sudo ./scripts/bootstrap-ubuntu.sh
cargo test --workspace
cargo run -p backlit-compositor -- --backend=headless --smoke-test
```

The bootstrap script installs compiler, Wayland, Mesa, libinput, systemd/logind, seat, Weston, Xwayland, and a small terminal for nested testing.

## macOS + Multipass Workflow

Use macOS as the editor and Git host, and use an Ubuntu VM for Linux builds and headless/nested checks.

```bash
brew install --cask multipass
multipass launch 26.04 \
  --name backlit-dev \
  --cpus 6 \
  --memory 12G \
  --disk 80G \
  --cloud-init dev/multipass-cloud-init.yaml
multipass mount "$PWD" backlit-dev:/work/backlit
multipass shell backlit-dev
cd /work/backlit
cargo test --workspace
```

Do not treat VM graphics performance as real product performance. Use it for build checks, protocol smoke tests, nested Wayland behavior, and CI reproduction.

## macOS + Parallels Linux E2E

The Parallels runner bootstraps an Ubuntu guest, updates a clean checkout from `https://github.com/juncoflockleader/backlit.git`, installs a current Rust toolchain with `rustfmt` and `clippy`, and runs the Linux E2E gate:

```bash
./scripts/verify-parallels-linux-e2e.sh
./scripts/render-parallels-gui-preview.sh
```

The E2E runner verifies the full Ubuntu path. The preview runner renders the Backlit preview inside Ubuntu, copies the generated PPM/session logs/manifest back to `target/gui-preview-parallels/`, and converts the PPM to a local PNG on macOS when possible.

The runners read a local credential file that is ignored by Git:

```bash
mkdir -p .local
cat > .local/parallels-ubuntu.env <<'EOF'
BACKLIT_PARALLELS_UBUNTU_USER=<guest-admin-user>
BACKLIT_PARALLELS_UBUNTU_PASSWORD=<guest-password>
EOF
chmod 600 .local/parallels-ubuntu.env
```

Useful overrides:

```bash
BACKLIT_PARALLELS_VM="Ubuntu 22.04.2 ARM64" ./scripts/verify-parallels-linux-e2e.sh
BACKLIT_E2E_BRANCH=main BACKLIT_E2E_OUT_DIR=target/linux-e2e-parallels ./scripts/verify-parallels-linux-e2e.sh
```

The Linux-side verifier can also be run directly inside any Ubuntu checkout:

```bash
./scripts/verify-linux-e2e.sh
```

It runs `cargo fmt`, workspace tests, `cargo clippy`, the deterministic GUI smoke verifier, the preview renderer, launch-performance verifier, resource-budget verifier, portal-security verifier, CI contract verifier, packaging contract verifier, staged session install verifier, launch-readiness verifier, session launch verifier, session clean-exit verifier, nested Wayland smoke verifier, and MVP 0 contract verifier, then writes `target/linux-e2e/manifest.json`.

## GUI Linux VM Workflow

Inside a GUI Linux VM, start a parent Wayland session:

```bash
dbus-run-session -- weston --socket=parent-wayland
```

In another terminal inside the VM:

```bash
export WAYLAND_DISPLAY=parent-wayland
cargo run -p backlit-compositor -- --backend=wayland --socket=backlit-0 --smoke-test
```

The nested Wayland path can be checked without a visible VM desktop by running Weston's headless backend:

```bash
./scripts/verify-nested-wayland-smoke.sh
```

This starts a temporary parent Weston compositor, verifies the parent socket with `wayland-info` or `weston-info`, then runs Backlit's Wayland backend preflight, compositor smoke path, and `backlit-session --backend=wayland --verify-services --verify-clean-exit` path against that socket.

When the real compositor loop lands, launch clients into Backlit with:

```bash
WAYLAND_DISPLAY=backlit-0 foot
```

## Real Hardware Workflow

Use real Intel/AMD Linux hardware before trusting anything about:

- DRM/KMS.
- libinput behavior.
- GPU buffer paths.
- monitor hotplug.
- suspend/resume.
- frame pacing.
- idle CPU and memory.

The Mac remains a good workstation. The Linux machine is the display lab.

## Useful Commands

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo run -p backlit-compositor -- --backend=headless --smoke-test
cargo run -p backlit-compositor-backend -- --backend=headless --verify
cargo run -p backlit-protocols -- --verify --list
cargo run -p backlit-perf -- --verify
cargo run -p backlit-launcher -- --verify --list --target=terminal --desktop-dir=crates/launcher/fixtures
cargo run -p backlit-launcher -- --verify --target=terminal --spawn-smoke --spawn-program=true --wayland-display=backlit-0
cargo run -p backlit-shortcuts -- --verify --list --resolve=Super+Enter
cargo run -p backlit-input -- --verify
cargo run -p backlit-surface -- --verify
cargo run -p backlit-session-supervisor -- --verify
cargo run -p backlit-clipboard -- --verify
cargo run -p backlit-portal-backend -- --verify
cargo run -p backlit-session -- --backend=headless --screenshot target/backlit-session.ppm --verify --verify-services --verify-clean-exit
./scripts/render-gui-preview.sh
./scripts/render-parallels-gui-preview.sh
./scripts/verify-gui-smoke.sh
./scripts/verify-launch-performance.sh
./scripts/verify-resource-budget.sh
./scripts/verify-portal-security.sh
./scripts/verify-launch-readiness.sh
./scripts/verify-session-launch.sh
./scripts/verify-session-clean-exit.sh
./scripts/verify-drm-session-smoke.sh
./scripts/verify-mvp0-contract.sh
./scripts/verify-ci-contract.sh
./scripts/verify-packaging-contract.sh
./scripts/verify-staged-session-install.sh
./scripts/verify-nested-wayland-smoke.sh
./scripts/verify-linux-e2e.sh
cargo run -p backlit-shell -- --component=all --socket=backlit-0 --verify
```

Current compositor flags:

```text
--backend=headless|wayland|drm
--socket=<name>
--smoke-test
--help
```

## GUI Smoke Verification

MVP 0 includes a deterministic headless GUI harness. It does not replace nested Wayland or real DRM/KMS testing, but it proves that the session launch path can create a visible shell preview and verify expected GUI regions in CI.

To render the current preview for inspection:

```bash
./scripts/render-gui-preview.sh
```

This writes `target/gui-preview/backlit-session.ppm` and, when `sips`, ImageMagick, or netpbm is available, `target/gui-preview/backlit-session.png`.

To render the same preview inside the Ubuntu Parallels guest and copy it back for inspection:

```bash
./scripts/render-parallels-gui-preview.sh
```

This writes host-side artifacts under `target/gui-preview-parallels/`, including `backlit-session.png` on macOS.

CI is defined in `.github/workflows/linux-e2e.yml` and is contract-checked locally:

```bash
./scripts/verify-ci-contract.sh
```

The workflow installs Ubuntu dependencies, Rust `rustfmt` and `clippy`, runs `./scripts/verify-linux-e2e.sh target/linux-e2e-ci`, and uploads the resulting artifacts.

```bash
./scripts/verify-gui-smoke.sh
./scripts/verify-launch-performance.sh
```

Artifacts are written to `target/gui-smoke/`:

- `manifest.json`: top-level verification summary.
- `backlit-session.ppm`: deterministic session screenshot.
- `demo-client.ppm`: deterministic demo client screenshot.
- `*.jsonl`: structured launch and verification events.

The verifier also runs `backlit-protocols --verify --list` so MVP protocol coverage stays explicit while the real Smithay compositor is being brought up.

It also runs `backlit-perf --verify`, which measures the deterministic GUI render path, headless backend present path, idle no-redraw behavior, targeted surface damage path, and 60-frame drag pacing path against generous MVP 0 smoke budgets.

The compositor smoke path also checks the headless direct-scanout policy: an opaque fullscreen dmabuf surface covering the output is eligible, while overlays and SHM buffers block scanout.

The launch-performance verifier runs the built `backlit-session`, `backlit-compositor`, and `backlit-shell` binaries directly, then writes `target/launch-performance/manifest.json`. It enforces the current MVP budgets for session GUI readiness under 500 ms, shell-ready service probes under 2 seconds, and terminal hotkey spawn under 300 ms.

The resource-budget verifier runs bounded idle probes for `backlit-compositor` and `backlit-shell`, samples Linux `/proc`, then writes `target/resource-budget/manifest.json`. On Linux it enforces compositor idle CPU under 0.5% and combined compositor+shell RSS under 250 MB; on non-Linux hosts it records an expected skip so Parallels remains the authoritative resource-budget proof.

Portal security is covered by `backlit-portal-backend --verify`, which denies direct screenshot, screencast, and remote-desktop capture while allowing consented portal-mediated screenshot, screencast, and file-chooser requests.

The default GUI render is guarded by checksum `5635038614353063225`; update it only when an intentional visual change is made.

The launcher catalog is verified in dry-run mode for the first required targets: terminal, browser, and settings.

The launcher also parses visible `.desktop` entries from a fixture directory in smoke tests, so freedesktop app discovery has a regression path before a full app indexer exists.

Launcher spawn verification proves the selected target can start a process with `WAYLAND_DISPLAY` set. In nested Wayland E2E this uses the parent Weston socket and the available Wayland info client as the spawned command.

Keyboard shortcut routing is also verified in dry-run mode for launcher, terminal, browser, settings, and app-switcher actions.

Input routing is verified by `backlit-input --verify`, which feeds deterministic keyboard and pointer events into the same policy layer the compositor will use for libinput events. It proves that `Super+Enter` routes to terminal launch, `Alt+Tab` changes focus, title-bar drags move windows, resize-grip drags resize windows, and pointer grabs end cleanly.

Surface lifecycle is verified by `backlit-surface --verify`, which proves the xdg-shell-style path from toplevel creation through initial configure/ack/commit, focus, maximize, fullscreen, close request, and clean window removal.

The session smoke path consumes those routes too: `Alt+Tab` cycles focus and `Super+Enter` resolves the terminal launch path, pointer input verifies focus/move/resize routing, surface lifecycle verifies map/configure/close behavior, spawns the terminal launch target with `WAYLAND_DISPLAY` set when `--verify-launch-spawn` is enabled, then records the resulting window-policy state in `session.jsonl`.

With `--verify-services`, `backlit-session` also resolves sibling `backlit-compositor` and `backlit-shell` binaries, runs their readiness probes, captures their logs, and emits `session.services_verified`.

With `--verify-clean-exit`, `backlit-session` also requests shutdown, closes all managed windows, clears focus, and emits `session.clean_exit`.

Session verification also checks output-aware geometry: maximized windows use the panel-reserved work area, while fullscreen uses the whole output.

Move and resize behavior is also verified through the session smoke path before maximize/fullscreen checks run.

Minimized windows are kept in policy state but skipped by focus cycling; this is verified in the session smoke path.

Closing the focused window is verified too, including fallback focus that skips minimized windows.

Crash isolation is covered by `backlit-session-supervisor --verify`: shell crashes restart without killing the compositor, while compositor crashes end the session.

Clipboard state is covered by `backlit-clipboard --verify`, which checks text ownership, replacement, clearing, and generation tracking.

Backend preflight can be run directly:

```bash
cargo run -p backlit-compositor-backend -- --backend=headless --verify
cargo run -p backlit-compositor-backend -- --backend=wayland --verify
cargo run -p backlit-compositor-backend -- --backend=drm --verify
```

The Wayland preflight expects `WAYLAND_DISPLAY` and `XDG_RUNTIME_DIR`; the DRM preflight expects Linux, `XDG_RUNTIME_DIR`, `/dev/dri` nodes, `/dev/input/event*` devices, and `XDG_SESSION_ID` so the real backend can later ask logind/libseat for device access.

To capture the current host's launch readiness:

```bash
./scripts/verify-launch-readiness.sh
./scripts/verify-session-launch.sh
./scripts/verify-session-clean-exit.sh
./scripts/verify-drm-session-smoke.sh
```

These write `target/launch-readiness/manifest.json`, `target/session-launch/manifest.json`, `target/session-clean-exit/manifest.json`, and `target/drm-session-smoke/manifest.json`. On macOS or headless CI they can pass with DRM expected-blocked; inside the Parallels Ubuntu GUI VM they should report DRM expected-ready and ready. The session launch verifier also checks that `packaging/sessions/backlit.desktop` resolves to `backlit-session` and that `backlit-session --preflight-only` exits cleanly for launchable backends. The session clean-exit verifier checks requested shutdown from the headless session path. The DRM session smoke verifier runs the full `backlit-session --backend=drm` path with GUI verification, terminal spawn verification, compositor/shell service probes, and clean shutdown when the host is launch-ready.

## Packaging Contract Verification

The packaging contract verifier checks that the session desktop entry, systemd units, Debian package split, and workspace binary names agree:

```bash
./scripts/verify-packaging-contract.sh
```

It writes `target/packaging-contract/manifest.json` by default.

## Staged Session Install Verification

The staged install verifier builds the session, compositor, and shell binaries, lays them out under a fake `/usr`, installs the session desktop entry and user systemd units, and verifies that all launch commands resolve to staged executables:

```bash
./scripts/verify-staged-session-install.sh
```

It then launches the staged `backlit-session` with the headless backend, `--verify`, and `--verify-services`, checks the deterministic GUI output plus compositor/shell startup probes, and writes `target/staged-session-install/manifest.json`.

## Engineering Rules

- Keep compositor policy small, measurable, and hard to crash.
- Keep shell clients separate from compositor state where practical.
- Put focus, placement, workspace, and snapping behavior in `backlit-window-policy` so it can be tested without a GPU.
- Keep the headless backend path testable without Wayland so CI can prove client/surface/damage behavior before DRM/KMS work lands.
- Emit structured JSON metrics for startup, backend selection, smoke tests, and future frame timing.
- No feature that touches startup, input, rendering, app launch, or session services should merge without a benchmark or regression check.
- Avoid animations and plugin systems until frame pacing is excellent.
- Prefer standards: Wayland protocols, freedesktop specs, systemd/logind, Mesa, libinput, portals, Debian packaging.

## Git Remote

This workspace is initialized with:

```bash
git remote add origin https://github.com/juncoflockleader/backlit.git
```

Use `main` as the default branch.
