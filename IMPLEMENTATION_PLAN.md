# Xfce Profile Editor Implementation Plan

## 1. Purpose

Build a small, maintainable Rust/GTK 4 application that replaces the useful
system-profile parts of Mugshot for Xfce. It must work as the command launched
by Whisker Menu's **Edit Profile** action without depending on Mugshot.

Working identifiers:

- repository and package: `xfce4-profile-editor`
- executable: `xfce4-profile-editor`
- window title: `Profile`
- application ID: `io.github.camelofcode.Xfce4ProfileEditor`
- licence: GNU General Public License version 2 or later
  (`GPL-2.0-or-later`)

The project is initially an independent proposal, not an official Xfce
component. A reverse-DNS application ID will be chosen after the hosting and
project namespace is finalised.

## 2. Scope

### In scope

- Display the current profile image as a circular image.
- Select a profile image from disk.
- Take a profile image using an available webcam.
- Update `~/.face` and the current user's AccountsService icon.
- Edit first name and last name as the GECOS full-name value.
- Edit office and home phone numbers in their standard GECOS fields.
- Edit the email address exposed by AccountsService.
- Provide immediate-update behaviour, with Help and Close buttons rather than
  Save and Cancel.
- Work under X11 and Wayland in an Xfce session.

### Out of scope

- Initials and fax fields, because they have no relevant standard system store.
- LibreOffice, Pidgin, or other application-specific synchronisation.
- Editing another user's account.
- Password, login name, shell, groups, account type, or automatic-login
  management.
- Stock avatar galleries in the first release.
- A compatibility executable named `mugshot`, which could conflict with the
  existing package.

## 3. Data ownership

There must be one authoritative store for each editable value.

| UI value | Authoritative storage | Notes |
| --- | --- | --- |
| Profile image | `~/.face` and AccountsService `IconFile` | Keep both consumers in sync. |
| First and last name | GECOS full-name field | Combine for storage and split for display. |
| Office phone | GECOS office-phone field | Preserve every other GECOS field. |
| Home phone | GECOS home-phone field | Preserve every other GECOS field. |
| Email address | AccountsService `Email` | Do not encode it into GECOS. |

The standard GECOS sequence is full name, room, office phone, home phone, and
other. The application does not expose room or other, but every read-modify-write
operation must preserve them exactly.

No application-specific GSettings profile store is needed.

## 4. Toolkit and dependencies

- Rust, using the stable toolchain supported by the selected `gtk4-rs` release.
- GTK 4.10 or newer.
- Raw `gtk4-rs` composite templates rather than Relm4, so layout remains in
  `.ui` resources.
- Meson is the supported project build and installation entry point. It drives
  Cargo for Rust compilation and provides the native packaging and gettext
  integration expected by the GTK ecosystem.
- GResource for UI templates, compiled CSS, icons, and other static assets.
- SCSS for the small amount of styling needed for the avatar and status states.
- GNU gettext for every user-visible string from the start: GtkBuilder strings
  use `translatable="yes"`, Rust presentation strings use `gettext-rs`, and
  system-owned Polkit prompts declare the same text domain.
- A D-Bus client suitable for keeping AccountsService access outside the GTK
  layer.
- `nokhwa` with the native Linux backend for webcam discovery and capture,
  isolated behind a Cargo feature.
- A non-GTK image library for orientation, square cropping, resizing, and PNG
  encoding.

Cargo remains the Rust compiler and workspace/test runner. `build.rs` compiles
the embedded SCSS and GResources for both Cargo and Meson builds, while Meson
owns supported build profiles, installation paths, policy rendering, and
translation catalogs. There must not be a second Make-based install path.

## 5. Repository layout

