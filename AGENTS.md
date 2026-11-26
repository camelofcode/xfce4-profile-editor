# Xfce Profile Editor Guidance

This project values explicit boundaries, readable Rust, and a small dependency
surface. The README describes the released features.

## Required skill

Use `.agents/skills/idiomatic-rust/SKILL.md` whenever a change adds, removes,
or substantially reshapes Rust types, modules, traits, service interfaces, or
error handling.

Use `.agents/skills/ui-core-boundary/SKILL.md` whenever a change adds or
substantially modifies a GTK window, UI flow, system service, persistence path,
image/camera pipeline, or feature that crosses `src/ui/` and `src/core/`.

Use `.agents/skills/resource-aware-builds/SKILL.md` before the first
compile-heavy Cargo or Meson command in a session. Its detected mode controls
resource scheduling, not which required checks are eventually run.

## Component boundaries

- Layout belongs in `ui/**/*.ui`. This includes widget hierarchy, spacing,
  labels, static menus, accessibility relationships, and ordinary widget
  properties. Do not recreate template-owned widgets in Rust.
- Styling belongs in `ui/sass/`. Rust may add or remove meaningful state
  classes but must not construct presentation-specific CSS.
- `src/ui/` contains GTK plumbing only: template subclasses and children,
  signal/action connections, window lifecycle, presentation state, and
  conversion of core results into GTK-facing values.
- `src/core/` owns models, validation, AccountsService access, current-user
  lookup, image processing, camera capture, and persistence orchestration.
- Dependency direction is `src/ui/ -> src/core/`. Core code must not import
  `gtk4`, `gdk4`, or UI modules, and public core APIs must use plain Rust data
  and typed errors.
- Display-name writes use AccountsService `SetRealName`; profile-image writes
  keep `~/.face` and AccountsService `IconFile` in sync. Do not add a direct
  `/etc/passwd` writer, project-owned privileged helper, or project Polkit
  policy without an explicit change to the project's scope.
- Installation-layout checks must use a non-empty `DESTDIR` inside `target/`
  or `/tmp`; never run an unstaged install or write to `/usr` during automated
  verification.
- Meson is the supported build/install entry point and drives Cargo. Do not add
  a parallel Makefile install path; keep Cargo workspace commands for Rust
  formatting, linting, and tests.
- Application builds, staging, installation, and launches go through
  `scripts/build-install-launch.sh`. Direct Cargo remains appropriate only for
  workspace formatting, linting, and tests.
- Direct Cargo and Meson builds share the default Cargo home and the workspace
  `target/` cache. Never override `CARGO_HOME` or `CARGO_TARGET_DIR` for routine
  repository work: changing the dependency source path or artifact directory
  invalidates reusable compilation. Do not create duplicate dependency caches
  without a demonstrated isolation need.
- `scripts/build-install-launch.sh` is the supported repeatable manual workflow
  for development builds, staged installs, explicit system installs, and
  launches. Keep system mutation behind its explicit `install` and `cycle`
  commands, and never compile as root.
- `scripts/package-release.sh` consumes the already verified
  `target/staged-install/` tree to create release artifacts. It must not invoke
  Cargo or Meson, create another build cache, or package files outside that
  staged layout.
- Manual desktop acceptance normally uses the script's `install` command and
  launches `/usr/bin/xfce4-profile-editor` through the configured Xfce profile
  action. Treat paths below `target/` as internal artifacts; do not ask the user
  to choose among them unless an uninstalled-build diagnostic specifically
  requires it.

If a responsibility does not fit these boundaries, stop and make the ownership
decision explicit before implementing it. Do not hide cross-layer behaviour in
generic `helpers` or `utils` modules.

## Licensing

- The project is licensed as `GPL-2.0-or-later`; do not add code or assets with
  incompatible terms.
- Before adding or updating a dependency, inspect the licence metadata for its
  complete resolved feature graph and confirm that every SPDX expression has a
  GPL-2.0-or-later-compatible choice. Record any unusual, missing, or custom
  licence before proceeding.
- Add `SPDX-License-Identifier: GPL-2.0-or-later` to new source, build, UI, and
  metadata files using the comment syntax appropriate to that format. The one
  exception is AppStream descriptive metadata, which uses `CC0-1.0` so Linux
  distributions may combine it into their metadata catalogues as intended.
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
- Every user-visible string must be translatable when introduced. Mark static
  GtkBuilder properties with `translatable="yes"` and call gettext for runtime
  UI messages in `src/ui/`. Core errors remain typed, locale-independent
  diagnostics.

## System data and security

- Do not edit `/etc/passwd` from the UI process, invoke a shell for account
  changes, collect passwords, or accept an arbitrary target user.
- Treat AccountsService, camera, filesystem, and image failures as
  typed recoverable errors; do not silently report partial success as complete.
- Tests for account changes must use disposable fixtures and must never alter a
  developer's real profile.

## Verification

Run formatting, Clippy with warnings denied, and the relevant tests for every
change. Use workspace-wide commands. UI work additionally requires resource
loading and exercising the affected runtime flow with a display or supported
headless backend. Before completion, confirm that no GTK/GDK imports or
presentation types have entered `src/core/`, and review the full diff for
missing resources, tests, or packaging changes.

On a machine in responsive build mode, use focused checks during implementation
and manual review. Run the complete workspace formatting, Clippy, test, and
packaging gate once after approval, immediately before commit; do not repeat the
full gate after every small iteration.

GitHub Actions must remain cost-conscious: keep one standard Linux validation
job, cancel superseded runs, avoid scheduled or matrix jobs, and retain
temporary artifacts for no more than seven days unless the project explicitly
revises its retention policy. Workflows use read-only token permissions by default;
only the tag-triggered release job may request `contents: write`. Release
automation creates a draft for manual verification and must never publish a
release directly. Changes reach protected `main` through pull requests after
the required validation job passes on an up-to-date branch.
