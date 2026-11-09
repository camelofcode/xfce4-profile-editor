// SPDX-License-Identifier: GPL-2.0-or-later

use gtk::gio;

const RESOURCE_PATHS: &[&str] = &[
    "/io/github/camelofcode/Xfce4ProfileEditor/ui/windows/profile-window.ui",
    "/io/github/camelofcode/Xfce4ProfileEditor/ui/windows/camera-window.ui",
    "/io/github/camelofcode/Xfce4ProfileEditor/ui/app.css",
];

#[test]
fn application_resources_are_embedded() {
    gio::resources_register_include!("xfce4_profile_editor.gresource")
        .expect("failed to register test resources");

    for path in RESOURCE_PATHS {
        gio::resources_lookup_data(path, gio::ResourceLookupFlags::NONE)
            .unwrap_or_else(|error| panic!("resource {path} is unavailable: {error}"));
    }
}
