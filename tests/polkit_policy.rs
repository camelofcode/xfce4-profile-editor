// SPDX-License-Identifier: GPL-2.0-or-later

const POLICY: &str =
    include_str!("../data/polkit/io.github.camelofcode.Xfce4ProfileEditor.policy.in");
const POLKIT_MESSAGE_MARKERS: &str = include_str!("../po/polkit-messages.rs");

#[test]
fn gecos_policy_is_narrow_and_requires_active_admin_authentication() {
    assert!(POLICY.contains("io.github.camelofcode.Xfce4ProfileEditor.modify-gecos"));
    assert!(POLICY.contains("<allow_any>no</allow_any>"));
    assert!(POLICY.contains("<allow_inactive>no</allow_inactive>"));
    assert!(POLICY.contains("<allow_active>auth_admin</allow_active>"));
    assert!(POLICY.contains("org.freedesktop.policykit.exec.path"));
    assert!(POLICY.contains("@libexecdir@/xfce4-profile-editor-helper"));
    assert!(POLICY.contains("org.freedesktop.policykit.exec.argv1"));
    assert!(POLICY.contains(">update-gecos</annotate>"));
    assert!(!POLICY.contains("auth_admin_keep"));
    assert!(!POLICY.contains("exec.allow_gui"));
}

#[test]
fn every_polkit_translation_marker_matches_the_policy() {
    let messages: Vec<_> = POLKIT_MESSAGE_MARKERS
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("gettext(\"")
                .and_then(|message| message.strip_suffix("\");"))
        })
        .collect();

    assert_eq!(messages.len(), 2, "expected both Polkit prompt messages");
    for message in messages {
        assert!(
            POLICY.contains(message),
            "Polkit translation marker does not match the policy: {message}"
        );
    }
}
