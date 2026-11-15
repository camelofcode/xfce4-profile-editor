<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Whisker-focused Initial Scope

Status: implemented in the current checkpoint

Date: 2026-11-15

## Decision

The initial Xfce Profile Editor edits only the current user's display name and
profile image. These are the values presented in Whisker Menu's profile header,
and clicking that header can be configured to launch this application.

Office phone, home phone, email, login name, and password are not required for
that integration. Shipping them would make a small profile editor behave like
an incomplete account-administration tool before the Xfce team has provided
feedback on whether those features belong.

## Persistence consequence

Display-name updates use AccountsService `SetRealName`. Profile images continue
to use atomic `~/.face` updates plus AccountsService `SetIconFile`.

This removes the project's lossless GECOS helper, shared GECOS crate, helper
crate, JSON protocol, `pkexec` invocation, and Polkit policy. AccountsService
may still apply distribution policy and present the desktop's system
authentication prompt.

AccountsService owns the legacy account-comment representation behind
`SetRealName`. The application does not expose or manage room, phone, or other
GECOS fields. This is a deliberate reduction in contract, aligned with the
standard desktop API used by comparable account-details tools.

## Recovery point

The complete working implementation of the broader fields and privileged
helper is preserved in the pushed annotated tag:

```text
archive/full-profile-fields-2026-11-15
```

The tag is a historical recovery point, not a release. Removed features should
be reconsidered only after upstream feedback establishes a useful Xfce-facing
requirement.
