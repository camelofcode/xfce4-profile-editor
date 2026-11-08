// SPDX-License-Identifier: GPL-2.0-or-later

use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};

use crate::config;

pub(crate) fn init() {
    bindtextdomain(config::GETTEXT_PACKAGE, config::LOCALEDIR)
        .expect("failed to bind the gettext translation directory");
    bind_textdomain_codeset(config::GETTEXT_PACKAGE, "UTF-8")
        .expect("failed to set the gettext translation encoding");
    textdomain(config::GETTEXT_PACKAGE).expect("failed to select the gettext translation domain");
}
