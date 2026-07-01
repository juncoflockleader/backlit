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
  session-supervisor/  Crash isolation and crash-log smoke checks.
  shell-protocol/      Private shell/compositor protocol model.
  shell/               Shell chrome state and host smoke checks.
  notification-daemon/ D-Bus notification behavior smoke checks.
  settings/            Minimal settings app surface smoke checks.
  settings-daemon/     Display, input, and power policy daemon smoke checks.
  portal-backend/      Future xdg-desktop-portal backend.
apps/
  settings/            Future richer settings UI assets.
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

The E2E runner verifies the full Ubuntu path, runs an opt-in root system-install smoke inside the disposable guest, then copies a compact evidence bundle back to `target/linux-e2e-parallels/`: the guest E2E manifest, GUI smoke/preview manifests, launch-readiness and DRM session-smoke manifests, Debian package build/install/system-install manifests, installed-package replay manifests, nested Wayland manifest, MVP 0/MVP 1 contract manifests, and the E2E GUI preview image. The package-install and system-install checks also run the scripted session replay from the installed `backlit-session` binary, so launcher and app-switcher overlay frames are proven after packaging; they also prove compositor socket clients and the installed Settings desktop entry map into managed policy with app id and focus preserved. The preview runner renders the Backlit preview inside Ubuntu, copies the generated PPM/session logs/manifest back to `target/gui-preview-parallels/`, and converts the PPM to a local PNG on macOS when possible.

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
./scripts/verify-parallels-linux-e2e.sh target/linux-e2e-parallels
```

The Linux-side verifier can also be run directly inside any Ubuntu checkout:

```bash
./scripts/verify-linux-e2e.sh
```

It runs `cargo fmt`, workspace tests, `cargo clippy`, the deterministic GUI smoke verifier, the preview renderer, compositor-runtime verifier, compositor-socket verifier, Smithay compositor runtime verifier, launch-performance verifier, launcher desktop discovery verifier, resource-budget verifier, notification-daemon verifier, settings-daemon verifier, service-lifecycle verifier, settings-app verifier, portal-security verifier, crash-log verifier, CI contract verifier, packaging contract verifier, package-manifest verifier, Debian package-build verifier, Debian package-install verifier, Debian system-install verifier, staged session install verifier, systemd activation verifier, Smithay runtime probe, launch-readiness verifier, session launch verifier, session clean-exit verifier, nested Wayland smoke verifier, MVP 1 contract verifier, and MVP 0 contract verifier, then writes `target/linux-e2e/manifest.json`.

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

This starts a temporary parent Weston compositor, verifies the parent socket with `wayland-info` or `weston-info`, launches the real terminal target (`foot`) against that socket with a short-lived command, then runs Backlit's Wayland backend preflight, compositor smoke path, and `backlit-session --backend=wayland --verify-services --verify-clean-exit` path. Weston headless does not expose a usable input seat in this setup, so the terminal check accepts `foot`'s known no-seat exit code while still requiring that the real terminal process was spawned with `WAYLAND_DISPLAY`.

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
cargo run -p backlit-notification-daemon -- --verify
cargo run -p backlit-settings-daemon -- --verify
cargo run -p backlit-settings -- --verify
cargo run -p backlit-portal-backend -- --verify
cargo run -p backlit-session -- --backend=headless --screenshot target/backlit-session.ppm --verify --verify-services --verify-clean-exit
./scripts/render-gui-preview.sh
./scripts/render-parallels-gui-preview.sh
./scripts/verify-gui-smoke.sh
./scripts/verify-compositor-runtime.sh
./scripts/verify-compositor-socket.sh
./scripts/verify-launch-performance.sh
./scripts/verify-resource-budget.sh
./scripts/verify-notification-daemon.sh
./scripts/verify-settings-daemon.sh
./scripts/verify-service-lifecycle.sh
./scripts/verify-settings-app.sh
./scripts/verify-portal-security.sh
./scripts/verify-crash-logs.sh
./scripts/verify-smithay-runtime-probe.sh
./scripts/verify-smithay-compositor-runtime.sh
./scripts/verify-launch-readiness.sh
./scripts/verify-session-launch.sh
./scripts/verify-session-replay.sh
./scripts/verify-session-clean-exit.sh
./scripts/verify-drm-session-smoke.sh
./scripts/verify-mvp0-contract.sh
./scripts/verify-ci-contract.sh
./scripts/verify-packaging-contract.sh
./scripts/verify-package-manifests.sh
./scripts/verify-debian-package-build.sh
./scripts/verify-debian-package-install.sh
./scripts/verify-debian-system-install.sh
./scripts/verify-staged-session-install.sh
./scripts/verify-systemd-activation.sh
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

On macOS, open the local preview with:

```bash
open target/gui-preview/backlit-session.png
```

To render the same preview inside the Ubuntu Parallels guest and copy it back for inspection:

```bash
./scripts/render-parallels-gui-preview.sh
```

This writes host-side artifacts under `target/gui-preview-parallels/`, including `backlit-session.png` on macOS:

```bash
open target/gui-preview-parallels/backlit-session.png
```

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

The shell smoke path runs `backlit-shell --component=all --verify` and checks the MVP chrome state: wallpaper, panel clock/battery indicators, power menu actions, NetworkManager-backed network status and control command plans, PipeWire-backed audio status and volume command plans, workspace indicator, launcher targets, app switcher entries, and lock-screen readiness.

It also runs `backlit-perf --verify`, which measures the deterministic GUI render path, headless backend present path, idle no-redraw behavior, targeted surface damage path, and 60-frame drag pacing path against generous MVP 0 smoke budgets.

The compositor smoke path also checks the headless direct-scanout policy: an opaque fullscreen dmabuf surface covering the output is eligible, while overlays and SHM buffers block scanout.

The compositor-runtime verifier runs `backlit-compositor --scripted-client --scripted-client-preview target/compositor-runtime/scripted-client-policy-preview.ppm --serve --serve-for-ms=25`, maps two app-like surfaces through the `CompositorRuntime` backend trait, verifies they become managed policy windows, writes and verifies a GUI preview frame from that compositor path, verifies targeted damage and idle no-redraw behavior, closes one surface, disconnects the client, and checks the cleanup frames. The manifest records the baseline runtime backend as `headless-compositor`. The Smithay compositor runtime verifier runs `backlit-compositor --backend=drm --runtime=smithay --scripted-client`, `backlit-compositor --backend=drm --runtime=smithay --smithay-client-smoke`, and a bounded `--runtime=smithay --serve` service socket with `--features smithay-backend`; on launch-ready Linux it requires the scripted lifecycle, a generated `wayland-client` registry/bind/surface/xdg-toplevel configure/ack/two-commit lifecycle with server-side xdg title and app-id metadata callbacks, a real 320x240 `wl_shm` buffer attach observed through Smithay buffer dimensions, a Backlit policy window mapped from that observed title/app id and buffer geometry, service-ready bootstrap, and demo-client socket lifecycle to report `runtime_backend: "smithay-compositor-runtime"` while proving that the Smithay runtime registers `wl_compositor`, `wl_subcompositor`, `wl_shm`, and `xdg_wm_base`, inserts Wayland clients, dispatches the Wayland display, and ticks its calloop event loop on the same scripted and service-socket presents. The compositor-socket verifier and session service probe additionally preserve demo-client app ids from socket announcements into managed window policy. The socket verifier also sends a multi-client management and lifecycle sequence through the service boundary: announce a first surface, announce and focus a second surface, move it, resize it, maximize it into the work area, fullscreen it across the output, damage the focused surface, close it, remove its policy window, fall focus back to the first window, and disconnect the closed client. It is a bounded proof for the app-window lifecycle and geometry contract that the future full Smithay protocol/runtime loop must preserve.

The compositor-socket verifier runs the compositor in bounded service mode with a private `XDG_RUNTIME_DIR`, requires the configured socket name to appear as a Unix socket, connects one `backlit-demo-client` as a persistent terminal-like window, then connects another with `--connect-management --connect-lifecycle`. It verifies both windows map into policy, the newer app receives focus, socket commands move and resize it, maximize uses the panel-reserved work area, fullscreen uses the whole output, the damage event presents exactly one damaged surface, close removes the newer backend surface and managed policy window, focus returns to the older app window, waits for clean exit, and verifies socket cleanup. This is the current executable contract behind `--socket=backlit-0` in the packaged user service.

The launch-performance verifier runs the built `backlit-session`, `backlit-compositor`, `backlit-shell`, `backlit-notification-daemon`, and `backlit-settings-daemon` binaries directly, then writes `target/launch-performance/manifest.json`. It enforces the current MVP budgets for session GUI readiness under 500 ms, service-ready probes under 2 seconds, and terminal hotkey spawn under 300 ms.

The resource-budget verifier runs bounded idle probes for `backlit-compositor` and `backlit-shell`, samples Linux `/proc`, then writes `target/resource-budget/manifest.json`. On Linux it verifies the non-smoke compositor service readiness path by accepting a bootstrap client and presenting a bootstrap surface, then enforces compositor idle CPU under 0.5% and combined compositor+shell RSS under 250 MB; on non-Linux hosts it records an expected skip so Parallels remains the authoritative resource-budget proof.

The service-lifecycle verifier runs the compositor, shell, notification daemon, and settings daemon with `--serve --serve-for-ms=25`. The packaged systemd units use unbounded `--serve`, while the verifier proves the same code path can enter service mode and exit cleanly when bounded for automation.

Settings daemon state is covered by `backlit-settings-daemon --verify`, which validates display mode/scale/refresh settings, keyboard and pointer policy, power idle policy, the lock/logout/reboot/shutdown power menu, and dry-run command plans for lock, logout, suspend, reboot, and shutdown through logind/systemd.

Settings app surface state is covered by `backlit-settings --verify`, which checks that the launcher `settings` target resolves to an installed `backlit-settings` binary and that display, input, and power panels can apply valid settings through the settings daemon policy.

Notification daemon state is covered by `backlit-notification-daemon --verify`, which verifies D-Bus-style notification fields, replace-id behavior, action invocation, critical notification persistence, and expired/dismissed/replaced close reasons.

Portal security is covered by `backlit-portal-backend --verify`, which denies direct screenshot, screencast, and remote-desktop capture while allowing consented portal-mediated screenshot, screencast, and file-chooser requests.

Crash logging is covered by `backlit-session-supervisor --verify` and `./scripts/verify-crash-logs.sh`. The supervisor emits structured crash-log records for shell and compositor failures, and packaged user services explicitly send stdout/stderr to the systemd journal with stable `SyslogIdentifier` values and `RUST_BACKTRACE=1`.

The static demo render is guarded by checksum `5635038614353063225`. The session preview render is policy-driven and guarded by checksum `15888844850457870477`; update either only when an intentional visual change is made.

The launcher catalog is verified in dry-run mode for the first required targets: terminal, browser, and settings. The launcher also parses visible freedesktop `.desktop` entries, including quoted `Exec=` arguments and field-code removal, then spawns a discovered fixture entry with `WAYLAND_DISPLAY` set so app-launch plumbing is executable before the real compositor loop lands.

The launcher discovers visible `.desktop` entries from XDG application directories by default. Smoke tests keep a fixture directory for deterministic parser coverage, while `verify-launcher-desktop-discovery.sh` also checks host app discovery and requires visible entries on Linux hosts that actually have desktop files installed.

Launcher spawn verification proves the selected target can start a process with `WAYLAND_DISPLAY` set. In nested Wayland E2E this uses the parent Weston socket for both the available Wayland info client and the real terminal target (`foot`) with a short-lived command; the terminal artifact records whether Weston headless hit the expected no-seat exit.

Keyboard shortcut routing is also verified in dry-run mode for launcher, terminal, browser, settings, and app-switcher actions.

Input routing is verified by `backlit-input --verify`, which feeds deterministic keyboard and pointer events into the same policy layer the compositor will use for libinput events. It proves that `Super+Enter` routes to terminal launch, `Alt+Tab` changes focus, title-bar drags move windows, resize-grip drags resize windows, and pointer grabs end cleanly.

Surface lifecycle is verified by `backlit-surface --verify`, which proves the xdg-shell-style path from toplevel creation through initial configure/ack/commit, focus, popup mapping, maximize, fullscreen, close request, and clean window removal. `backlit-compositor -- --smoke-test` also drives that xdg toplevel and popup path through the compositor smoke by mapping configured surfaces into the headless backend frame before maximizing, fullscreening, and closing them.

The session smoke path consumes those routes too: `Alt+Tab` cycles focus and `Super+Enter` resolves the terminal launch path, pointer input verifies focus/move/resize routing, surface lifecycle verifies map/configure/close behavior, spawns the terminal launch target with `WAYLAND_DISPLAY` set when `--verify-launch-spawn` is enabled, launches the demo client into the compositor service socket with app id preserved, resolves and spawns a discovered `.desktop` entry with the same display when `--verify-desktop-launch` is enabled, maps that entry into managed focused window policy with app id preserved, renders the preview from `WindowPolicy`, then records the resulting window-policy state in `session.jsonl`. Installed-package checks use the packaged Settings desktop entry for that session-level desktop launch probe.

`./scripts/verify-session-replay.sh` runs `backlit-session --scripted-replay-dir` and writes nine deterministic frames for initial focus, app switcher overlay, launcher overlay, terminal launch, window move, window resize, snap, hidden workspace, and switched workspace states.

With `--verify-services`, `backlit-session` also resolves sibling `backlit-compositor`, `backlit-demo-client`, `backlit-shell`, `backlit-notification-daemon`, and `backlit-settings-daemon` binaries. It starts the compositor in bounded service mode, launches the demo client into that compositor socket when the host permits Unix sockets, runs the remaining service readiness probes, captures their logs, and emits `session.services_verified`.

With `--verify-clean-exit`, `backlit-session` also requests shutdown, closes all managed windows, clears focus, and emits `session.clean_exit`.

Session verification also checks output-aware geometry: maximized windows use the panel-reserved work area, while fullscreen uses the whole output.

Move and resize behavior is also verified through the session smoke path before maximize/fullscreen checks run.

Workspace switching and left/right window snapping are verified through the session smoke path too: a moved window is hidden from the active workspace until switching to its workspace, and snapped windows occupy the expected work-area halves.

Minimized windows are kept in policy state but skipped by focus cycling; this is verified in the session smoke path.

Closing the focused window is verified too, including fallback focus that skips minimized windows.

Crash isolation is covered by `backlit-session-supervisor --verify`: shell crashes restart without killing the compositor, compositor crashes end the session, and both paths emit journal-addressable crash records.

Clipboard state is covered by `backlit-clipboard --verify`, which checks text ownership, replacement, clearing, and generation tracking.

Backend preflight can be run directly:

```bash
cargo run -p backlit-compositor-backend -- --backend=headless --verify
cargo run -p backlit-compositor-backend -- --backend=wayland --verify
cargo run -p backlit-compositor-backend -- --backend=drm --verify
```

The Wayland preflight expects `WAYLAND_DISPLAY` plus an `XDG_RUNTIME_DIR` owned by the launching user. The DRM preflight expects Linux, a user-owned `XDG_RUNTIME_DIR`, read/write access to at least one `/dev/dri/card*` node, `/dev/input/event*` devices, and an active local logind session with a concrete seat and session type. Input is considered ready when event nodes are directly readable or when loginctl, libseat, and libinput are available for brokered access.

To capture the current host's launch readiness:

```bash
./scripts/verify-launch-readiness.sh
./scripts/verify-session-launch.sh
./scripts/verify-session-replay.sh
./scripts/verify-session-clean-exit.sh
./scripts/verify-drm-session-smoke.sh
./scripts/verify-systemd-activation.sh
./scripts/verify-mvp1-contract.sh
```

These write `target/smithay-runtime-probe/manifest.json`, `target/smithay-compositor-runtime/manifest.json`, `target/launch-readiness/manifest.json`, `target/session-launch/manifest.json`, `target/session-replay/manifest.json`, `target/session-clean-exit/manifest.json`, `target/drm-session-smoke/manifest.json`, `target/systemd-activation/manifest.json`, and `target/mvp1-contract/manifest.json`. On macOS or headless CI they can pass with DRM expected-blocked; inside the Parallels Ubuntu GUI VM the wrapper maps the active `parallels` logind session before running E2E, so the manifests should report a user-owned runtime, active local session, accessible DRM card, `input_broker_ready: true`, DRM expected-ready, and ready. Direct `/dev/input/event*` access is recorded separately; when it is unavailable, the manifest should report `input_broker_mode: "logind-libseat"`. The Smithay runtime probe compiles `backlit-compositor-backend` with `--features smithay-backend`, verifies Smithay DRM/GBM/EGL/GLES/libinput/libseat/calloop components are linkable, resolves the selected DRM card into a Smithay `DrmNode`, opens that primary DRM card through `DrmDevice`, enumerates KMS CRTCs, connectors, connected modes, and planes, selects a connected connector plus mode/CRTC/primary-plane scanout target tuple, inserts the DRM notifier into calloop, dispatches that event loop once, opens the matching render node, creates a Smithay GBM device, GBM allocator, EGL display, EGL context, and GLES renderer from that node, renders a 16x16 offscreen GLES frame, copies it back to CPU memory, verifies a red RGBA sample, creates a Smithay libseat session for the active seat, creates a libinput udev context through that session, assigns it to the seat, inserts both the libseat notifier and libinput backend into calloop, dispatches that event loop once, creates a Smithay Wayland display, binds a listening socket, connects/accepts/inserts a local client, dispatches/flushes clients, runs one calloop dispatch, and requires `smithay_runtime_probe: true` plus `smithay_runtime_bootstrap: true` on launch-ready Linux. The Smithay compositor runtime verifier compiles `backlit-compositor` with the same feature and requires `backlit-compositor --backend=drm --runtime=smithay --scripted-client`, `backlit-compositor --backend=drm --runtime=smithay --smithay-client-smoke`, plus a bounded `--runtime=smithay --serve` demo-client socket lifecycle to pass on launch-ready Linux, including `smithay_core_protocol_globals: true`, `smithay_real_wayland_client: true`, `smithay_real_wayland_metadata: true`, `smithay_real_shm_buffer: true`, `smithay_real_wayland_policy_window: true`, `smithay_event_loop_runtime: true`, and `smithay_event_loop_service_socket: true` evidence from the runtime's Wayland display, core protocol globals, xdg-toplevel client lifecycle, xdg title/app-id callbacks, real wl_shm buffer commit dimensions, Backlit policy-window mapping, and calloop dispatch counters. `backlit-compositor-backend`, `backlit-compositor`, and `backlit-session` also emit backend launch-plan events after preflight. For DRM these events intentionally report `implementation: "pre-smithay-policy-harness"` until the real loop lands, while requiring `display_driver: "drm-kms"`, a direct-libinput or logind-libseat-libinput input driver, and selected DRM/input device evidence on launch-ready hosts. The session launch verifier also checks that `packaging/sessions/backlit.desktop` resolves to `backlit-session --backend=drm --activate-systemd`, that `backlit-session --preflight-only` exits cleanly for launchable backends, and that `backlit-session --verify-systemd-units` accepts the `backlit-session.target` user target plus the dry-run `systemctl --user import-environment/start/stop` launch plan for compositor, shell, notification, and settings services. The packaged service units run those services with unbounded `--serve`; bounded `--serve-for-ms` is reserved for automation. The import plan must carry `XDG_RUNTIME_DIR`, `XDG_SESSION_ID`, `XDG_SEAT`, `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `XDG_CURRENT_DESKTOP`, and `DESKTOP_SESSION` into the user manager so DRM/logind preflight sees the same launch context as the session entrypoint. The systemd activation verifier executes that import/start/stop sequence through a fake `systemctl` without mutating the host user manager. The session replay verifier captures deterministic frames for visible focus/launch/move/resize/snap/workspace state changes. The session clean-exit verifier checks requested shutdown from the headless session path. The DRM session smoke verifier runs the full `backlit-session --backend=drm` path with GUI verification, terminal spawn verification, compositor/shell/settings service probes, demo-client launch through the session-started compositor socket, launch-plan verification, and clean shutdown when the host is launch-ready. The MVP 1 contract verifier gathers this evidence into one readiness gate and stays explicit that the real compositor loop is still being brought up.

