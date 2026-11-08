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
