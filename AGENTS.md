# Xfce Profile Editor Guidance

This project values explicit boundaries, readable Rust, and a small dependency
surface. Read `IMPLEMENTATION_PLAN.md` before making architectural or
scope-changing decisions.

## Required skill

Use `.agents/skills/ui-core-boundary/SKILL.md` whenever a change adds or
substantially modifies a GTK window, UI flow, system service, persistence path,
image/camera pipeline, or feature that crosses `src/ui/` and `src/core/`.

## Component boundaries

- Layout belongs in `ui/**/*.ui`. This includes widget hierarchy, spacing,
  labels, static menus, accessibility relationships, and ordinary widget
  properties. Do not recreate template-owned widgets in Rust.
- Styling belongs in `ui/sass/`. Rust may add or remove meaningful state
  classes but must not construct presentation-specific CSS.
- `src/ui/` contains GTK plumbing only: template subclasses and children,
  signal/action connections, window lifecycle, presentation state, and
  conversion of core results into GTK-facing values.
- `src/core/` owns models, validation, GECOS handling, AccountsService access,
  privileged-helper communication, image processing, camera capture, and
  persistence orchestration.
- Dependency direction is `src/ui/ -> src/core/`. Core code must not import
  `gtk4`, `gdk4`, or UI modules, and public core APIs must use plain Rust data
  and typed errors.
- Keep the privileged helper independent of GTK and narrowly limited to the
  current caller's supported GECOS fields.

If a responsibility does not fit these boundaries, stop and make the ownership
decision explicit before implementing it. Do not hide cross-layer behaviour in
generic `helpers` or `utils` modules.

## Licensing

- The project is licensed as `GPL-2.0-or-later`; do not add code or assets with
  incompatible terms.
- Add `SPDX-License-Identifier: GPL-2.0-or-later` to new source, build, UI, and
  metadata files using the comment syntax appropriate to that format.
- Preserve third-party copyright and licence notices, and document vendored or
  adapted material explicitly.

## UI integration

- Use GTK 4.10-or-newer APIs and resource-backed composite templates.
- Keep the application usable on small screens, keyboard accessible, and
  responsive while D-Bus, image, camera, or privileged work is running.
- Never perform blocking system or image work on the GTK main thread.
- Keep static fallback content and layout in `.ui`; use Rust only where the
  state is genuinely dynamic.
- Every new or renamed UI/SCSS resource must be reflected in the GResource and
  build inputs and covered by a resource-load check.

## System data and security

- Do not edit `/etc/passwd` from the UI process, invoke a shell for account
  changes, collect passwords, or accept an arbitrary target user.
- Preserve all unexposed GECOS fields exactly.
- Treat AccountsService, Polkit, camera, filesystem, and image failures as
  typed recoverable errors; do not silently report partial success as complete.
- Tests for account changes must use disposable fixtures and must never alter a
  developer's real profile.

## Verification

Run formatting, Clippy with warnings denied, and the relevant tests for every
change. UI work additionally requires resource loading and exercising the
affected runtime flow with a display or supported headless backend. Before
completion, confirm that no GTK/GDK imports or presentation types have entered
`src/core/` and review the full diff for missing resources, tests, or packaging
changes.

GitHub Actions for the private repository must remain cost-conscious: keep one
standard Linux validation job, cancel superseded runs, avoid scheduled or
matrix jobs, and retain temporary artifacts for no more than seven days unless
the implementation plan is deliberately revised. Workflows use read-only token
permissions by default; only the tag-triggered release job may request
`contents: write`. Release automation creates a draft for manual verification
and must never publish a release directly.
