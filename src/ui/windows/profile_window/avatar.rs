// SPDX-License-Identifier: GPL-2.0-or-later

//! Avatar selection, persistence, removal, and preview presentation.

use std::path::PathBuf;

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::core::{
    AvatarRemovalError, AvatarSaveError, AvatarSaveOutcome, SUPPORTED_AVATAR_MIME_TYPES, SUPPORTED_AVATAR_SUFFIXES,
    remove_current_avatar, save_current_avatar,
};
#[cfg(feature = "camera")]
use crate::core::{CameraFrame, save_current_camera_avatar};

use super::ProfileWindow;

impl ProfileWindow {
    pub(super) fn choose_photo(&self) {
        let imp = self.imp();
        if !self.begin_image_operation() {
            return;
        }
        imp.avatar_popover.popdown();

        let filter = gtk::FileFilter::new();
        filter.set_name(Some(&gettext("Images")));
        for mime_type in SUPPORTED_AVATAR_MIME_TYPES {
            filter.add_mime_type(mime_type);
        }
        for suffix in SUPPORTED_AVATAR_SUFFIXES {
            filter.add_suffix(suffix);
        }
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Choose a profile image"))
            .modal(true)
            .filters(&filters)
            .default_filter(&filter)
            .build();
        let open = dialog.open_future(Some(self));
        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            match open.await {
                Ok(file) => {
                    let Some(window) = weak_window.upgrade() else {
                        return;
                    };
                    if let Some(path) = file.path() {
                        window.save_selected_photo(path);
                    } else {
                        window.finish_image_operation();
                        window.show_save_status(&gettext("Only local image files can be used."), true, None);
                    }
                }
                Err(error) => {
                    let Some(window) = weak_window.upgrade() else {
                        return;
                    };
                    window.finish_image_operation();
                    if !dialog_error_is_cancellation(&error) {
                        let diagnostic = error.to_string();
                        window.show_save_status(
                            &gettext("The image chooser could not be opened."),
                            true,
                            Some(&diagnostic),
                        );
                    }
                }
            }
        });
    }

    fn save_selected_photo(&self, path: PathBuf) {
        self.show_save_status(&gettext("Saving profile image…"), false, None);
        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || save_current_avatar(path)).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            window.finish_image_operation();
            window.finish_avatar_save_result(result);
        });
    }

    #[cfg(feature = "camera")]
    pub(super) fn save_camera_photo(&self, frame: CameraFrame) {
        if !self.begin_image_operation() {
            return;
        }
        self.show_save_status(&gettext("Saving profile image…"), false, None);
        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || save_current_camera_avatar(frame)).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            window.finish_image_operation();
            window.finish_avatar_save_result(result);
        });
    }

    fn finish_avatar_save_result(
        &self,
        result: Result<Result<AvatarSaveOutcome, AvatarSaveError>, Box<dyn std::any::Any + Send>>,
    ) {
        match result {
            Ok(Ok(outcome)) => self.show_saved_avatar(outcome),
            Ok(Err(error)) => {
                let diagnostic = error.to_string();
                self.show_save_status(&localized_avatar_save_error(&error), true, Some(&diagnostic));
            }
            Err(_) => self.show_save_status(&gettext("Profile image saving stopped unexpectedly."), true, None),
        }
    }

    fn show_saved_avatar(&self, outcome: AvatarSaveOutcome) {
        let (png, path, accounts_error) = outcome.into_parts();
        let bytes = glib::Bytes::from_owned(png);
        match gtk::gdk::Texture::from_bytes(&bytes) {
            Ok(texture) => {
                let imp = self.imp();
                imp.avatar_picture.set_paintable(Some(&texture));
                imp.avatar_picture.set_tooltip_text(Some(&format!(
                    "{}\n{}",
                    gettext("Selected photo"),
                    path.display()
                )));
                imp.avatar_picture.set_visible(true);
                imp.avatar_present.set(true);
                imp.remove_photo_button.set_sensitive(true);
                match accounts_error {
                    Some(error) => {
                        let diagnostic = error.to_string();
                        self.show_warning_status(
                            &gettext("The profile image was saved locally, but AccountsService could not be updated."),
                            Some(&diagnostic),
                        );
                    }
                    None => self.show_save_status(&gettext("Profile image saved."), false, None),
                }
            }
            Err(error) => {
                let diagnostic = error.to_string();
                self.show_save_status(
                    &gettext("The photo preview could not be displayed."),
                    true,
                    Some(&diagnostic),
                );
            }
        }
    }

    pub(super) fn remove_photo(&self) {
        if !self.begin_image_operation() {
            return;
        }
        self.imp().avatar_popover.popdown();
        self.show_save_status(&gettext("Removing profile image…"), false, None);
        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(remove_current_avatar).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            window.finish_image_operation();
            window.finish_avatar_removal_result(result);
        });
    }

    fn finish_avatar_removal_result(
        &self,
        result: Result<Result<Option<crate::core::AccountsError>, AvatarRemovalError>, Box<dyn std::any::Any + Send>>,
    ) {
        match result {
            Ok(Ok(outcome)) => {
                let imp = self.imp();
                imp.avatar_picture.set_paintable(None::<&gtk::gdk::Paintable>);
                imp.avatar_picture.set_tooltip_text(None);
                imp.avatar_picture.set_visible(false);
                imp.avatar_present.set(false);
                imp.remove_photo_button.set_sensitive(false);
                match outcome {
                    Some(error) => {
                        let diagnostic = error.to_string();
                        self.show_warning_status(
                            &gettext("The local profile image was removed, but AccountsService could not be updated."),
                            Some(&diagnostic),
                        );
                    }
                    None => self.show_save_status(&gettext("Profile image removed."), false, None),
                }
            }
            Ok(Err(error)) => {
                let diagnostic = error.to_string();
                self.show_save_status(&localized_avatar_removal_error(&error), true, Some(&diagnostic));
            }
            Err(_) => self.show_save_status(&gettext("Profile image removal stopped unexpectedly."), true, None),
        }
    }

    fn begin_image_operation(&self) -> bool {
        let imp = self.imp();
        if imp.image_operation_running.replace(true) {
            return false;
        }
        imp.choose_photo_button.set_sensitive(false);
        imp.remove_photo_button.set_sensitive(false);
        imp.take_photo_button.set_sensitive(false);
        true
    }

    fn finish_image_operation(&self) {
        let imp = self.imp();
        imp.image_operation_running.set(false);
        imp.choose_photo_button.set_sensitive(true);
        imp.remove_photo_button.set_sensitive(imp.avatar_present.get());
        imp.take_photo_button.set_sensitive(self.camera_device_is_available());
    }
}

fn dialog_error_is_cancellation(error: &glib::Error) -> bool {
    error.matches(gtk::DialogError::Cancelled)
        || error.matches(gtk::DialogError::Dismissed)
        || error.matches(gio::IOErrorEnum::Cancelled)
}

fn localized_avatar_save_error(error: &AvatarSaveError) -> String {
    match error {
        AvatarSaveError::Image(_) => gettext("The selected image could not be used."),
        AvatarSaveError::User(_) => {
            gettext("The current user account could not be read. The profile image was not saved.")
        }
        AvatarSaveError::Local(_) => gettext("The profile image could not be saved."),
    }
}

fn localized_avatar_removal_error(error: &AvatarRemovalError) -> String {
    match error {
        AvatarRemovalError::User(_) => {
            gettext("The current user account could not be read. The profile image was not removed.")
        }
        AvatarRemovalError::Local(_) => gettext("The profile image could not be removed."),
    }
}
