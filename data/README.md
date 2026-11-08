# Installation Layout

The eventual native package installs these files in one transaction:

```text
@bindir@/xfce4-profile-editor
@libexecdir@/xfce4-profile-editor-helper
@datadir@/polkit-1/actions/io.github.camelofcode.Xfce4ProfileEditor.policy
```

The helper must be owned by root and executable but must not have a setuid bit.
The policy is generated from `data/polkit/*.policy.in` by replacing
`@libexecdir@` with the package's absolute libexec directory. The package must
depend on Polkit, AccountsService, and the provider of `/usr/sbin/usermod`.

No Polkit JavaScript rule is installed. Distribution administrators retain the
ability to override the declared action using their normal Polkit rules.

For package builds, configure Meson with `-Dprofile=release` and the intended
prefix, then run `meson install` with a non-empty `DESTDIR`. Meson builds both
release binaries, installs them into this layout, and renders the policy with
the configured absolute `libexecdir` without writing outside the staging
directory.

The installed policy declares the `xfce4-profile-editor` gettext domain so the
system Polkit agent can translate its description and authentication message.
The matching extraction markers in `po/polkit-messages.rs` are kept in sync by
an automated test.
