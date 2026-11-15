<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Installation Layout

The native package installs the application at:

```text
@bindir@/xfce4-profile-editor
```

Profile image and display-name updates use the current user's files and the
distribution-provided AccountsService interface. The project does not install
a privileged helper or its own Polkit policy.

For package builds, configure Meson with `-Dprofile=release` and the intended
prefix, then run `meson install` with a non-empty `DESTDIR`. Developers can use
`scripts/build-install-launch.sh stage` for this safe staged workflow. Its
explicit `install` command copies the verified application with `sudo install`
and removes the two obsolete helper/policy files from pre-simplification
development installations when present.