```text
.
├── AGENTS.md
├── Cargo.toml
├── build.rs
├── meson.build
├── meson_options.txt
├── resources.gresource.xml
├── build-aux/
│   └── cargo-build.sh
├── crates/
│   ├── profile-data/
│   │   └── src/lib.rs
│   └── profile-helper/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── main.rs
├── data/
│   ├── meson.build
│   ├── icons/
│   ├── polkit/
│   ├── xfce4-profile-editor.desktop.in
│   └── xfce4-profile-editor.metainfo.xml.in
├── po/
│   ├── LINGUAS
│   ├── POTFILES.in
│   ├── meson.build
│   └── xfce4-profile-editor.pot
├── scripts/
│   └── build-install-launch.sh
├── src/
│   ├── config.rs
│   ├── i18n.rs
│   ├── meson.build
│   ├── main.rs
│   ├── core/
│   │   ├── account_service.rs
│   │   ├── camera.rs
│   │   ├── gecos.rs
│   │   ├── image.rs
│   │   ├── profile.rs
│   │   └── profile_store.rs
│   └── ui/
│       ├── mod.rs
│       └── windows/
│           ├── camera_window.rs
│           └── profile_window.rs
├── ui/
│   ├── sass/
│   │   ├── _camera-window.scss
│   │   ├── _profile-window.scss
│   │   └── app.scss
│   └── windows/
│       ├── camera-window.ui
│       └── profile-window.ui
└── tests/
```

The `ui/` and `src/ui/` directories mirror one another. Static widget
hierarchy, spacing, labels, accessibility relationships, menus, and actions
belong in `.ui` files. Rust UI modules bind template children, connect signals,
translate core results into presentation state, and manage window lifecycles.
All system interaction and data transformation belongs in `src/core/`.
Lossless GECOS data rules shared across the privilege boundary belong in
`crates/profile-data/`; the GTK-free privileged boundary belongs in
`crates/profile-helper/`.

Dependency direction is always `ui -> core`. Core modules must not import GTK
or GDK types and must expose plain Rust data and typed errors.

## 6. Core model and services

The main model should remain small:

```rust
struct Profile {
    first_name: String,
    last_name: String,
    home_phone: String,
    email: String,
    office_phone: String,
    avatar: Option<Avatar>,
}
```

`ProfileStore` coordinates independent services but does not hide which store
failed. Partial failure must be representable so the UI can show that, for
example, the local image was saved but AccountsService could not be updated.

### Loading

1. Resolve the current UID and account through NSS-safe APIs.
2. Parse GECOS into a lossless representation, retaining empty and unexposed
   fields.
3. Read AccountsService `Email` and `IconFile` when available.
4. Prefer a valid `~/.face` for display, then a valid AccountsService icon,
   then the themed default avatar.

### Validation

- Trim accidental surrounding whitespace without changing meaningful internal
  spacing.
- Reject GECOS delimiters, colons, newlines, and NUL characters before invoking
  privileged code.
- Apply conservative phone validation that permits common international phone
  formatting rather than silently deleting characters.
- Allow an empty value to clear any editable field.

## 7. Privileged GECOS updates

The Phase 3 compatibility and security decision is recorded in
[`docs/phase-3-gecos-persistence-spike.md`](docs/phase-3-gecos-persistence-spike.md).
It selects a narrowly scoped `pkexec` helper using the system `usermod`
primitive and rejects AccountsService `SetRealName` and `chfn` because they do
not preserve every GECOS field representation required by this project.

The GTK process must never edit `/etc/passwd`, collect a password, invoke a
shell, or interpolate user values into a command string.

Before the form is implemented, build a focused compatibility and security
spike which proves a safe update path on Fedora. The preferred design is a
small installed helper with a narrowly scoped Polkit action:

- it derives the target account from the authenticated caller and cannot name
  an arbitrary target user;
- it accepts only the three supported GECOS values;
- it validates all input again at the privilege boundary;
- it preserves the room and other GECOS fields;
- it uses the distribution account-management primitive without a shell;
- it returns structured success, cancellation, denial, validation, and system
  errors;
- it refuses non-local accounts rather than pretending to update them.

The spike must also evaluate the current AccountsService `SetRealName`
behaviour. It may be used only if tests prove that it preserves the other GECOS
subfields on every supported target. Do not maintain a second real-name copy or
perform two competing writes.

The exact helper protocol and Polkit policy are not considered final until they
have received a dedicated security review.

## 8. Main window

Use a resizable `GtkApplicationWindow` containing a `GtkScrolledWindow` so it
remains usable on small screens.

The `.ui` template contains:

```text
             (circular profile image)

First Name       [                              ]
Last Name        [                              ]
Home Phone       [                              ]
Email Address    [                              ]
Office Phone     [                              ]

[ Help ]                                      [ Close ]
```

Activating the image opens a popover with Choose Photo, Take Photo, and Remove
Photo. Take Photo is disabled or hidden when camera support is not compiled or
no camera is available.

There is no Save or Cancel button. Editing follows immediate-update semantics:

