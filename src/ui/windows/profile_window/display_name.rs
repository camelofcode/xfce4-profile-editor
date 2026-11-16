// SPDX-License-Identifier: GPL-2.0-or-later

//! Display-name dirty state, deferred status, and close-time saving.

use std::time::Duration;

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::core::{DisplayNameSaveOutcome, save_current_display_name};

use super::ProfileWindow;

const EDIT_STATUS_DEBOUNCE: Duration = Duration::from_millis(500);
type DisplayNameSaveTaskResult =
    Result<Result<DisplayNameSaveOutcome, crate::core::AccountsError>, Box<dyn std::any::Any + Send>>;

impl ProfileWindow {
    pub(super) fn profile_changed(&self) {
        let imp = self.imp();
        if imp.save_running.get() || imp.display_name_state.borrow().is_none() {
            return;
        }
        if !self.display_name_is_dirty() {
            self.cancel_edit_status_timeout();
            self.show_loaded_warnings();
            return;
        }

        self.cancel_edit_status_timeout();
        let weak_window = self.downgrade();
        let source = glib::timeout_add_local_once(EDIT_STATUS_DEBOUNCE, move || {
            if let Some(window) = weak_window.upgrade() {
                window.imp().edit_status_timeout.borrow_mut().take();
                window.show_profile_edit_status();
            }
        });
        imp.edit_status_timeout.replace(Some(source));
    }

    pub(super) fn cancel_edit_status_timeout(&self) {
        if let Some(source) = self.imp().edit_status_timeout.borrow_mut().take() {
            source.remove();
        }
    }

    pub(super) fn show_profile_edit_status(&self) {
        let imp = self.imp();
        if imp.save_running.get() {
            return;
        }
        if !self.display_name_is_dirty() {
            self.show_loaded_warnings();
            return;
        }
        self.show_save_status(&gettext("Unsaved profile changes."), false, None);
    }

    fn request_profile_save(&self) {
        self.cancel_edit_status_timeout();
        let imp = self.imp();
        let Some(state) = imp.display_name_state.borrow().as_ref().cloned() else {
            return;
        };
        let display_name = imp.display_name_entry.text().to_string();
        if !self.display_name_is_dirty() {
            if imp.close_after_save.replace(false) {
                self.close();
            }
            return;
        }
        if imp.save_running.get() {
            return;
        }

        imp.save_running.set(true);
        imp.display_name_entry.set_editable(false);
        self.show_save_status(&gettext("Saving profile…"), false, None);

        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || save_current_display_name(state, display_name)).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            window.finish_profile_save_result(result);
        });
    }

    fn finish_profile_save_result(&self, result: DisplayNameSaveTaskResult) {
        match result {
            Ok(Ok(outcome)) => {
                let (state, display_name, saved) = outcome.into_parts();
                let imp = self.imp();
                imp.display_name_state.replace(Some(state));
                imp.display_name_entry.set_text(&display_name);
                self.finish_profile_save(saved);
            }
            Ok(Err(error)) => {
                let diagnostic = error.to_string();
                self.finish_profile_save_error(
                    &gettext("The display name could not be saved. It is still in the form."),
                    Some(&diagnostic),
                );
            }
            Err(_) => self.finish_profile_save_error(&gettext("Profile saving stopped unexpectedly."), None),
        }
    }

    fn finish_profile_save(&self, saved: bool) {
        let imp = self.imp();
        imp.save_running.set(false);
        imp.display_name_entry.set_editable(true);
        if saved {
            self.show_save_status(&gettext("Profile saved."), false, None);
        }
        if imp.close_after_save.replace(false) {
            self.close();
        }
    }

    fn finish_profile_save_error(&self, message: &str, diagnostic: Option<&str>) {
        let imp = self.imp();
        imp.save_running.set(false);
        imp.close_after_save.set(false);
        imp.display_name_entry.set_editable(true);
        self.show_save_status(message, true, diagnostic);
    }

    fn display_name_is_dirty(&self) -> bool {
        let imp = self.imp();
        imp.display_name_state
            .borrow()
            .as_ref()
            .is_some_and(|state| !state.matches(imp.display_name_entry.text().as_str()))
    }

    pub(super) fn delay_close_for_save(&self) -> bool {
        let imp = self.imp();
        if imp.display_name_state.borrow().is_none() {
            return false;
        }
        if !self.display_name_is_dirty() && !imp.save_running.get() {
            return false;
        }

        imp.close_after_save.set(true);
        if !imp.save_running.get() {
            self.request_profile_save();
        }
        true
    }
}
