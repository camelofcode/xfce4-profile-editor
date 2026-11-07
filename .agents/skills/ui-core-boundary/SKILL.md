---
name: ui-core-boundary
description: Add or change Xfce Profile Editor features that cross GTK UI plumbing and system-facing core logic while preserving the repository's strict layer boundary.
---

# UI/Core Boundary

Read `IMPLEMENTATION_PLAN.md` and inspect the nearest related UI template, UI
module, core service, resource entry, and test before editing.

## Assign ownership first

Classify every new responsibility before writing code:

- Put widget hierarchy, spacing, labels, menus, accessibility relationships,
  and static properties in `ui/**/*.ui`.
- Put visual rules in `ui/sass/`; Rust may only toggle semantic state classes.
- Put template binding, signal/action wiring, window lifecycle, and rendering
  of core results in `src/ui/`.
- Put models, validation, D-Bus, GECOS, helper communication, filesystem work,
  image transformation, and camera capture in `src/core/`.

Dependency direction is `ui -> core`. Core APIs return plain Rust values and
typed errors; they must not expose or import GTK/GDK types. If a responsibility
does not have a clear owner, resolve the boundary explicitly instead of placing
it in a catch-all helper module.

## Implement the complete flow

- Define or extend the core model and service contract before connecting GTK
  callbacks.
- Keep blocking and device work off the GTK main thread. Make cancellation and
  object lifetime explicit.
- Let the UI represent loading, partial success, cancellation, and recoverable
  errors without duplicating core policy.
- Preserve unedited system data, especially hidden GECOS fields, at every write
  boundary.
- Update `.ui`, SCSS, GResource/build inputs, packaging metadata, and tests when
  the feature touches them.

## Verify the boundary

Run formatting, Clippy, and focused tests, followed by the full applicable test
suite. Load every changed template and exercise the UI flow with a display or
supported headless backend. Search `src/core/` for GTK/GDK imports and inspect
the final diff for business logic left in signal handlers or layout recreated
in Rust.
