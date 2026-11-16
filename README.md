# Xfce Profile Editor

[![CI](https://github.com/camelofcode/xfce4-profile-editor/actions/workflows/ci.yml/badge.svg)](https://github.com/camelofcode/xfce4-profile-editor/actions/workflows/ci.yml)

`xfce4-profile-editor` is a proposed Rust/GTK 4 utility for editing the current
user's system-backed profile information in Xfce.

The project is currently independent and is not an official Xfce component.
Its working name reflects the desktop it is intended to integrate with.

## Status

The repository is a functional work in progress focused on Whisker Menu's
profile header. Display-name editing, profile-image selection/removal, and
camera capture are implemented. The complete camera flow has been manually
validated on a camera-equipped Cinnamon/X11 workstation, outside the Xfce
desktop it is intended to integrate with. A translated desktop launcher,
application icon, and AppStream metadata are provided for Xfce integration.
Remaining work is primarily packaging, interface polish, and additional
resilience checks. See
[IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for the agreed scope,
architecture, implementation phases, and acceptance criteria. See
[PROGRESS.md](PROGRESS.md) for the current implementation and handoff status.

The initial application will edit only information with an established system
store:

- profile image (`~/.face` and AccountsService)
- real/display name (AccountsService)

Phone numbers, email, password, login name, and application-specific
integrations are intentionally out of scope for the initial release. The
earlier broader implementation is preserved in the Git tag
`archive/full-profile-fields-2026-11-15` for possible future reference.

## Licence

Xfce Profile Editor is licensed under the
[GNU General Public License, version 2 or later](LICENSE)
(`GPL-2.0-or-later`).

Copyright © 2026 [Chris Clayton](mailto:chris@chrisclayton.dev). See the
[copyright notice](COPYRIGHT) for details.

## Contributing

Code, documentation, testing, design feedback, and translations are welcome.
Translators are especially encouraged to help make the application useful
across the Xfce community. See [CONTRIBUTING.md](CONTRIBUTING.md) for the
development and translation workflows.

## Development

The foundation requires Rust 1.92 or newer, Meson 1.4 or newer, GTK 4.10
development files, GNU gettext, `pkg-config`, `glib-compile-resources`, and
either Dart Sass (`sass`) or `sassc`.

Use the authoritative developer workflow to configure and compile a
development build without installing it:

```sh
scripts/build-install-launch.sh build
```

Camera support is enabled by default and can be omitted with Meson's
`-Dcamera=false` option. This excludes Nokhwa and its V4L2 dependency graph.

The same script owns every application build, staged installation, explicit
system installation, and launch:

```sh
scripts/build-install-launch.sh build
scripts/build-install-launch.sh stage
scripts/build-install-launch.sh install
scripts/build-install-launch.sh launch
```

`build` and `stage` never request elevation. `install` stages and verifies the
release first, then uses `sudo meson install --no-rebuild` to install the
already-built application and desktop metadata; compilation therefore remains
unprivileged.
`cycle` performs the explicit install followed by launch. The script adapts
compiler parallelism to the current machine. Repository build scripts,
including the Meson-to-Cargo driver, are kept together under `scripts/`.

For normal desktop acceptance, run `scripts/build-install-launch.sh install`.
The application then appears as **Profile Editor** in Xfce Settings and may be
launched through Xfce's configured profile action. Set Whisker Menu's **Edit
Profile** command to `xfce4-profile-editor`. This tests the authoritative
`/usr/bin/xfce4-profile-editor` installation. Use `cycle` when an immediate
launch is more convenient. Paths below `target/` are internal build and staging
artifacts and are not part of the normal launch workflow.

All script, Meson, lint, and test commands reuse Cargo's normal dependency
sources and the workspace `target/` artifact cache. Do not assign a temporary
`CARGO_HOME` or `CARGO_TARGET_DIR`: changing either path prevents Cargo from
reusing compatible artifacts. Development/test and release artifacts remain
separate because Rust compiles their dependencies with different compiler
settings, but each configuration is cached after its first build.

Stage the native installation layout without root access with:

```sh
scripts/build-install-launch.sh stage
```

This builds the release binary and places the application below `DESTDIR`.
Meson honours standard installation-directory options for package builds. Do
not run an unstaged system installation during development.

All user-visible messages are gettext-enabled. Refresh the translation template
after changing strings with:

```sh
meson compile -C target/meson-build xfce4-profile-editor-pot
```

Run the workspace checks with:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gtk4-builder-tool validate ui/windows/profile-window.ui
```
