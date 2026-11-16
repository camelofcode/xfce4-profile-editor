<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Xfce Profile Editor Implementation Plan

## 1. Purpose

Build a small Rust/GTK 4 application for the command launched by Whisker
Menu's **Edit Profile** action. The application edits exactly the two values
shown in Whisker Menu's profile header: the current user's display name and
profile image.

Working identifiers:

- repository and package: `xfce4-profile-editor`
- executable: `xfce4-profile-editor`
- window title: `Profile`
- application ID: `io.github.camelofcode.Xfce4ProfileEditor`
- licence: `GPL-2.0-or-later`

The project is initially an independent proposal, not an official Xfce
component. The first useful release should remain deliberately small so it can
be presented to the Xfce team for feedback before broader account-management
features are considered.

## 2. Scope

### In scope

- Display the current user's real/display name.
- Update the display name through AccountsService `SetRealName`.
- Display the current profile image as a circular image.
- Select, normalise, and remove a profile image.
- Take a profile image using an available webcam.
- Keep `~/.face` and AccountsService `IconFile` in sync.
- Provide a buttonless edit flow with a Close button rather than Save and
  Cancel.
- Work under X11 and Wayland in an Xfce session.

### Out of scope

- Office phone, home phone, email, initials, fax, and other personal details.
- Editing another user's account.
- Password, login name, shell, groups, account type, or automatic-login
  management.
- Direct `/etc/passwd` or GECOS persistence.
- A project-owned privileged helper or Polkit policy.
- LibreOffice, Pidgin, or other application-specific synchronisation.
- Stock avatar galleries in the first release.
- A compatibility executable named `mugshot`, which could conflict with the
  existing package.

The broader pre-simplification implementation remains available in the Git tag
`archive/full-profile-fields-2026-11-15` if later Xfce feedback justifies
revisiting any removed feature.

## 3. Data ownership

There is one authoritative system interface for each editable value.

| UI value | Authoritative storage/interface | Notes |
| --- | --- | --- |
| Display name | AccountsService `RealName` / `SetRealName` | Editing is disabled when AccountsService is unavailable. |
| Profile image | `~/.face` and AccountsService `IconFile` | Keep both Whisker-facing consumers in sync. |

AccountsService is distribution-provided and may apply its own Polkit policy.
The application requests interactive authorization for writes but does not
ship a policy, invoke `pkexec`, collect a password, or promise that every
distribution will authorize a write without a system prompt.

`SetRealName` delegates the legacy GECOS representation to AccountsService.
The application does not expose or manage legacy room/phone/other fields. This
is a deliberate scope tradeoff in favour of the standard desktop account API
used by comparable account-details tools.

No application-specific GSettings profile store is needed.

## 4. Toolkit and dependencies

- Rust using the stable toolchain supported by the selected `gtk4-rs` release.
- GTK 4.10 or newer.
- Resource-backed `gtk4-rs` composite templates rather than Relm4.
- Meson as the supported build/install entry point; Cargo remains the Rust
  compiler and workspace test runner.
- GResource for UI templates, compiled CSS, and static assets.
- SCSS for the small amount of avatar and status styling.
- GNU gettext for every user-visible string.
- `zbus` for GTK-independent AccountsService access.
- `nokhwa` with the native Linux backend for optional webcam capture.
- `image` for bounded orientation, square cropping, resizing, and PNG encoding.

`scripts/build-install-launch.sh` is the authoritative developer entry point
for application builds, staging, installation, and launching. Direct Cargo
commands are reserved for formatting, linting, and tests and reuse the same
Cargo home and workspace `target/` cache. There is no parallel Makefile path.

## 5. Repository layout

```text
.
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── AGENTS.md
├── Cargo.toml
├── IMPLEMENTATION_PLAN.md
├── PROGRESS.md
├── build.rs
├── data/
│   ├── icons/
│   ├── io.github.camelofcode.Xfce4ProfileEditor.desktop.in.in
│   ├── io.github.camelofcode.Xfce4ProfileEditor.metainfo.xml
│   └── meson.build
├── meson.build
├── meson_options.txt
├── resources.gresource.xml
├── docs/
│   └── releasing.md
├── packaging/
│   ├── debian/
│   └── rpm/
├── po/
├── scripts/
│   ├── build-install-launch.sh
│   ├── cargo-build.sh
│   └── package-release.sh
├── src/
│   ├── core/
│   │   ├── camera/
│   │   │   ├── capture.rs
│   │   │   ├── device.rs
│   │   │   ├── frame.rs
│   │   │   └── mod.rs
│   │   ├── profile/
│   │   │   ├── accounts.rs
│   │   │   ├── avatar_persistence.rs
│   │   │   ├── current_user_lookup.rs
│   │   │   ├── display_name.rs
│   │   │   ├── image.rs
│   │   │   ├── loader.rs
│   │   │   ├── models/
│   │   │   │   ├── account_details.rs
│   │   │   │   ├── avatar.rs
│   │   │   │   ├── avatar_save_outcome.rs
│   │   │   │   ├── current_user.rs
│   │   │   │   ├── display_name_save_outcome.rs
│   │   │   │   ├── load_warning.rs
│   │   │   │   ├── loaded_profile.rs
│   │   │   │   └── mod.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   └── ui/
│       └── windows/
│           ├── camera_window/
│           │   ├── capture.rs
│           │   ├── mod.rs
│           │   └── preview.rs
│           └── profile_window/
│               ├── avatar.rs
│               ├── camera.rs
│               ├── display_name.rs
│               ├── loading.rs
│               ├── mod.rs
│               └── status.rs
└── ui/
    ├── sass/
    └── windows/
        ├── camera-window.ui
        └── profile-window.ui
```

