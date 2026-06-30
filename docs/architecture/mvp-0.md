# MVP 0 Architecture

MVP 0 is the development harness:

- Cargo workspace.
- Headless compositor entrypoint and backend state model.
- Backend preflight checks for headless, nested Wayland, and DRM launch paths.
- Nested Wayland backend flag.
- Pure window-policy logic.
- MVP shell role smoke checks.
- Deterministic demo GUI renderer.
- Headless session launch verification.
- Basic MVP protocol smoke registry.
- Headless performance smoke checks.
- JSON metrics.
- CI.
- Packaging skeleton.

The real compositor will integrate Smithay behind `backlit-compositor` and keep policy logic testable in `backlit-window-policy`.
