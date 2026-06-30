# MVP 0 Architecture

MVP 0 is the development harness:

- Cargo workspace.
- Headless compositor entrypoint and backend state model.
- Backend preflight checks for headless, nested Wayland, and DRM launch paths.
- Nested Wayland backend flag and Weston-backed compositor/session smoke verifier with clean shutdown.
- Pure window-policy logic.
- MVP shell chrome smoke for wallpaper, panel status indicators, panel power menu, NetworkManager/PipeWire status and control plans, workspace indicator, launcher targets, app switcher entries, and lock-screen readiness.
- Dry-run launcher catalog for terminal, browser, and settings.
- Deterministic `.desktop` app discovery smoke for launcher.
- Launcher process-spawn smoke with Wayland display environment propagation.
- Keyboard shortcut routing for launcher, app switcher, and core launch targets.
- Input event routing smoke for keyboard shortcuts, pointer focus, window move, and resize.
- xdg-shell-style toplevel lifecycle smoke for configure/ack/map, focus, maximize, fullscreen, and close.
- Session interaction smoke for app switching and terminal launch resolution.
- Session service orchestration smoke for launching compositor, shell, and settings daemon probes from `backlit-session`.
- Session move/resize smoke through pure window policy.
- Workspace switching smoke that hides non-active workspace windows from focus and pointer routing.
- Window snap smoke for left/right work-area tiling.
- Minimized-window focus skipping in session smoke.
- Focus fallback after closing a window.
- Clean session shutdown smoke for closing managed windows and clearing focus across headless, nested Wayland, and launch-ready DRM paths.
- Output work-area policy for panel-aware maximize and fullscreen geometry.
- Session supervisor crash isolation and journal-addressable crash-log smoke.
- Clipboard state smoke for text owner, replacement, and clearing.
- Notification daemon smoke for D-Bus-style notify, replace, actions, persistence, and close reasons.
- Settings app smoke for display, input, and power panels backed by the settings daemon policy.
- Settings daemon smoke for display, input, power policy validation, and dry-run logind/systemd power action plans.
- Portal security smoke for denied direct capture and consented portal-mediated flows.
- Deterministic demo GUI renderer.
- Headless session launch verification.
- Backlit user systemd target and dry-run session launch plan for starting/stopping runtime services.
- Viewable GUI preview renderer with verified session services.
- Parallels GUI preview export from Ubuntu guest artifacts to the host workspace.
- Basic MVP protocol smoke registry.
- Headless performance smoke checks.
- Headless frame damage smoke for no idle redraws and targeted surface damage.
- Headless drag-frame pacing smoke for dropped-frame and pointer-to-frame latency budgets.
- Headless direct-scanout eligibility smoke for fullscreen dmabuf surfaces and overlay/SHM blockers.
- Launch performance budget verifier for session GUI readiness, shell readiness, and terminal hotkey spawn.
- Linux resource budget verifier for bounded idle CPU and compositor+shell RSS probes.
- JSON metrics.
- Linux E2E verifier for fmt, tests, clippy, GUI smoke, launch performance, resource budgets, settings daemon policy, packaging contract, session clean exit, and nested Wayland smoke inside an Ubuntu guest.
- GitHub Actions Linux E2E workflow with artifact upload and local contract verification.
- MVP 0 contract verifier that checks the executable harness still covers the design deliverables and, inside E2E, validates the generated artifact manifests.
- Parallels runner for repeatable macOS-to-Ubuntu guest verification with the active guest user's logind session and runtime directory.
- Packaging contract verifier for session desktop entry, Backlit user target, systemd units, and Debian package split.
- Staged session install verifier for fake `/usr` layout, systemd launch-plan resolution, launch command resolution, and headless GUI verification from staged binaries.
- CI.
- Packaging skeleton.

The real compositor will integrate Smithay behind `backlit-compositor` and keep policy logic testable in `backlit-window-policy`.