- mark text fields dirty while typing;
- validate after a short idle debounce;
- commit on focus loss or Enter;
- coalesce related GECOS changes into one privileged operation;
- show Saving, Saved, and error states without blocking the main loop;
- on Close, flush pending valid changes and wait for active writes;
- keep the window open when a write fails so the user can retry or explicitly
  discard the unsaved edit.

## 9. Profile images

Image processing runs outside the GTK main thread:

1. Open an asynchronous GTK file dialog filtered to supported image MIME types.
2. Decode the selected image with bounded dimensions.
3. Apply embedded orientation.
4. Centre-crop to a square and resize to 512 by 512 pixels.
5. Encode a normalised PNG.
6. Write beside `~/.face`, set appropriate permissions, and atomically rename
   it into place.
7. Call AccountsService `SetIconFile` using the stable `~/.face` path with
   interactive authorization enabled.
8. Update the preview and Whisker-visible state only from the confirmed result.

Do not pass a private temporary path to AccountsService. Removing a photo must
have equally explicit behaviour and tests for both storage locations.

## 10. Webcam window

Camera support is a separate core service and a second `.ui`-defined window.

- Enumerate devices instead of assuming `/dev/video0`.
- Open capture on a worker thread.
- Use a bounded latest-frame channel so the camera cannot queue unbounded work.
- Convert only the displayed frame to a GTK texture in the UI layer.
- Offer Capture, Retake, Use Photo, and Cancel states.
- Feed an accepted frame through the same normalisation and save pipeline used
  for files.
- Stop capture and release the device when the window closes, the application
  exits, or an error occurs.
- Handle permission denial, unplugging, unsupported formats, and a busy camera
  as normal errors.

## 11. Implementation phases

### Phase 1: project foundation

- Create the Cargo package and module tree.
- Compile and register `.ui`, SCSS, and icon resources.
- Create an empty composite-template main window.
- Add rustfmt, Clippy, unit-test, and resource-validation commands.

Completion: a clean build opens and closes the resource-backed main window
without GTK warnings.

### Phase 2: profile reads

- Implement the profile model and lossless GECOS parser.
- Add current-user resolution and AccountsService reads.
- Populate a read-only version of the form and avatar fallback chain.

Completion: values match `getent passwd` and AccountsService on a test machine,
including accounts with short or partially empty GECOS data.

### Phase 3: safe persistence

- Complete the Polkit/helper spike and document the decision.
- Implement name and phone writes with preservation tests.
- Implement AccountsService email writes.
- Add dirty-state, commit, error, and close-flush behaviour.

Completion: each field can be changed and cleared; room and other GECOS values
remain byte-for-byte unchanged; denied authentication loses no edit.

### Phase 4: image selection

- Implement image validation and normalisation.
- Add the file dialog and circular preview.
- Implement atomic `~/.face` and AccountsService updates.
- Add removal behaviour.

Completion: Whisker Menu and the application show the new image after a
successful update, including after logout and login.

### Phase 5: camera capture

- Add device discovery and availability state.
- Implement the camera window and frame pipeline.
- Reuse the Phase 4 image-save path.

Completion: capture works with a default webcam, cleanly handles no camera, and
releases the device on every exit path.

### Phase 6: desktop integration and packaging

- Add the desktop file, application icon, AppStream metadata, help, initial
  translation catalogs, and translator documentation. The Meson/gettext
  infrastructure and message marking are established in earlier phases.
- Package the helper and Polkit policy with safe ownership and permissions.
- Document setting Whisker Menu's Edit Profile command to
  `xfce4-profile-editor`.
- Add Fedora build and installation instructions.

Completion: a clean package install appears in the application menu, launches
from Whisker, and passes metadata validation.

### Phase 7: CI and release automation

Add GitHub Actions only after the local build and test commands are stable, so
CI executes established project checks rather than becoming a second build
system.

While the repository is private, keep the workflow deliberately economical:

- run on pull requests, pushes to `main`, and explicit manual dispatches;
- use one standard Linux runner and one combined validation job rather than a
  distribution or toolchain matrix;
- run formatting, Clippy, tests, resource validation, and the supported
  headless GTK checks;
- cache Cargo downloads and appropriate build output with a bounded cache key;
- cancel an older in-progress run when a newer commit arrives on the same
  branch;
- do not add scheduled, nightly, coverage, or multi-distribution jobs until the
  repository is public or there is a demonstrated need;
