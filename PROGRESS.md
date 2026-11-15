<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Project Progress

Last updated: 2026-11-15

This is the concise handoff record. `IMPLEMENTATION_PLAN.md` is the source of
truth for current scope and architecture.

## Current position

- Active branch: `whisker-focused-profile`
- Repository base: `5679c47` (merged public pull-request workflow)
- Archived full-field implementation tag:
  `archive/full-profile-fields-2026-11-15` (pushed to `origin`)
- Current checkpoint: Whisker-focused scope simplification — pull request #2
  open for review
- Next checkpoint after merge: Phase 6 desktop integration and packaging
- Expected repository state after handoff: committed topic branch pushed with
  a pull request open against `main`

## Approved current checkpoint

The product scope is now the two values presented by Whisker Menu's profile
header: the current user's display name and profile image.

Implemented in the worktree:

- replaced first name, last name, office phone, home phone, and email with one
  **Display name** field;
- changed name loading/writing to AccountsService `RealName`/`SetRealName`;
- removed the remaining passwd/GECOS parser and display-name fallback;
- reduced current-user resolution to the effective UID and the desktop
  session's absolute home directory, which are the only values needed for
  AccountsService and `~/.face`;
- removed legacy GECOS character validation and delegated display-name policy
  to AccountsService;
- retained close-time dirty-state saving and recoverable write errors;
- removed the shared GECOS crate, privileged-helper crate, helper client,
  combined profile transaction, email persistence, JSON protocol, and project
  Polkit policy;
- reduced Meson packaging to the application binary;
- removed the empty packaging-data scaffold and legacy helper/policy migration
  path; staged installs are rebuilt from a clean generated destination;
- removed the one-shot `ProfileStore` and `Profile` wrappers, the redundant
  avatar-source variants, and an unused camera-description field;
- replaced stateless current-user and AccountsService adapter structs with
  plain functions, using closures only at the internal test seams;
- added a required repository skill covering function-versus-struct choices,
  trait use, typed enums, wrapper types, visibility, and structural review;
- grouped core code into cohesive `profile` and `camera` domains, with camera
  capture isolated from device discovery and `core/mod.rs` retained as the UI
  façade;
- updated CI, translation inputs, repository guidance, plan, and packaging
  documentation for the reduced architecture;
- reduced the profile window from 560×600 to a fixed 460×360 and removed the
  disabled Help button;
- preserved the prior implementation in the pushed archival tag above.

The project no longer owns an elevation policy. AccountsService remains free
to apply the distribution's policy and may show a system authentication prompt.
`SetRealName` owns the legacy GECOS representation; room/phone/other fields are
not part of the simplified application's contract.

Verification completed:

- authoritative Meson development build passes in responsive mode;
- the changed GtkBuilder profile template validates (Mesa warnings only);
- Clippy passes with warnings denied for all features and with camera support
  disabled;
- all-feature testing passes with 35 unit tests plus the embedded-resource
  integration test, including the display-backed window flow;
- camera-disabled testing passes with 24 unit tests plus the resource test;
- gettext extraction and message validation pass with the reduced string set;
- the release stage contains only `usr/bin/xfce4-profile-editor` at mode `0755`;
- `src/core/` contains no GTK/GDK imports;
- formatting and `git diff --check` pass;
- the installed compact window, display-name save, and avatar flow were
  manually approved through the configured Xfce Whisker profile action.

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
- Manual acceptance installs `/usr/bin/xfce4-profile-editor` and launches it
  through the configured Whisker action.
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
