---
name: resource-aware-builds
description: Choose responsive or high-throughput settings before compile-heavy Cargo or Meson work, based on the current Linux machine's CPU and memory.
---

<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Resource-Aware Builds

Before the first compile-heavy Cargo or Meson command in a session, run:

```sh
sh .agents/skills/resource-aware-builds/scripts/detect-build-mode.sh
```

Treat `cargo check`, `build`, `clippy`, and `test`, plus Meson compile and
install commands that invoke Cargo, as compile-heavy. Formatting, metadata
inspection, gettext extraction, and read-only checks do not require a mode.

Apply the reported mode for the rest of the session unless the machine or its
available resources change materially. Briefly tell the user which mode is in
use the first time a substantial build starts.

## Modes

- `responsive`: Prefix compile-heavy commands with
  `CARGO_BUILD_JOBS=1 nice -n 10`. Run them sequentially and use development
  builds unless the current checkpoint specifically requires a release
  artifact. This mode prioritises keeping the desktop and browser usable.
- `balanced`: Set `CARGO_BUILD_JOBS` to the reported `jobs` value and use
  `nice -n 5`. Avoid concurrent compiler-heavy commands.
- `unrestricted`: Let Cargo use its default parallelism and normal priority.
  Parallelise independent work when it will not contend for Cargo's target
  lock or obscure failures.

The user's explicit preference for responsiveness or throughput overrides the
detected mode.

## Preserve useful work

- Reuse the workspace `target/` cache across direct Cargo and Meson builds.
  Do not create a second target directory merely to isolate build front ends.
- Do not run `cargo clean`, delete a target directory, install compiler tools,
  or change system configuration unless the user has approved it.
- Do not reduce verification coverage because a machine is constrained.
  Throttle or defer expensive checks instead.
- Keep machine-specific job limits out of committed Cargo configuration so
  capable workstations and CI remain free to use their resources.
