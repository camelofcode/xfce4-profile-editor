# Phase 3 GECOS Persistence Spike

Status: approved  
Date: 2026-11-08

## Decision

Use a small installed Rust helper invoked through `pkexec` for name and phone
updates. The helper will accept no UID or username, derive the calling UID from
the `PKEXEC_UID` value supplied by `pkexec`, require that UID to have exactly
one local `/etc/passwd` entry, reconstruct the complete GECOS value itself, and
invoke `/usr/sbin/usermod` without a shell.

Email remains an AccountsService property and will be written through
`org.freedesktop.Accounts.User.SetEmail`. It must not be copied into GECOS.

This checkpoint documents the privileged boundary only. It does not install a
helper, request authorization, or modify any account.

## Compatibility evidence

The spike was performed on Fedora 44 with:

```text
accountsservice 23.13.9-16.fc44
polkit 127-2.fc44.2
shadow-utils 4.19.0-7.fc44
util-linux 2.41.6-1.fc44
```

The machine exposes `SetRealName` and `SetEmail` on
`org.freedesktop.Accounts.User`. Its own-user AccountsService action is
currently implicitly allowed for an active session.

### Why not AccountsService `SetRealName`

AccountsService 23.13.9 implements `SetRealName` by running `usermod -c` with
only the supplied real name. `usermod -c` replaces the complete GECOS/comment
value. Using this method would therefore discard room, phone, other, and any
site-specific fields. It does not meet this project's preservation rule.

Source:

- [AccountsService 23.13.9 source tree](https://cgit.freedesktop.org/accountsservice/tree/src/user.c?h=23.13.9)
- [AccountsService change showing the `usermod -c` real-name path](https://cgit.freedesktop.org/accountsservice/commit/?id=224b7e93a27a1ab5cf2eec2f56bc3adafd02e7af)
- [shadow-utils `usermod` source](https://github.com/shadow-maint/shadow/blob/master/src/usermod.c)

### Why not `chfn`

`chfn` has useful field-specific options, but its implementation parses only
the five traditional GECOS fields, retains only one `other` field, and removes
trailing empty fields when it writes. That can discard extra site-specific
fields and changes the byte representation of short or trailing-empty data.
Fedora's current `CHFN_RESTRICT` configuration also disallows direct changes
for the development account.

Source:

- [util-linux `chfn` source](https://github.com/util-linux/util-linux/blob/v2.41.6/login-utils/chfn.c)

### Why `pkexec`

`pkexec` uses the desktop session's registered Polkit authentication agent, so
Xfce can present its standard elevation dialog. It gives the executed program
a minimal environment and sets `PKEXEC_UID` to the UID of the invoking
process. Exit status 126 represents a dismissed authentication dialog; 127
represents failure to obtain authorization or another `pkexec` error.

Source:

- [Polkit `pkexec` reference](https://polkit.pages.freedesktop.org/polkit/pkexec.1.html)

## Privileged boundary

The installed helper should have one purpose and one request type:

```text
xfce4-profile-editor-helper update-gecos
```

The unprivileged application starts it with an absolute path through `pkexec`,
writes one size-limited JSON request to standard input, closes the pipe, and
reads one size-limited JSON response. The request contains:

```text
version
expected_gecos
full_name
office_phone
home_phone
```

It deliberately contains no username, UID, file path, executable path, or
arbitrary operation. Values are not placed in a shell command string.

The helper performs these steps in order:

1. Require effective UID 0, parse `PKEXEC_UID` as a numeric UID, and reject
   UID 0.
2. Read `/etc/passwd` directly and find exactly one local entry with that UID.
   Do not use NSS here: an LDAP or other remote account must be rejected.
3. Require the current local GECOS string to equal `expected_gecos`. A mismatch
   is a conflict and performs no write.
4. Validate the three editable values again. Reject commas, colons, newlines,
   NUL characters, control characters, and overlong values.
5. Parse the current GECOS losslessly. Replace only full name (field 0), office
   phone (field 2), and home phone (field 3), padding missing intervening fields
   only when an edited value requires it. Preserve every existing unexposed or
   extra field exactly.
6. Run `/usr/sbin/usermod --comment NEW_GECOS -- USERNAME` with
   `std::process::Command`, a cleared environment, explicit standard-I/O
   handling, captured output, and no shell.
7. Re-read `/etc/passwd` and verify the requested fields changed and every
   unexposed field still matches. Return a structured result.

The helper must reject oversized input before allocating an unbounded buffer.
It must never accept a target account, edit `/etc/passwd` itself, launch a
configurable program, or fall back to a remote NSS account.

## Polkit policy

Install a dedicated action such as
`io.github.camelofcode.Xfce4ProfileEditor.modify-gecos` with:

```text
allow_any:      no
allow_inactive: no
allow_active:   auth_admin
```

Bind that action to the exact installed helper path using
`org.freedesktop.policykit.exec.path`. Do not use `auth_admin_keep`, do not
allow GUI environment variables, and ensure the helper is root-owned and not
writable by ordinary users.

The policy prompt should say that authentication is required to change the
current user's name and phone numbers. The application never displays or
collects a password itself.

## Result model

The unprivileged core maps outcomes to distinct states:

- success;
- validation failure;
- stale-data conflict (reload without losing the user's edit);
- non-local account;
- authentication cancelled (`pkexec` 126);
- authorization or elevation failure (`pkexec` 127);
- account tool/system failure;
- malformed or oversized helper response.

Raw helper or `usermod` output is for diagnostics and must not be displayed as
trusted markup.

## Residual risks and constraints

- `usermod` owns the password-file locking and atomic replacement, but it has
  no compare-and-swap option. A different administrator could alter GECOS
  between the helper's precondition check and `usermod`. The helper reduces
  this window by doing no intervening work, then detects unexpected results in
  its post-write verification. This residual race must be called out in the
  security review; inventing a second password-file writer would be riskier.
- Authorization success does not imply that the account is local or writable.
  Those checks remain mandatory inside the elevated helper.
- Every validation rule is duplicated at the privileged boundary. UI-side
  validation exists for usability, not security.
- The helper and policy are Linux packaging components and must be installed
  to fixed paths before persistence can work.

## Required tests before enabling editing

1. Unit-test request size limits, parsing, validation, and result mapping.
2. Unit-test lossless replacement for short, empty, trailing-empty, standard,
   and extra-field GECOS strings.
3. Unit-test that the command specification has a fixed executable, `--`
   terminator, derived username, and no shell.
4. Test missing, duplicate, remote-only, UID 0, and stale local accounts.
5. Test helper execution against a disposable local account in an isolated VM
   or image; never use the developer's real account for automated write tests.
6. Verify success, denial, cancelled prompt, malformed input, tool failure,
   and post-write mismatch.
7. Confirm room, `other`, empty, and extra fields remain byte-for-byte unchanged
   after each successful write case.

Implementation of the helper should stop for another review before it is
installed or invoked against a real account.
