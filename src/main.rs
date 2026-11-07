// SPDX-License-Identifier: GPL-2.0-or-later

#![deny(unsafe_code)]

mod core;
mod ui;

use gtk::prelude::*;
use gtk::{gio, glib};

const APPLICATION_ID: &str = "io.github.camelofcode.Xfce4ProfileEditor";

fn main() -> glib::ExitCode {
    gio::resources_register_include!("xfce4_profile_editor.gresource")
        .expect("failed to register application resources");

    let application = gtk::Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    application.connect_startup(|_| ui::load_styles());
    application.connect_activate(ui::present_profile_window);

    application.run()
}
