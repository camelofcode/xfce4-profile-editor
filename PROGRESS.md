<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Project Progress

Last updated: 2026-11-16

This is the concise handoff record. `IMPLEMENTATION_PLAN.md` is the source of
truth for current scope and architecture.

## Current position

- Active branch: `phase-6-desktop-integration`
- Repository base: `c7d973c` (merged Whisker-focused scope and readability
  refactor)
- Archived full-field implementation tag:
  `archive/full-profile-fields-2026-11-15` (pushed to `origin`)
- Current checkpoint: Phase 6 desktop integration — manually accepted with the
  final gate passing; ready for a pull request
- Next checkpoint: distro packaging guidance, then Phase 7 CI release
  automation
- Expected repository state after handoff: committed topic branch pushed with
  a pull request open against `main`

## Current checkpoint worktree

Implemented:

- added a translated **Profile Editor** desktop launcher in Xfce's personal
  Settings category;
- added an original scalable application icon and installed it through the
  hicolor icon theme;
- added translated AppStream metadata for software catalogues;
- kept code, build files, the icon, and desktop entry under
  `GPL-2.0-or-later`, with the AppStream metadata under `CC0-1.0` as required
  for catalogue aggregation;
- updated Meson installation and post-install cache refreshes;
- expanded the authoritative build script to verify and install the complete
  desktop layout without compiling as root;
- added desktop-entry and AppStream validation to the single CI job;
- updated README installation and Whisker configuration guidance.

Focused verification completed:

- translated desktop and AppStream files generate successfully;
- `desktop-file-validate` passes;
- AppStream validation passes, with only the existing mixed-case application
  ID reported as a pedantic preference;
- gettext extraction includes desktop and AppStream strings;
- the release stage contains the binary at `0755` plus desktop entry, icon,
  and AppStream metadata at `0644`;
- SVG/XML parsing and `git diff --check` pass;
- the system installation appears as **Profile Editor** in Xfce Settings with
  the expected icon and launches successfully;
- formatting and Clippy pass with warnings denied for both all features and
  the camera-disabled configuration;
- all-feature testing passes with 35 unit tests plus the resource test, and
  camera-disabled testing passes with 24 unit tests plus the resource test;
- both GtkBuilder templates validate against the active desktop display.

## Completed phases

### Phase 1 — project foundation

- Rust/GTK 4 composite-template application with GTK 4.10 minimum.
- Meson drives Cargo; GResource, SCSS, gettext, and display-backed tests are
  established.
- UI layout, GTK plumbing, and GTK-independent core boundaries are enforced.

### Phase 2 — profile reads

- Effective-UID/session-home lookup and AccountsService integration.
- Asynchronous loading, local avatar fallback, degraded-service warnings, and
  a small-screen-friendly Whisker-styled window.

### Former Phase 3 — full profile persistence

- The lossless GECOS helper, Polkit path, phone fields, and email persistence
  were implemented and manually verified.
- This implementation was removed from current scope on 2026-11-15 and remains
  recoverable from `archive/full-profile-fields-2026-11-15`.

### Phase 4 — image selection and persistence

- Bounded PNG/JPEG/WebP normalisation with orientation, square crop, and
  512×512 PNG output.
- Asynchronous GTK file selection, atomic `~/.face` replacement, mode `0644`,
  AccountsService `SetIconFile`, partial-failure reporting, and removal.
- Manually confirmed in the editor and Whisker Menu.

### Phase 5 — camera capture

- Optional Nokhwa/V4L2 discovery, sparse device indices, camera window,
  bounded latest-frame pipeline, capture/retake/use-photo flow, and shared
  avatar persistence.
- Camera-equipped hardware acceptance passed on Cinnamon/X11.
- Busy, denied, and unplugged-device resilience checks remain outstanding.

### Phase 6 — desktop integration and packaging (in progress)

- The Xfce Settings launcher, scalable icon, AppStream metadata, translation
  extraction, staged layout checks, and CI metadata validation are implemented.
- Xfce Settings installation and launch acceptance passed; portable distro
  packaging guidance remains.

## Build and workflow decisions

- `scripts/build-install-launch.sh` is the authoritative application build,
  stage, install, and launch workflow.
- Direct Cargo is used only for formatting, Clippy, and tests.
- Cargo and Meson share the normal Cargo home and workspace `target/`; do not
  create temporary dependency caches.
- The netbook uses one low-priority compile job; stronger machines and CI may
  use their available resources.
- On constrained hardware, use focused checks during iteration and run the
  full gate once after manual approval, immediately before commit.
- Manual acceptance installs the binary and desktop metadata through Meson,
  then checks both Xfce Settings and the configured Whisker action.
- Protected `main` receives changes through pull requests after the single
  required **Build, test, and stage** job passes.

## Agreed product decisions

- Initial product scope is display name plus profile image only.
- Login name and password are explicitly out of scope.
- Broader account fields will be reconsidered only after Xfce feedback.
- Rust/GTK 4, no libadwaita or Relm4.
- GPL-2.0-or-later.
- All user-visible strings are translatable.
- The application never collects a password, targets another user, edits
  `/etc/passwd`, or invokes a shell for account changes.
