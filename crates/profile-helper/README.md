# Privileged Helper Boundary

This workspace package contains the GTK-independent shared protocol,
privileged preparation logic, and `xfce4-profile-editor-helper` executable.

The one-shot executable parses a bounded request, selects the calling UID from
the local passwd database, validates an update, preserves the complete GECOS
representation, runs only the fixed `usermod` command, and verifies the
resulting local account. Its request, response, size-limit, and exit-code types
are also used by the application-side client so the privilege boundary does
not have parallel protocol definitions.

Automated tests substitute an in-memory backend and must never execute the
system command. Application-side client tests substitute a fake process runner
and must never start `pkexec`. Real-account write tests require a separate,
explicitly approved disposable test environment.
