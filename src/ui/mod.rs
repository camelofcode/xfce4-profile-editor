// SPDX-License-Identifier: GPL-2.0-or-later

mod windows;

use gtk::gdk;
use gtk::prelude::*;

use windows::ProfileWindow;

const STYLESHEET_RESOURCE: &str = "/io/github/camelofcode/Xfce4ProfileEditor/ui/app.css";

pub(crate) fn load_styles() {
    let display = gdk::Display::default().expect("GTK did not provide a default display");
    let provider = gtk::CssProvider::new();
    provider.load_from_resource(STYLESHEET_RESOURCE);

    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub(crate) fn present_profile_window(application: &gtk::Application) {
    if let Some(window) = application.active_window() {
        window.present();
        return;
    }

    ProfileWindow::new(application).present();
}
