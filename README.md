# Xfce Profile Editor

[![CI](https://github.com/camelofcode/xfce4-profile-editor/actions/workflows/ci.yml/badge.svg)](https://github.com/camelofcode/xfce4-profile-editor/actions/workflows/ci.yml)

Xfce Profile Editor is an independent Rust/GTK 4 app for editing your desktop
profile. The maintainer has tested version 0.1 extensively on Xfce with Whisker
Menu and on Cinnamon.

## Features

- Change your display name.
- Choose a profile photo from a file, take one with your webcam, or remove your
  current photo.

Your name and photo appear in Whisker Menu and wherever else your desktop shows
your system profile, including login and lock screens.

Version 0.1 does not offer direct editing of `/etc/passwd` GECOS fields or
password changes. User feedback will guide future features.

## Xfce integration

The installed `.desktop` file adds a **Profile Editor** launcher to the
**Personal** section of Xfce Settings. To open it from Whisker Menu, set the
menu’s **Edit Profile** command to `xfce4-profile-editor`. The package also
includes a scalable application icon and AppStream metadata.

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

To test the installed desktop integration, run
`scripts/build-install-launch.sh install` and open **Profile Editor** from Xfce
Settings or Whisker Menu. This launches `/usr/bin/xfce4-profile-editor`. Use
`cycle` when an immediate launch is more convenient. Paths below `target/` are
internal build and staging artifacts.

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

## Release packages

The tag-triggered GitHub Actions release workflow builds x86_64 `.deb`, `.rpm`,
and `linux-x86_64.tar.xz` artifacts from the same verified Meson staging tree.
It always creates a draft release for manual package and hardware acceptance;
publishing remains an explicit maintainer action. See
[docs/releasing.md](docs/releasing.md) for versioning, manual test builds, tag
creation, and release approval.

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
