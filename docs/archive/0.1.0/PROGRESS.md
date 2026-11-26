<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Project Progress

Archived after the 0.1.0 release. The handoff below records the state of
development on 2026-11-16.

## Current position

- Active branch: `phase-7-release-automation`
- Repository base: `f84880b` (merged desktop integration)
- Archived full-field implementation tag:
  `archive/full-profile-fields-2026-11-15` (pushed to `origin`)
- Current checkpoint: x86_64 release packages and draft-release automation —
  pull-request package construction and artifact upload pass
- Next checkpoint: target-distribution installation acceptance before merging,
  then exercise a manual workflow run from `main`
- Expected repository state after handoff: committed topic branch pushed with
  pull request #4 open against `main`

## Current checkpoint worktree

Implemented:

- added a review-triggered and manually runnable release-package build with
  read-only repository permissions and seven-day artifact retention;
- added a `vMAJOR.MINOR.PATCH` tag path that verifies the tag is on `main`,
  checks Meson/Cargo/AppStream version agreement, and grants `contents: write`
  only to the short draft-release job;
- packages the one verified Meson staging tree as `.deb`, `.rpm`, and
  `linux-x86_64.tar.xz`, with shared-library dependency discovery and
  `SHA256SUMS`;
- added Debian and RPM metadata, installed licence/copyright files, and a
  repeatable packaging script that never compiles;
- documented manual build testing, tagging, target-distribution acceptance,
  and the explicit draft-publication checkpoint.

Focused verification completed:

- release and installation scripts pass POSIX shell syntax checks;
- the workflow parses as YAML and all external actions are pinned to full
  commit hashes; the newly added artifact actions are MIT-licensed;
- the responsive release stage passes and contains the binary at `0755` plus
  desktop metadata, icon, licence, and copyright files at `0644`;
- Meson and Cargo both report version `0.1.0`;
- formatting, Clippy, and tests pass for all features and for the
  camera-disabled configuration (35 and 24 unit tests respectively, plus the
  resource test in each configuration);
- the pull-request release workflow builds and inspects the `.deb`, `.rpm`, and
  tar archive successfully, including declared GTK/AccountsService dependencies
  and installed-file lists;
- the seven-day workflow artifact downloads with all three packages and its
  recorded SHA-256 checksums verify successfully.

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
  packaging metadata and guidance are implemented, pending target-distribution
  installation tests.

### Phase 7 — CI and release automation (in progress)

- Manual and tag-triggered release-package builds are implemented without a
  matrix or schedule.
- Package construction and artifact inspection pass; the tag-only draft-release
  path remains intentionally unexecuted until the first approved release tag.

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
