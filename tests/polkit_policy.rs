// SPDX-License-Identifier: GPL-2.0-or-later

const POLICY: &str =
    include_str!("../data/polkit/io.github.camelofcode.Xfce4ProfileEditor.policy.in");

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
