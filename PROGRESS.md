# Project Progress

Last updated: 2026-11-07

This file is the concise handoff record for continuing development from a new
machine or a fresh Codex session. `IMPLEMENTATION_PLAN.md` remains the source of
truth for scope and architecture; this file records what has actually been
completed and what should happen next.

## Current position

- Active branch: `main`
- Last completed phase: Phase 1 — project foundation
- Next phase: Phase 2 — profile reads
- Expected repository state after the Phase 1 commit: clean and pushed

## Completed

### Phase 1: project foundation

- Created the Cargo application with Rust 1.92 as the minimum toolchain.
- Selected `gtk4-rs` 0.11.5 with GTK 4.10 and XML validation features.
- Selected application ID `io.github.camelofcode.Xfce4ProfileEditor`.
- Added a resource-backed `GtkApplicationWindow` composite template.
- Established mirrored `ui/` and `src/ui/` directories.
- Established a GTK-independent `src/core/` boundary.
- Added SCSS compilation through Dart Sass or `sassc`.
- Added GResource compilation and registration.
- Added embedded-resource and display-backed window tests.
- Documented local build and verification commands.

Phase 1 verification passed:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
GSK_RENDERER=cairo G_DEBUG=fatal-warnings cargo test --all-features
gtk4-builder-tool validate ui/windows/profile-window.ui
```

## Agreed decisions

- Working name and executable: `xfce4-profile-editor`.
- Toolkit and language: Rust with GTK 4, without libadwaita or Relm4.
- Licence: `GPL-2.0-or-later`.
- Layout stays in `.ui`, styling in SCSS, thin GTK plumbing in `src/ui/`, and
  system/data logic in `src/core/`.
- Editable data is limited to the system-backed image, first and last name,
  home phone, email, and office phone fields.
- Initials, fax, LibreOffice, Pidgin, and other application integrations are
  out of scope.
- Authentication uses AccountsService/Polkit and a narrowly scoped privileged
  helper where required; the application never collects a password.
- Development proceeds one reviewable phase at a time. Work remains
  uncommitted until it has been manually approved, then it is committed and
  pushed directly to `main` while the repository is private.
- Pull requests and branch protection will become the normal workflow when the
  repository is public.

## Next work

Phase 2 adds read-only profile loading:

1. Implement the plain Rust profile model.
2. Implement lossless GECOS parsing with preservation of hidden fields.
3. Resolve the current user through NSS-safe APIs.
4. Read email and icon information from AccountsService.
5. Populate a read-only form and avatar fallback state.
6. Add focused core, D-Bus boundary, and UI-loading tests.

Do not implement profile writes, Polkit elevation, image selection, or camera
capture during Phase 2.

## Resuming on another machine

```sh
git clone git@github.com:camelofcode/xfce4-profile-editor.git
cd xfce4-profile-editor
cat PROGRESS.md
cat IMPLEMENTATION_PLAN.md
cargo test --all-features
```

If the repository is already cloned, pull `main` and check `git status` before
starting. Any uncommitted changes may be work awaiting review and must not be
discarded.
