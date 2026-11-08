// SPDX-License-Identifier: GPL-2.0-or-later

#![deny(unsafe_code)]

mod config;
mod core;
mod i18n;
mod ui;

use gtk::prelude::*;
use gtk::{gio, glib};

fn main() -> glib::ExitCode {
    i18n::init();

    gio::resources_register_include!("xfce4_profile_editor.gresource")
        .expect("failed to register application resources");

    let application = gtk::Application::builder()
        .application_id(config::APPLICATION_ID)
        .build();

    application.connect_startup(|_| ui::load_styles());
    application.connect_activate(ui::present_profile_window);

    application.run()
}
