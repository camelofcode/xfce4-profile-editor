# Project Progress

Last updated: 2026-11-10

This file is the concise handoff record for continuing development from a new
machine or a fresh Codex session. `IMPLEMENTATION_PLAN.md` remains the source of
truth for scope and architecture; this file records what has actually been
completed and what should happen next.

## Current position

- Active branch: `main`
- Last completed phase: Phase 5 — camera capture, including primary
  camera-equipped hardware acceptance
- Next phase: Phase 6 — desktop integration and packaging
- Last approved implementation commit: `0eb256e` (`Add camera capture
  workflow`)
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
- Completed checkpoint: Phase 3e Polkit denial and authenticated stale-request
  tests
- Completed checkpoint: Phase 3f application-side privileged-helper client
- Completed checkpoint: Phase 3g interactive GECOS persistence
- Completed checkpoint: Phase 3h AccountsService email persistence
- Completed checkpoint: Phase 4a — image validation and normalisation core
- Completed checkpoint: Phase 4b — asynchronous image selection and preview
- Completed checkpoint: Phase 4c — atomic image persistence and AccountsService update
- Completed checkpoint: Phase 4d — profile-image removal
- Completed checkpoint: Phase 5a — camera capability and device discovery
- Completed checkpoint: Phase 5b — camera window, bounded frame pipeline, and
  avatar-save integration
- Completed checkpoint: Phase 5c — primary camera-equipped hardware acceptance
- Remaining Phase 5 resilience checks: busy, denied, and unplugged-device paths
- Expected repository state: clean, with Phase 5b committed on `main`

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

Phase 3e was completed on 2026-11-08. The installed application, helper, and
policy exactly matched their staged artifacts. The action was registered with
`allow_active=auth_admin`, the absolute helper path, and the exact
`update-gecos` first argument. Dismissing the Xfce Polkit dialog produced
`pkexec` status 127 and left the passwd record unchanged. Approving the dialog
then produced helper status 3 and the structured response:

```json
{"status":"error","kind":"stale_data","message":"the account changed after it was loaded"}
```

The current profile values were supplied as the proposed values for defence in
depth, the deliberately incorrect `expected_gecos` value prevented the command
boundary from being reached, and the exact passwd record was unchanged after
both tests.

The first Phase 3f slice adds a GTK-independent application client for the
installed helper. It serialises the shared versioned protocol, validates and
bounds requests before starting a process, invokes only the fixed `pkexec`,
helper, and operation argument vector, drains bounded output, and maps Polkit
and every helper response to typed core errors. Process execution is injected
for tests: the 10 new client tests never start `pkexec`, display an
authentication prompt, or touch account data. Clippy passes with warnings
denied and all 39 workspace tests pass. That checkpoint deliberately left the
blocking boundary disconnected from profile state and GTK.

Phase 3f was manually approved and committed on 2026-11-08. The Phase 3g
checkpoint now retains the exact loaded GECOS representation alongside the
displayed form, prepares a single combined name-and-phone update, and advances
that snapshot only after confirmed helper success. The name and phone entries
become editable after loading; short-idle and focus-loss validation do not
request privilege, and Enter advances to the next field without writing. Only
closing with unsaved changes starts the blocking helper client through
`gio::spawn_blocking`, so the complete form produces at most one GECOS
elevation request. The form remains open with its edits after validation,
authentication, authorization, stale-data, or system failure. Name and phone
inputs are temporarily locked during the write, and Close waits for success
rather than discarding an edit. Email remains read-only for the separate
AccountsService checkpoint.

The Phase 3g automated checks pass with 42 workspace tests, Clippy warnings
denied, a display-backed window construction test, template and policy XML
validation, updated gettext extraction, and a Meson development build. Core
tests prove that an unchanged form does not invoke the client, a changed form
uses the exact loaded snapshot while preserving room/hidden/extra fields, and
invalid input never starts the helper. The first successful real profile write
is deliberately left for manual review because it displays Polkit and changes
the developer account.

Phase 3g was manually approved on 2026-11-08. The installed application
successfully accumulated the complete form, requested elevation only when the
form was closed, and saved the combined GECOS update as intended.

The Phase 3h checkpoint adds conservative email validation and writes the
current user's email through AccountsService `SetEmail`. The installed Fedora
AccountsService interface assigns own-email changes to
`org.freedesktop.accounts.change-own-user-data`; its local active-session
policy is `allow_active=yes`, so this machine should not display an elevation
prompt for email-only changes. Permission and service failures remain typed
and recoverable for systems with different policy.

