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
- Last approved commit before this checkpoint: `f58b2ca` (`Add resource-aware build policy`)
- Completed checkpoint: Phase 3a — GECOS persistence security decision
- Completed checkpoint: Phase 3b — pure helper boundary and tests
- Completed checkpoint: Phase 3c — one-shot helper runner and Polkit packaging
- Completed checkpoint: Phase 3d — Meson packaging, gettext baseline, and
  non-root staged installation
- Completed checkpoint: resource-aware build policy and shared Cargo/Meson
  compilation cache
- Completed checkpoint: repeatable manual build, stage, install, and launch
  workflow
- Completed checkpoint: Phase 3e system installation and application launch
- Next checkpoint: Phase 3e safe Polkit prompt test with a stale request
- Expected repository state after the developer-workflow commit: clean and
  pushed

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
- Compile-heavy commands select a resource mode from the current machine. The
  netbook favours desktop responsiveness; capable workstations and CI retain
  Cargo's default parallelism. All environments run the same required checks.
- Direct Cargo and Meson builds share the workspace target cache rather than
  compiling the GTK dependency graph twice.
- A release runtime spot-check found the Rust/GTK4 application within about
  3.4 MiB PSS and 1.3 MiB private-dirty memory of a comparable temporary
  C/GTK4 window. Keep Rust and benchmark release-profile tuning before release.

## Next work

Phase 3d was manually approved on 2026-11-08. Meson is now the single supported
build/install entry point and drives locked Cargo workspace builds. It supports
standard `DESTDIR` and directory options and renders the absolute helper path
into the Polkit policy. The application template, runtime presentation
messages, and Polkit authentication prompt are gettext-enabled; the generated
POT is tracked from the start. This supersedes the uncommitted Makefile staging
draft after review of the gtk-rs application guidance.

Nothing is installed under `/usr`, and the staged policy is not registered.

Phase 3d verification currently passes with 29 tests and a generated catalog
containing all 19 current user-facing messages:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
GSK_RENDERER=cairo G_DEBUG=fatal-warnings cargo test --workspace --all-features
gtk4-builder-tool validate ui/windows/profile-window.ui
xmllint --noout data/polkit/io.github.camelofcode.Xfce4ProfileEditor.policy.in
meson compile -C target/meson-build xfce4-profile-editor-pot
msgfmt --check --check-accelerators=_ po/xfce4-profile-editor.pot
```

The Meson release build staged and verified:

```text
usr/bin/xfce4-profile-editor                                      0755
usr/libexec/xfce4-profile-editor-helper                          0755
usr/share/polkit-1/actions/io.github.camelofcode.Xfce4ProfileEditor.policy  0644
```

The rendered policy retains the gettext domain and binds the action to the
absolute `/usr/libexec/xfce4-profile-editor-helper` path. The helper still has
no GTK dependency.

The next manual checkpoint may install the same three files under `/usr` with
explicit approval and exercise the standard Polkit prompt using a deliberately
stale request that cannot reach `usermod`.

The build-ergonomics checkpoint was manually approved on 2026-11-08. Its
project skill detects CPU and memory before compile-heavy work. This netbook
selects one-job, low-priority responsive builds; an 8-or-more-CPU machine with
at least 16 GiB RAM selects unrestricted Cargo parallelism when memory is
available. Meson now shares Cargo's workspace target cache. The resource build
script also avoids rewriting unchanged generated CSS, eliminating a
self-triggered rebuild loop: an unchanged warm Meson release build fell from
about 23 seconds to 0.12 seconds. Clippy and all 29 tests pass in responsive
mode.

The developer-workflow checkpoint was manually approved on 2026-11-08. It adds
`scripts/build-install-launch.sh`. Its safe `build` and `stage` commands never
request elevation. Explicit `install` and `cycle` commands stage and verify the
release before using `sudo install` to copy only the three verified files, so
compilation and Meson metadata never run as root. The script does not invoke
the privileged helper or change account data. The system installation and
installed application launch were manually verified on the same date. The
installed policy has the expected gettext domain and absolute helper binding;
the safe Polkit prompt test remains deliberately separate.

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
