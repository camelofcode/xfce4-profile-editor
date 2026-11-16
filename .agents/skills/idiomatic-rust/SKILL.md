---
name: idiomatic-rust
description: Add, refactor, or review Rust abstractions in Xfce Profile Editor while keeping stateless work function-based, state explicit, errors typed, and visibility narrow.
---

<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Idiomatic Rust Structure

Use the smallest abstraction that expresses the current behaviour clearly.
Do not introduce object-like structure in anticipation of possible future
state or implementations.

## Choose functions, structs, and traits deliberately

- Use module-level functions for stateless operations. Do not create a unit or
  empty struct merely to provide a namespace for methods.
- Use a struct when instances carry data, enforce an invariant, own a resource,
  or have meaningful lifecycle/state. Methods should normally use that state;
  associated constructors and consuming accessors are appropriate.
- Give an independent production domain data carrier its own clearly named
  file within that domain. Do not split framework-required state holders,
  behaviour-heavy resource types, tightly coupled private errors, or
  incidental test-only structs solely to satisfy a one-type-per-file rule.
- Introduce a trait when polymorphism is part of the real design or several
  operations form a meaningful replaceable interface. Do not add a trait and a
  zero-sized production implementation solely to fake one function in tests.
- For a narrow internal test seam, prefer a generic callable (`impl Fn`,
  `FnMut`, or `FnOnce`) or pass the required plain value directly.
- Framework-required GTK subclass types and stateful test fakes are not
  namespace structs; retain them when they own real state or satisfy a required
  framework contract.

## Model values and failures precisely

- Use an enum for a closed set of alternative states or failure categories.
  Use a struct for one consistently shaped value with meaningful fields.
- Preserve underlying errors with `#[source]` or `#[from]` where useful. Do not
  flatten distinct causes into an untyped message and recreate the distinction
  with constructor functions.
- Avoid wrapper structs that only transport one value unless the wrapper adds
  a domain invariant, prevents misuse, or is expected by an external API.
- Keep public core errors locale-independent; translate them into user-facing
  messages in `src/ui/`.

## Keep the surface small

- Give items the narrowest practical visibility. A function used only by its
  module and tests should remain private.
- Do not export a type or function solely because it may be useful later.
- Before retaining an abstraction during a scope reduction, trace every
  production caller and remove fields, variants, modules, and dependencies
  whose only consumer was removed functionality.
- Prefer domain-specific module names over generic `helpers` or `utils`.

## Review before completion

For a structural Rust change:

1. Trace new and changed types through all production call sites and tests.
2. Search for unit structs, single-use traits, unused fields, broad
   `pub(crate)` exports, one-field wrappers, and message-only error structs.
3. Confirm every remaining struct carries state/data or fulfills a real
   framework contract, and every remaining trait represents a useful
   abstraction boundary.
4. Run formatting, warnings-denied Clippy, and the relevant feature-enabled
   and feature-disabled tests under the repository's resource-aware settings.
