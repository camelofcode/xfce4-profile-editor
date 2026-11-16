// SPDX-License-Identifier: GPL-2.0-or-later

//! Asynchronous profile loading and initial widget presentation.

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::core::{LoadedProfile, load_current_profile};

use super::ProfileWindow;

impl ProfileWindow {
    pub(super) fn load_profile(&self) {
        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(load_current_profile).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };

            match result {
                Ok(Ok(loaded)) => window.show_profile(loaded),
                Ok(Err(_)) => window.show_load_error(&gettext("The current user account could not be read.")),
                Err(_) => window.show_load_error(&gettext("Profile loading stopped unexpectedly.")),
            }
        });
    }

    fn show_profile(&self, loaded: LoadedProfile) {
        let imp = self.imp();
        let LoadedProfile {
            display_name,
            avatars,
            warnings,
            display_name_state,
        } = loaded;

        imp.display_name_entry.set_text(&display_name);
        imp.display_name_entry.set_editable(display_name_state.is_some());
        imp.display_name_state.replace(display_name_state);
        imp.load_warnings.replace(warnings);
        imp.avatar_menu_button.set_sensitive(true);

        imp.avatar_present.set(!avatars.is_empty());
        imp.remove_photo_button.set_sensitive(imp.avatar_present.get());
        for avatar in avatars {
            let tooltip = format!("{}\n{}", gettext("Profile image"), avatar.source_path().display());
            let bytes = glib::Bytes::from_owned(avatar.bytes);
            if let Ok(texture) = gtk::gdk::Texture::from_bytes(&bytes) {
                imp.avatar_picture.set_paintable(Some(&texture));
                imp.avatar_picture.set_tooltip_text(Some(&tooltip));
                imp.avatar_picture.set_visible(true);
                break;
            }
        }

        imp.loading_indicator.set_spinning(false);
        imp.loading_indicator.set_visible(false);
        self.show_loaded_warnings();
    }
}