Static hierarchy, spacing, labels, accessibility relationships, and widget
properties belong in `ui/`. Styling belongs in `ui/sass/`. GTK signal wiring,
window lifecycle, and presentation state belong in `src/ui/`. AccountsService,
current-process identity, image transformation, camera capture, validation,
and persistence belong in `src/core/`.

Core code is grouped by product domain rather than technical layer. The
`profile` domain keeps its data and AccountsService/filesystem behaviour
together; the `camera` domain owns device discovery, frame capture, and its
worker lifecycle. The larger profile domain groups its independent production
data carriers under `profile/models/`, one named file per value; stateful
resources, persistence operations, and their tightly coupled errors stay in
behaviour-focused modules.
`src/core/mod.rs` is the single façade consumed by the UI.

Dependency direction is always `src/ui/ -> src/core/`. Core APIs use plain
Rust values and typed errors and must not import GTK or GDK.

Each window remains one GTK subclass. Its module root owns template binding,
signal connections, lifecycle hooks, and construction. Named profile-window
modules contain the loading, display-name, avatar, camera, and status flows;
named camera-window modules separate capture-worker coordination from frame
preview interactions. These are source-navigation boundaries, not additional
controller objects.

## 6. Core loading

The load result remains small:

```rust
struct LoadedProfile {
    display_name: String,
    avatars: Vec<Avatar>,
    warnings: Vec<LoadWarning>,
    display_name_state: Option<DisplayNameState>,
}
```

Loading runs outside the GTK main thread:

1. Resolve the effective UID from the kernel and the home directory from the
   desktop session's absolute `HOME` value.
2. Read AccountsService `RealName` and `IconFile`.
3. Keep name editing disabled and report a recoverable warning if
   AccountsService is unavailable; do not parse passwd/GECOS as a fallback.
4. Prefer a valid `~/.face`, then a valid AccountsService icon, then the themed
   default avatar.
5. Report degraded AccountsService and unreadable-image states as recoverable
   warnings.

Display names are trimmed before comparison and saving. AccountsService owns
the accepted-value policy and reports unsupported values as recoverable write
errors; the application does not impose passwd/GECOS formatting rules.

## 7. Main window and save flow

Use a compact, fixed-size `GtkApplicationWindow` containing a
`GtkScrolledWindow`. The fixed 460×360 default comfortably contains the avatar,
single field, status message, and footer while the scroller protects the flow
on constrained displays and with longer translations.

```text
             (circular profile image)

Display name:  [                              ]

                                             [ Close ]
```

Activating the image opens a popover with Choose photo, Take photo, and Remove
photo. Take photo is hidden when camera support is unavailable.

There is no Save or Cancel button:

- mark the display name dirty while typing;
- show a concise unsaved state after a short idle debounce or focus loss,
  without writing on every keystroke or blur;
- save once through AccountsService when the window closes;
- keep the window open and retain the edit if the write fails;
- keep all D-Bus and image work off the GTK main thread.

Image selection and removal persist immediately because they are explicit
actions, not ambiguous form edits.

## 8. Profile images

Image processing runs outside the GTK main thread:

1. Open GTK's asynchronous file dialog filtered to supported image types.
2. Decode with bounded input, dimension, and allocation limits.
3. Apply embedded orientation.
4. Centre-crop to a square and resize to 512 by 512 pixels.
5. Encode a normalised PNG.
6. Write beside `~/.face`, set mode `0644`, synchronise, and atomically rename.
7. Call AccountsService `SetIconFile` with the stable `~/.face` path and
   interactive authorization enabled.
8. Update the preview only from the confirmed result and report partial
   AccountsService failure explicitly.

## 9. Webcam window

Camera support remains a separate core service and `.ui`-defined window.

