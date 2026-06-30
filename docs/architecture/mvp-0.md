# MVP 0 Architecture

MVP 0 is the development harness:

- Cargo workspace.
- Headless compositor entrypoint and backend state model.
- Backend preflight checks for headless, nested Wayland, and DRM launch paths.
- Nested Wayland backend flag.
- Pure window-policy logic.
- MVP shell role smoke checks.
- Dry-run launcher catalog for terminal, browser, and settings.
- Deterministic `.desktop` app discovery smoke for launcher.
- Keyboard shortcut routing for launcher, app switcher, and core launch targets.
- Session interaction smoke for app switching and terminal launch resolution.
- Session move/resize smoke through pure window policy.
- Minimized-window focus skipping in session smoke.
- Focus fallback after closing a window.
- Output work-area policy for panel-aware maximize and fullscreen geometry.
- Session supervisor crash isolation smoke.
- Clipboard state smoke for text owner, replacement, and clearing.
- Deterministic demo GUI renderer.
- Headless session launch verification.
- Basic MVP protocol smoke registry.
- Headless performance smoke checks.
- JSON metrics.
- CI.
- Packaging skeleton.

The real compositor will integrate Smithay behind `backlit-compositor` and keep policy logic testable in `backlit-window-policy`.
