# Privileged Helper Boundary

This workspace package contains the GTK-independent protocol and preparation
logic for the future `xfce4-profile-editor-helper` executable.

The Phase 3c checkpoint adds the one-shot executable entry point and production
backend, but does not install or invoke them. It parses a bounded request,
selects the calling UID from the local passwd database, validates an update,
preserves the complete GECOS representation, runs only the fixed `usermod`
command, and verifies the resulting local account.

Automated tests substitute an in-memory backend and must never execute the
system command. Do not install the helper or Polkit policy, add real-account
write tests, or connect the GTK application to this executable without the
next explicit review described in `PROGRESS.md` and the security decision.