- Enumerate devices rather than assuming `/dev/video0`.
- Open and capture on worker threads.
- Use a bounded latest-frame channel.
- Offer Capture, Retake, Use photo, and Cancel states.
- Feed accepted frames through the same normalisation and persistence path.
- Release the device on every exit path.
- Treat unavailable, busy, denied, unsupported, and unplugged devices as
  recoverable errors.

## 10. Implementation phases

### Phase 1: project foundation — complete

Cargo/Meson, GTK resources, SCSS, the main window, formatting, linting, tests,
and resource validation are established.

### Phase 2: profile reads — complete

Current-user resolution, AccountsService reads, avatar fallback, asynchronous
loading, warnings, and the read-only UI are established.

### Phase 3: display-name persistence — current simplification checkpoint

- Replace first/last name, phone, and email editing with one display-name field.
- Write through AccountsService `SetRealName` only.
- Remove the helper, shared GECOS crate, helper crate, JSON protocol, project
  Polkit policy, and their package artifacts.
- Retain normalization, dirty-state, close-time save, and recoverable errors.

Completion: the installed application changes the Whisker-visible display name
and requires no project-owned privileged component.

### Phase 4: image selection — complete

Bounded normalisation, asynchronous file selection, atomic `~/.face` writes,
AccountsService updates, preview, and removal are implemented.

### Phase 5: camera capture — complete for primary hardware

Discovery, capture window, bounded frame pipeline, and avatar-save integration
are implemented and have passed primary camera-equipped hardware acceptance.
Busy, denied, and unplugged-device resilience checks remain desirable.

### Phase 6: desktop integration and packaging — in progress

- Install a translated **Profile Editor** launcher in Xfce's personal Settings
  category, together with its scalable icon and translated AppStream metadata.
- Document configuring Whisker Menu's Edit Profile command as
  `xfce4-profile-editor` and validate the complete staged desktop layout.
- Add Fedora build/install instructions and prepare portable distro packaging
  guidance.

Completion: a clean package install appears where appropriate, launches from
Whisker, updates the displayed name/avatar, and passes metadata validation.

### Phase 7: CI and release automation

Keep one cost-conscious Linux validation job, cancel superseded runs, avoid
scheduled and matrix jobs initially, use read-only token permissions, and
retain temporary artifacts for no more than seven days. A tag-triggered
release job may request `contents: write`, but it must create a draft release
for manual verification and must never publish directly.

The initial release build uses one Ubuntu runner and one verified Meson staging
tree to produce x86_64 Debian, RPM, and compressed `/usr`-tree artifacts plus
SHA-256 checksums. Manual workflow runs retain read-only test artifacts; only a
matching `vMAJOR.MINOR.PATCH` tag on `main` starts the short draft-release job.

## 11. Verification

Automated coverage includes:

- AccountsService-backed display-name state and fake-client persistence;
- current-process UID/home resolution and degraded AccountsService handling;
- image orientation, crop, resize, size limits, atomic write, and removal;
- resource loading and display-backed window lifecycle;
- camera discovery and fake-frame capture paths;
- Meson build and staged installation containing the application binary,
  desktop entry, scalable icon, and AppStream metadata.

Required checks before committing a checkpoint:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
GSK_RENDERER=cairo G_DEBUG=fatal-warnings cargo test --workspace --all-features
gtk4-builder-tool validate ui/windows/profile-window.ui
gtk4-builder-tool validate ui/windows/camera-window.ui
meson compile -C target/meson-build xfce4-profile-editor-pot
msgfmt --check --check-accelerators=_ po/xfce4-profile-editor.pot
scripts/build-install-launch.sh stage
```

UI changes also require exercising the installed flow. Core changes require
confirming that GTK/GDK imports have not crossed into `src/core/`.

Manual acceptance targets Fedora Xfce on X11 first, then Wayland when
available, followed by other distributions based on contributor feedback.
Test AccountsService present/degraded, image selection/removal, camera
present/absent/busy/denied, light/dark themes, HiDPI, keyboard navigation, and
small displays.

## 12. Early decisions still required

- supported distribution/version matrix beyond Fedora;
- naming or application-ID changes requested by an Xfce contribution review;
- whether any post-v1 account feature is justified by upstream feedback.

## 13. Primary references

- AccountsService: <https://www.freedesktop.org/software/accountsservice/>
- GTK Rust composite templates:
  <https://gtk-rs.org/gtk4-rs/stable/latest/book/composite_templates.html>
- GTK Rust internationalisation and Meson integration:
  <https://gtk-rs.org/gtk4-rs/stable/latest/book/i18n.html>
- Nokhwa camera library: <https://github.com/l1npengtul/nokhwa>
- Mugshot behavioural reference: <https://github.com/bluesabre/mugshot>
- Whisker Menu: <https://docs.xfce.org/panel-plugins/xfce4-whiskermenu-plugin/start>
