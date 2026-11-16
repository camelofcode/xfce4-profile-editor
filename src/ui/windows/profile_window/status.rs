// SPDX-License-Identifier: GPL-2.0-or-later

//! Status-label presentation and localized load warnings.

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::core::LoadWarning;

use super::ProfileWindow;

impl ProfileWindow {
    pub(super) fn show_loaded_warnings(&self) {
        let imp = self.imp();
        show_warnings(&imp.status_label, &imp.load_warnings.borrow());
    }

    pub(super) fn show_save_status(&self, message: &str, error: bool, diagnostic: Option<&str>) {
        let label = &self.imp().status_label;
        clear_status_classes(label);
        label.add_css_class(if error { "error" } else { "dim-label" });
        label.set_label(message);
        label.set_tooltip_text(diagnostic);
        label.set_visible(true);
    }

    pub(super) fn show_warning_status(&self, message: &str, diagnostic: Option<&str>) {
        let label = &self.imp().status_label;
        clear_status_classes(label);
        label.add_css_class("warning");
        label.set_label(message);
        label.set_tooltip_text(diagnostic);
        label.set_visible(true);
    }

    pub(super) fn show_load_error(&self, message: &str) {
        let imp = self.imp();
        imp.loading_indicator.set_spinning(false);
        imp.loading_indicator.set_visible(false);
        clear_status_classes(&imp.status_label);
        imp.status_label.add_css_class("error");
        imp.status_label.set_label(message);
        imp.status_label.set_tooltip_text(None);
    }
}

fn show_warnings(label: &gtk::Label, warnings: &[LoadWarning]) {
    clear_status_classes(label);
    label.set_tooltip_text(None);
    if warnings.is_empty() {
        label.set_visible(false);
        return;
    }

    label.add_css_class("warning");
    label.set_label(&gettext("Some profile information could not be loaded."));
    label.set_tooltip_text(Some(
        &warnings.iter().map(localized_warning).collect::<Vec<_>>().join("\n\n"),
    ));
}

fn clear_status_classes(label: &gtk::Label) {
    label.remove_css_class("dim-label");
    label.remove_css_class("warning");
    label.remove_css_class("error");
}

fn localized_warning(warning: &LoadWarning) -> String {
    let message = match warning {
        LoadWarning::AccountsServiceUnavailable(_) => gettext("AccountsService profile information could not be read."),
        LoadWarning::AvatarUnavailable { path, .. } => {
            format!("{}\n{}", gettext("A profile image could not be used."), path.display())
        }
    };
    format!("{message}\n{}", warning.diagnostic())
}
