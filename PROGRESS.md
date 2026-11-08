# Project Progress

Last updated: 2026-11-08

This file is the concise handoff record for continuing development from a new
machine or a fresh Codex session. `IMPLEMENTATION_PLAN.md` remains the source of
truth for scope and architecture; this file records what has actually been
completed and what should happen next.

## Current position

- Active branch: `main`
- Last completed phase: Phase 2 — profile reads
- Next phase: Phase 3 — safe persistence
- Last approved commit: `3e95734` (`Add safe profile helper boundary`)
- Completed checkpoint: Phase 3a — GECOS persistence security decision
- Completed checkpoint: Phase 3b — pure helper boundary and tests
- Completed checkpoint: Phase 3c — one-shot helper runner and Polkit packaging
- Next checkpoint: Phase 3d — non-root staged installation
- Expected repository state after the Phase 3c commit: clean and pushed

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

### Phase 2: profile reads

- Added a plain Rust profile model and first-name/remainder name split.
- Added lossless GECOS parsing that retains missing, empty, hidden, and extra
  fields exactly when serialised.
- Added current-user lookup using the effective UID and `getent passwd`, so
  the lookup follows the machine's NSS configuration without unsafe code.
- Added read-only AccountsService access for `Email` and `IconFile` over the
  system D-Bus.
- Added bounded background reads of `~/.face` and the AccountsService icon,
  preserving the fallback order and allowing the UI to skip corrupt images.
- Added a read-only, scrollable GTK form with loading, partial-warning, and
  fatal-error states. Blocking account and file work runs off the GTK thread.
- Matched the supplied Whisker Menu settings reference with a framed content
  region, a separate fixed button footer, sentence-case labels ending in
  colons, and themed icons on the Help and Close buttons.
- Added focused parser, model, service-boundary, degraded-service, resource,
  and display-backed UI tests.

Automated verification passed:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
GSK_RENDERER=cairo G_DEBUG=fatal-warnings cargo test --all-features
gtk4-builder-tool validate ui/windows/profile-window.ui
```

Live verification on the development machine found UID 1000 through NSS,
found `/org/freedesktop/Accounts/User1000`, read its empty email and
`/home/kris/.face` icon path, and kept the application running without GTK
fatal warnings.

Manual verification was approved on 2026-11-08, including the avatar, current
profile values, scrolling behaviour, Whisker-style framed layout and footer,
and Close button.

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

Add a native installation target that supports `DESTDIR`, then stage an install
under the build directory without root. Verify the exact file list, executable
and data modes, and rendered absolute Polkit helper path. Do not install under
`/usr`, register the policy, or exercise a real account write.

Phase 3c verification passed with 28 tests:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
GSK_RENDERER=cairo G_DEBUG=fatal-warnings cargo test --workspace --all-features
gtk4-builder-tool validate ui/windows/profile-window.ui
xmllint --noout data/polkit/io.github.camelofcode.Xfce4ProfileEditor.policy.in
```

The built helper links only the standard C/GCC runtime, not GTK. A direct
unprivileged invocation refused to proceed with structured `not_privileged`
status and exit code 4, before reading a request or account data.

Phase 3b includes:

- a single shared lossless GECOS implementation used by the application and
  helper;
- conservative validation duplicated at the privileged boundary;
- a size-limited, versioned JSON request and structured response model;
- exact local `/etc/passwd` account selection from a caller UID, rejecting UID
  0, missing, duplicate, and malformed entries;
- stale-data rejection using the exact originally loaded GECOS value;
- preparation of a fixed `/usr/sbin/usermod --comment ... -- USERNAME`
  argument vector without executing it;
- tests for short, empty, trailing-empty, hidden, and extra GECOS fields,
  hostile protocol input, target selection, validation, and command shape.

Phase 3b verification passed with 22 tests:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
GSK_RENDERER=cairo G_DEBUG=fatal-warnings cargo test --workspace --all-features
gtk4-builder-tool validate ui/windows/profile-window.ui
```

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