The close-time transaction is coordinated in GTK-free core code. It validates
every changed value before writing, applies GECOS first, and attempts email
only after GECOS succeeds. If email then fails, the confirmed GECOS snapshot is
retained and a retry writes only email. When AccountsService was unavailable
during loading, email remains read-only. Tests use injected functions and a
fake email client; they never call the live D-Bus method or change account
data.

Phase 3h automated verification passes with 50 workspace tests, Clippy
warnings denied, updated gettext extraction, template and policy XML
validation, a display-backed window construction test, and a Meson development
build in responsive mode.

Manual testing confirmed that email changes persist without an elevation
prompt. It also found that leaving an untouched field incorrectly displayed
the unsaved-changes status. The clean-state path now restores the original
load warning or hides the status when no warning exists; this minor UI fix is
manually confirmed. Phase 3h and the complete Phase 3 safe-persistence flow
were approved on 2026-11-08.

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
the safe Polkit prompt test was completed separately as Phase 3e.

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

Phase 4a is implemented and awaiting review. It adds a GTK-free image function
that accepts PNG, JPEG, and WebP bytes; enforces 16 MiB input, 16,384-pixel
side, and 32-megapixel limits; requests the image library's 128 MiB decoder
allocation cap; applies embedded EXIF orientation; centre-crops without first
allocating a full-size crop; and returns a 512 by 512 RGBA PNG. It does not yet
add a file chooser, write `~/.face`, call AccountsService, or change the UI.

Six focused tests cover PNG crop/output, JPEG EXIF orientation, WebP input,
invalid and oversized data, excessive dimensions, and excessive total pixels.
Clippy passes with warnings denied and the full workspace suite passes with 56
tests. The core boundary search finds no GTK/GDK imports. Phase 4a was manually
approved on 2026-11-08.

Phase 4b is implemented. Activating the avatar now
opens a template-defined action popover. Choose photo uses GTK's asynchronous
native file dialog, filtered to PNG, JPEG, and WebP; Take photo is hidden until
camera support exists, and Remove photo remains disabled for its later storage
checkpoint. A selected local file is read with a hard 16 MiB bound and passed
through the Phase 4a normaliser on a blocking worker, then the GTK layer renders
only the resulting PNG preview. Cancellation is silent and file, decode, or
preview failures are translated recoverable statuses with technical details in
the tooltip. This checkpoint deliberately does not write `~/.face` or call
AccountsService.

Two additional core tests cover bounded local-file reads and rejection of
directories and oversized files. Clippy passes with warnings denied, the full
workspace suite passes with 58 tests, the updated template validates, gettext
extraction contains all new UI messages, and the core boundary search finds no
GTK/GDK imports. Manual review confirmed the native picker, centre-cropped
preview, and error-free interaction. The release build produced the preview
immediately; the roughly five-second debug-build delay was compiler
optimisation rather than a release-runtime issue. Phase 4b was approved on
2026-11-08.

Phase 4c is implemented and awaiting manual review. Choosing a photo now runs
one GTK-free core operation on the blocking worker: it normalises the source,
writes a uniquely created temporary file beside `~/.face`, synchronises it,
sets mode `0644`, and atomically renames it into place. It then calls
AccountsService `SetIconFile` for the loaded UID with the stable `.face` path
and D-Bus interactive authorization enabled. The preview changes only after a
confirmed local write. AccountsService failure is represented as partial
success, retaining the confirmed local image and showing a translated warning
with the service diagnostic.

Three focused tests use temporary homes and an injected icon client. They
prove atomic replacement and permissions, the stable AccountsService path,
partial service failure, and that invalid input neither changes `.face` nor
calls the service. Clippy passes with warnings denied, the full workspace suite
passes with 61 tests, gettext extraction is current, and the core boundary
search finds no GTK/GDK imports. Manual release testing confirmed immediate
normalisation, persistence across reopening, and the updated Whisker Menu
image. Phase 4c was approved on 2026-11-08.

Phase 4d is implemented and awaiting manual review. Remove photo is enabled
whenever an avatar candidate is loaded or a new image has been saved. It runs
off the GTK thread, treats a missing `~/.face` as already removed, refuses to
clear AccountsService when local deletion fails, and clears AccountsService
with interactive authorization only after local success. The preview returns
to the themed default only after confirmed local removal; a subsequent service
failure is shown as explicit partial success.

Four focused removal tests cover complete success, an already-missing local
file, local failure before the service boundary, and AccountsService failure
after local success. Clippy passes with warnings denied, the full workspace
suite passes with 65 tests, the template validates, gettext extraction is
current, and the core boundary search finds no GTK/GDK imports. Manual testing
confirmed removal in the editor and Whisker Menu, persistence across reopening,
and successful restoration by choosing the original image again. Phase 4d and
the complete Phase 4 image-selection flow were approved on 2026-11-08.

