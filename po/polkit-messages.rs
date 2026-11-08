// SPDX-License-Identifier: GPL-2.0-or-later

// Translation extraction markers for the messages Polkit looks up at runtime
// through the gettext-domain attributes in the installed policy.
fn polkit_messages() {
    gettext("Change your profile name and phone numbers");
    gettext("Authentication is required to change your profile name and phone numbers");
}
