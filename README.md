# Xfce Profile Editor

`xfce4-profile-editor` is a proposed Rust/GTK 4 utility for editing the current
user's system-backed profile information in Xfce.

The project is currently independent and is not an official Xfce component.
Its working name reflects the desktop it is intended to integrate with.

## Status

The repository is under phased development. Profile reads, the narrowly
scoped GECOS persistence path, AccountsService email writes, and profile-image
selection/removal are implemented. Camera capture is implemented; its backend
has been validated on camera-equipped hardware and the window is awaiting final
manual acceptance. See
[IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for the agreed scope,
architecture, implementation phases, and acceptance criteria. See
[PROGRESS.md](PROGRESS.md) for the current implementation and handoff status.

The initial application will edit only information with an established system
store:

- profile image (`~/.face` and AccountsService)
- first and last name (the GECOS full-name field)
- office and home phone numbers (GECOS)
- email address (AccountsService)

Initials, fax details, and application-specific integrations are intentionally
out of scope.

## Licence

Xfce Profile Editor is licensed under the
[GNU General Public License, version 2 or later](LICENSE)
(`GPL-2.0-or-later`).

## Development

The foundation requires Rust 1.92 or newer, Meson 1.4 or newer, GTK 4.10
development files, GNU gettext, `pkg-config`, `glib-compile-resources`, and
either Dart Sass (`sass`) or `sassc`.

Configure, build, and run a development build with:

```sh
meson setup target/meson-build -Dprofile=development
meson compile -C target/meson-build
target/meson-build/src/xfce4-profile-editor
```

Camera support is enabled by default and can be omitted with Meson's
`-Dcamera=false` option. This excludes Nokhwa and its V4L2 dependency graph.

The repeatable developer workflow is also available through one script:

```sh
scripts/build-install-launch.sh build
scripts/build-install-launch.sh stage
scripts/build-install-launch.sh install
scripts/build-install-launch.sh launch
```

`build` and `stage` never request elevation. `install` stages and verifies the
release first, then uses `sudo install` to copy only the three verified files;
compilation and Meson metadata therefore remain unprivileged. `cycle` performs
the explicit install followed by launch. The script adapts compiler
parallelism to the current machine and never invokes the privileged profile
helper or changes account data.

Stage the native installation layout without root access with:

```sh
meson setup target/meson-release --prefix=/usr -Dprofile=release
DESTDIR="$PWD/target/staged-install" meson install -C target/meson-release
```

This builds release binaries and places the application, helper, and rendered
Polkit policy below `DESTDIR`. It does not register or activate the policy.
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
xmllint --noout data/polkit/io.github.camelofcode.Xfce4ProfileEditor.policy.in
```