Phase 5a is implemented. Nokhwa 0.10.11 is selected
with only its native V4L2 input feature, behind the default-enabled Cargo
feature `camera`; Meson can omit the complete camera dependency graph with
`-Dcamera=false`. Discovery runs off the GTK thread, retains sparse device
indices reported by V4L2 instead of assuming `/dev/video0`, and records
`NotCompiled`, `NoDevices`, available devices, or a typed backend error without
starting a camera stream. The Take photo action remains hidden until the capture
window exists in Phase 5b.

The resolved all-feature Cargo graph was audited from package licence metadata.
No package lacks licence metadata, and every SPDX expression offers a
GPL-2.0-or-later-compatible choice. The decision and camera-specific licence
details are recorded in `docs/phase-5-camera-backend.md`; `AGENTS.md` now makes
the same resolved-graph check mandatory for future dependency changes.

Three focused tests cover no-device discovery, preservation of multiple sparse
device indices, and typed backend failure. Both all-feature and
`--no-default-features` configurations compile cleanly. Formatting and Clippy
pass with warnings denied, and the full all-feature workspace suite passes with
68 tests. The sandbox exposed no `/dev/video*` device, so that checkpoint's
manual review was limited to a normal launch with Take photo hidden. The host
session does have a working webcam for the Phase 5b acceptance test.

The Phase 5a application smoke test was approved on 2026-11-09.

Phase 5b is implemented and approved. The core opens Nokhwa only on a
dedicated worker, negotiates a safe format close to 640 by 480 at 30 frames per
second, and replaces a single latest-frame slot so stale frames cannot queue.
MJPEG uses the existing bounded image decoder; YUYV, NV12, raw RGB/BGR, and
greyscale use Nokhwa's conversion path. All data crossing into GTK is a plain,
validated RGB frame.

The resource-backed camera window provides translated Capture, Retake, Use
photo, and Cancel states with icon-bearing footer buttons. It creates a GTK
memory texture only for the frame being displayed. Capturing freezes that
frame while the worker continues to use the bounded slot; every close, cancel,
error, and application teardown path signals shutdown and joins the worker off
the GTK thread. An accepted frame uses the Phase 4 centre-crop, 512-pixel PNG,
atomic `.face`, and AccountsService persistence path without an intermediate
image encoding.

Nine new focused tests cover capture-node filtering, latest-frame replacement, worker shutdown/source
release, frame errors, safe format selection, MJPEG decoding, malformed frame
lengths, direct RGB normalisation, and camera-frame persistence. The existing
display-backed test now constructs the camera template and exercises live,
captured, and retake button states. Clippy passes with warnings denied and all
77 all-feature workspace tests pass. Initial host testing exposed a device
selection bug: Nokhwa enumerated the webcam's metadata-only `/dev/video1` node
before its capture-capable `/dev/video0` node. Discovery now filters nodes using
their V4L2 video-capture and streaming capabilities and sorts the remaining
devices by real index. The corrected backend path discovered `/dev/video0` and
captured a decoded 640 by 480 RGB frame on the host. Preview, Capture, Retake,
Use photo, and device release were manually verified again on 2026-11-10 using
the staged release on a camera-equipped Cinnamon/X11 workstation. The complete
flow worked without GTK warnings despite running outside its intended Xfce
desktop. Busy, denied, and unplugged-device behaviour remain additional
resilience checks rather than blockers to publishing the repository as a work
in progress.

## Public-readiness checkpoint

The 2026-11-10 audit found the repository clean and synchronised with `main`,
with no common credential signatures or suspicious historical filenames. The
GPL licence, README, SPDX headers, architecture boundary, and locked dependency
graph are present. Rust 1.99.1, GTK 4.22.5, Meson 1.12.0, and the documented
native tools were available on the workstation; no additional development
packages were required.

Formatting, warnings-as-errors Clippy, all 77 camera-enabled tests, the
camera-disabled Clippy/test configuration, both GtkBuilder templates, the
Polkit policy, and the gettext catalogue passed. The safe Meson development
build completed, and a clean optimised release plus non-root staged
installation completed and verified. The full camera workflow was then
manually accepted on Cinnamon/X11.

A single public-only GitHub Actions job now mirrors those checks on the
standard Ubuntu 24.04 runner, pins its checkout action, cancels superseded
runs, and performs no packaging publication. The job remains disabled while
the repository is private so it cannot consume private Actions minutes.

The final repository-layout cleanup consolidates both build entry points under
`scripts/`: `build-install-launch.sh` remains the supported developer workflow,
while Meson invokes the internal `cargo-build.sh` driver from the same
directory. No parallel build path or behaviour was added.

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
