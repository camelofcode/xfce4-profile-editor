<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Contributing

Contributions to Xfce Profile Editor are welcome. The project is an independent
work in progress rather than an official Xfce component, so bug reports,
testing on different Linux systems, interface feedback, documentation, code,
and translations are all valuable.

By submitting a contribution, you agree to distribute it under the project's
GPL-2.0-or-later licence. You retain copyright in your contribution.

## Development workflow

Read [README.md](README.md) for dependencies and build commands, and
[AGENTS.md](AGENTS.md) for the architectural and verification rules. Keep UI
layout in GtkBuilder files, presentation in SCSS, GTK plumbing in `src/ui/`,
and system/data behaviour in `src/core/`.

Before proposing a change, run the applicable checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
GSK_RENDERER=cairo G_DEBUG=fatal-warnings cargo test --workspace --all-features
gtk4-builder-tool validate ui/windows/profile-window.ui
gtk4-builder-tool validate ui/windows/camera-window.ui
```

Push the change to a topic branch and open a pull request. Direct pushes to
`main` are blocked for maintainers as well as contributors. GitHub must report
the required **Build, test, and stage** check as passing on an up-to-date branch
before the pull request can be merged.

Changes to user-visible text must remain translatable. Static GtkBuilder text
uses `translatable="yes"`; runtime presentation text uses `gettext` in the UI
layer.

## Translating

Translations are especially welcome. No Rust or GTK experience is required.

To refresh the translation template after user-visible text changes:

```sh
scripts/build-install-launch.sh build
meson compile -C target/meson-build xfce4-profile-editor-pot
```

Meson and xgettext regenerate the message list and project-controlled header
fields. Review the resulting diff and retain the checked-in template title,
copyright year, and maintainer attribution at the top of the POT file.

To begin a translation, replace `ll_CC` with the appropriate locale:

```sh
msginit \
  --input=po/xfce4-profile-editor.pot \
  --locale=ll_CC \
  --output-file=po/ll_CC.po
```

Add the locale code to `po/LINGUAS`, translate the `msgstr` entries in the new
PO file, and keep your name and contact details in that file's translator
header. `msginit` replaces the template's source-language metadata with the
selected translation locale. Existing translations can be merged with the
latest template using:

```sh
meson compile -C target/meson-build xfce4-profile-editor-update-po
```

Validate a translation before submitting it:

```sh
msgfmt --check --check-accelerators=_ \
  --output-file=/tmp/xfce4-profile-editor.mo \
  po/ll_CC.po
```

Translation contributors are credited through the PO header and Git history.
Questions can be raised through GitHub Issues or sent to
[Chris Clayton](mailto:chris@chrisclayton.dev).
