# Xfce Profile Editor

`xfce4-profile-editor` is a proposed Rust/GTK 4 utility for editing the current
user's system-backed profile information in Xfce.

The project is currently independent and is not an official Xfce component.
Its working name reflects the desktop it is intended to integrate with.

## Status

The repository is in the planning stage. See
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

The foundation requires Rust 1.92 or newer, GTK 4.10 development files,
`pkg-config`, `glib-compile-resources`, and either Dart Sass (`sass`) or
`sassc`.

Build and run it with:

```sh
cargo run
```

Run the workspace checks with:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gtk4-builder-tool validate ui/windows/profile-window.ui
```