- upload artifacts only for manual package/release runs or when an artifact is
  necessary to diagnose a failure, and retain temporary artifacts for no more
  than seven days;
- use read-only `GITHUB_TOKEN` permissions by default and grant
  `contents: write` only to the release job;
- pin every third-party action to a reviewed full commit SHA.

Before enabling CI, set the repository's Actions spending limit to zero or
otherwise ensure jobs stop when the included private-repository allowance is
exhausted. Review actual runner usage after the first month rather than
assuming the initial workflow is cheap enough.

Create a separate release workflow with this lifecycle:

1. A maintainer updates the version and release notes and runs all checks
   locally.
2. A signed annotated tag named `vMAJOR.MINOR.PATCH` is pushed.
3. The tag workflow verifies that the tag matches the package version and that
   the commit passes the complete validation suite.
4. It builds the agreed source archive and supported package artifacts, then
   generates SHA-256 checksums.
5. It creates a **draft** GitHub Release using the repository's `GITHUB_TOKEN`
   and attaches the verified artifacts and generated release notes.
6. A maintainer downloads or installs the artifacts, checks the draft, and
   publishes it manually.

Do not put long-lived signing keys in Actions for the initial release process.
If signed packages become necessary, design that key-management path
separately. Exercise the release workflow through a non-publishing manual dry
run before creating `v0.1.0`.

When the repository becomes public, standard GitHub-hosted runners no longer
consume the private-repository compute allowance. At that point, consider a
supported-distribution matrix, scheduled dependency/security checks, coverage,
and longer-lived useful artifacts; do not enable them automatically merely
because they are free.

Completion: ordinary CI is fast, cancellable, least-privileged, and remains
within the configured zero-spend policy; a version tag can reproducibly create
a reviewed draft release without publishing it automatically.

## 12. Verification

### Automated tests

- Lossless GECOS parse/serialise tests, including missing and extra fields.
- Tests proving room and other are preserved across every supported update.
- Name and phone validation tests, including malicious delimiter input.
- Name split/join round trips for single and multi-part names.
- AccountsService tests against a fake D-Bus service.
- Image orientation, crop, resize, size-limit, and atomic-write tests.
- Helper tests against disposable account fixtures only; tests must never alter
  the developer's real account.
- Resource-load and window-lifecycle tests under a display or supported
  headless backend.
- Camera-service tests with a fake frame source.
- Workflow tests or dry runs proving that CI uses the same commands documented
  for local development and that the release path cannot publish from an
  unverified commit.

### Required checks for each feature

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
meson setup target/meson-build
meson compile -C target/meson-build
meson compile -C target/meson-build xfce4-profile-editor-pot
```

UI changes also require loading the affected template and exercising the flow.
Core changes require confirming that no GTK/GDK imports have crossed into
`src/core/`.

### Manual acceptance matrix

- Fedora Xfce on X11.
- Fedora Xfce on Wayland when available.
- AccountsService present and absent/degraded.
- Authentication accepted, cancelled, and denied.
- Local account and detected non-local account.
- Webcam present, absent, busy, denied, and unplugged during preview.
- Light and dark GTK themes, HiDPI, keyboard-only navigation, and a small
  display.
- A release-candidate installation from the exact artifacts attached to the
  draft GitHub Release.

## 13. Early decisions still required

These decisions should be made before the relevant implementation phase, not
guessed during scaffolding:

- supported distribution/version matrix beyond Fedora;
- naming changes required by a future Xfce contribution.

## 14. Primary references

- AccountsService D-Bus API:
  <https://www.freedesktop.org/software/accountsservice/>
- GTK Rust composite templates:
  <https://gtk-rs.org/gtk4-rs/stable/latest/book/composite_templates.html>
- GTK Rust internationalisation and Meson integration:
  <https://gtk-rs.org/gtk4-rs/stable/latest/book/i18n.html>
- Nokhwa camera library:
  <https://github.com/l1npengtul/nokhwa>
- Existing Mugshot implementation for behavioural comparison only:
  <https://github.com/bluesabre/mugshot>
- GitHub Actions billing and included usage:
  <https://docs.github.com/en/billing/concepts/product-billing/github-actions>
- GitHub Actions budgets and hard usage stops:
  <https://docs.github.com/en/billing/how-tos/set-up-budgets>
- GitHub release management:
  <https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository>
