// SPDX-License-Identifier: GPL-2.0-or-later

pub(crate) const APPLICATION_ID: &str = "io.github.camelofcode.Xfce4ProfileEditor";
pub(crate) const GETTEXT_PACKAGE: &str = match option_env!("GETTEXT_PACKAGE") {
    Some(package) => package,
    None => "xfce4-profile-editor",
};
pub(crate) const LOCALEDIR: &str = match option_env!("LOCALEDIR") {
    Some(directory) => directory,
    None => "/usr/share/locale",
};
