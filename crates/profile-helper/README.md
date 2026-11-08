# Privileged Helper Boundary

This workspace package contains the GTK-independent protocol and preparation
logic for the future `xfce4-profile-editor-helper` executable.

At the Phase 3b checkpoint it deliberately has no executable entry point and
cannot change an account. It can only parse a bounded request, select the
calling UID from supplied local-passwd data, validate an update, preserve the
complete GECOS representation, and produce a fixed `usermod` command
specification for tests.

Do not add command execution, Polkit installation, or real-account write tests
without the next explicit review described in `PROGRESS.md` and
`docs/phase-3-gecos-persistence-spike.md`.