## Packaging Contract Verification

The packaging contract verifier checks that the session desktop entry, Backlit systemd target, systemd units, Debian package split, and workspace binary names agree:

```bash
./scripts/verify-packaging-contract.sh
```

It writes `target/packaging-contract/manifest.json` by default.

## Staged Session Install Verification

The Debian package-build verifier assembles the `fastgui-*` package roots from `packaging/debian/*.install`, builds `.deb` artifacts with `dpkg-deb` on Linux, and inspects `fastgui-core`, runtime package contents, and package dependencies. The Debian package-install verifier installs the `fastgui-core` dependency closure into a disposable `dpkg --root` tree, checks dpkg status, then runs `backlit-session --backend=headless --verify --verify-services --verify-clean-exit` plus scripted replay from the installed `/usr/bin` tree. On non-Debian hosts both write expected-blocked manifests so the same E2E script remains usable from macOS.

The Debian system-install verifier is intentionally guarded: it only mutates the host when run as root with `BACKLIT_ALLOW_SYSTEM_PACKAGE_INSTALL=1`. In that mode it installs the freshly built `fastgui-core` closure into the real dpkg database, verifies `/usr/bin/backlit-session`, user systemd units, settings, services, GUI launch, and clean exit, then purges the packages before writing its manifest. Without root or the explicit environment variable it writes an expected-blocked manifest.

The staged install verifier builds the session, compositor, shell, and settings daemon binaries, lays them out under a fake `/usr`, installs the session desktop entry, `backlit-session.target`, and user systemd units, and verifies that all launch commands resolve to staged executables. The systemd activation verifier separately proves the session launcher can execute the target import/start/stop command sequence:

```bash
./scripts/verify-staged-session-install.sh
./scripts/verify-systemd-activation.sh
```

It then launches the staged `backlit-session` with the headless backend, `--verify`, and `--verify-services`, checks the deterministic GUI output plus compositor/shell/settings startup probes, and writes `target/staged-session-install/manifest.json`.

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
