// SPDX-License-Identifier: GPL-2.0-or-later

use std::cell::{Cell, RefCell};
use std::time::Duration;

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::core::{
    GecosFields, GecosSaveOutcome, GecosState, GecosWriteError, LoadWarning, LoadedProfile,
    Profile, ProfileStore, save_current_gecos,
};

const VALIDATION_DEBOUNCE: Duration = Duration::from_millis(500);

mod imp {
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/camelofcode/Xfce4ProfileEditor/ui/windows/profile-window.ui")]
    pub struct ProfileWindow {
        #[template_child]
        pub avatar_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub close_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub email_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub first_name_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub home_phone_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub last_name_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub loading_indicator: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub office_phone_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        pub(super) gecos_state: RefCell<Option<GecosState>>,
        pub(super) validation_timeout: RefCell<Option<glib::SourceId>>,
        pub(super) save_running: Cell<bool>,
        pub(super) close_after_save: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProfileWindow {
        const NAME: &'static str = "Xfce4ProfileEditorWindow";
        type Type = super::ProfileWindow;
        type ParentType = gtk::ApplicationWindow;

        fn class_init(class: &mut Self::Class) {
            class.bind_template();
        }

        fn instance_init(instance: &InitializingObject<Self>) {
            instance.init_template();
        }
    }

    impl ObjectImpl for ProfileWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let window = self.obj();
            let weak_window = window.downgrade();
            self.close_button.connect_clicked(move |_| {
                if let Some(window) = weak_window.upgrade() {
                    window.close();
                }
            });

            for entry in [
                self.first_name_entry.get(),
                self.last_name_entry.get(),
                self.home_phone_entry.get(),
                self.office_phone_entry.get(),
            ] {
                let weak_window = window.downgrade();
                entry.connect_changed(move |_| {
                    if let Some(window) = weak_window.upgrade() {
                        window.gecos_changed();
                    }
                });

                let weak_window = window.downgrade();
                entry.connect_activate(move |_| {
                    if let Some(window) = weak_window.upgrade() {
                        window.child_focus(gtk::DirectionType::TabForward);
                    }
                });

                let focus = gtk::EventControllerFocus::new();
                let weak_window = window.downgrade();
                focus.connect_leave(move |_| {
                    if let Some(window) = weak_window.upgrade() {
                        window.cancel_validation_timeout();
                        window.validate_gecos_edits();
                    }
                });
                entry.add_controller(focus);
            }
            window.load_profile();
        }

        fn dispose(&self) {
            if let Some(source) = self.validation_timeout.borrow_mut().take() {
                source.remove();
            }
        }
    }
    impl WidgetImpl for ProfileWindow {}
    impl WindowImpl for ProfileWindow {
        fn close_request(&self) -> glib::Propagation {
            if self.obj().delay_close_for_save() {
                glib::Propagation::Stop
            } else {
                self.parent_close_request()
            }
        }
    }
    impl ApplicationWindowImpl for ProfileWindow {}
}

glib::wrapper! {
    pub struct ProfileWindow(ObjectSubclass<imp::ProfileWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible,
                    gtk::Buildable, gtk::ConstraintTarget, gtk::Native,
                    gtk::Root, gtk::ShortcutManager;
}

impl ProfileWindow {
    pub(crate) fn new(application: &gtk::Application) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    fn load_profile(&self) {
        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(ProfileStore::load_current).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };

            match result {
                Ok(Ok(loaded)) => window.show_profile(loaded),
                Ok(Err(_)) => {
                    window.show_load_error(&gettext("The current user account could not be read."))
                }
                Err(_) => window.show_load_error(&gettext("Profile loading stopped unexpectedly.")),
            }
        });
    }

    fn show_profile(&self, loaded: LoadedProfile) {
        let imp = self.imp();
        let LoadedProfile {
            profile,
            warnings,
            gecos_state,
        } = loaded;
        let Profile {
            first_name,
            last_name,
            home_phone,
            email,
            office_phone,
            avatars,
        } = profile;

        imp.first_name_entry.set_text(&first_name);
        imp.last_name_entry.set_text(&last_name);
        imp.home_phone_entry.set_text(&home_phone);
        imp.email_entry.set_text(&email);
        imp.office_phone_entry.set_text(&office_phone);
        imp.gecos_state.replace(Some(gecos_state));
        self.set_gecos_editable(true);

        for avatar in avatars {
            let tooltip = format!(
                "{}\n{}",
                gettext("Profile image"),
                avatar.source_path().display()
            );
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
        show_warnings(&imp.status_label, &warnings);
    }

    fn gecos_fields(&self) -> GecosFields {
        let imp = self.imp();
        GecosFields {
            first_name: imp.first_name_entry.text().to_string(),
            last_name: imp.last_name_entry.text().to_string(),
            home_phone: imp.home_phone_entry.text().to_string(),
            office_phone: imp.office_phone_entry.text().to_string(),
        }
    }

    fn set_gecos_fields(&self, fields: &GecosFields) {
        let imp = self.imp();
        imp.first_name_entry.set_text(&fields.first_name);
        imp.last_name_entry.set_text(&fields.last_name);
        imp.home_phone_entry.set_text(&fields.home_phone);
        imp.office_phone_entry.set_text(&fields.office_phone);
    }

    fn set_gecos_editable(&self, editable: bool) {
        let imp = self.imp();
        imp.first_name_entry.set_editable(editable);
        imp.last_name_entry.set_editable(editable);
        imp.home_phone_entry.set_editable(editable);
        imp.office_phone_entry.set_editable(editable);
    }

    fn gecos_changed(&self) {
        let imp = self.imp();
        let Some(state) = imp.gecos_state.borrow().as_ref().cloned() else {
            return;
        };
        if state.matches(&self.gecos_fields()) {
            self.cancel_validation_timeout();
            return;
        }

        self.cancel_validation_timeout();
        let weak_window = self.downgrade();
        let source = glib::timeout_add_local_once(VALIDATION_DEBOUNCE, move || {
            if let Some(window) = weak_window.upgrade() {
                window.imp().validation_timeout.borrow_mut().take();
                window.validate_gecos_edits();
            }
        });
        imp.validation_timeout.replace(Some(source));
    }

    fn cancel_validation_timeout(&self) {
        if let Some(source) = self.imp().validation_timeout.borrow_mut().take() {
            source.remove();
        }
    }

    fn validate_gecos_edits(&self) {
        let fields = self.gecos_fields();
        match fields.validate() {
            Ok(()) => self.show_save_status(&gettext("Unsaved profile changes."), false, None),
            Err(error) => {
                let diagnostic = error.to_string();
                self.show_save_status(
                    &gettext("A name or phone number is not valid."),
                    true,
                    Some(&diagnostic),
                );
            }
        }
    }

    fn request_gecos_save(&self) {
        self.cancel_validation_timeout();
        let imp = self.imp();
        let Some(state) = imp.gecos_state.borrow().as_ref().cloned() else {
            return;
        };
        let fields = self.gecos_fields();
        if state.matches(&fields) {
            if imp.close_after_save.replace(false) {
                self.close();
            }
            return;
        }
        if imp.save_running.get() {
            return;
        }

        imp.save_running.set(true);
        self.set_gecos_editable(false);
        self.show_save_status(&gettext("Saving profile…"), false, None);

        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || save_current_gecos(state, fields)).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            window.finish_gecos_save(result);
        });
    }

    fn finish_gecos_save(
        &self,
        result: Result<Result<GecosSaveOutcome, GecosWriteError>, Box<dyn std::any::Any + Send>>,
    ) {
        let imp = self.imp();
        imp.save_running.set(false);
        self.set_gecos_editable(true);

        match result {
            Ok(Ok(outcome)) => {
                let (state, fields, saved) = outcome.into_parts();
                imp.gecos_state.replace(Some(state));
                self.set_gecos_fields(&fields);
                if saved {
                    self.show_save_status(&gettext("Profile saved."), false, None);
                }
                if imp.close_after_save.replace(false) {
                    self.close();
                }
            }
            Ok(Err(error)) => {
                imp.close_after_save.set(false);
                let diagnostic = error.to_string();
                self.show_save_status(
                    &localized_gecos_write_error(&error),
                    true,
                    Some(&diagnostic),
                );
            }
            Err(_) => {
                imp.close_after_save.set(false);
                self.show_save_status(&gettext("Profile saving stopped unexpectedly."), true, None);
            }
        }
    }

    fn delay_close_for_save(&self) -> bool {
        let imp = self.imp();
        let Some(state) = imp.gecos_state.borrow().as_ref().cloned() else {
            return false;
        };
        let dirty = !state.matches(&self.gecos_fields());
        if !dirty && !imp.save_running.get() {
            return false;
        }

        imp.close_after_save.set(true);
        if !imp.save_running.get() {
            self.request_gecos_save();
        }
        true
    }

    fn show_save_status(&self, message: &str, error: bool, diagnostic: Option<&str>) {
        let label = &self.imp().status_label;
        label.remove_css_class("dim-label");
        label.remove_css_class("warning");
        label.remove_css_class("error");
        label.add_css_class(if error { "error" } else { "dim-label" });
        label.set_label(message);
        label.set_tooltip_text(diagnostic);
        label.set_visible(true);
    }

    fn show_load_error(&self, message: &str) {
        let imp = self.imp();
        imp.loading_indicator.set_spinning(false);
        imp.loading_indicator.set_visible(false);
        imp.status_label.remove_css_class("dim-label");
        imp.status_label.add_css_class("error");
        imp.status_label.set_label(message);
        imp.status_label.set_tooltip_text(None);
    }
}

fn localized_gecos_write_error(error: &GecosWriteError) -> String {
    match error {
        GecosWriteError::Validation(_) | GecosWriteError::HelperValidation(_) => {
            gettext("A name or phone number is not valid. Your changes were not saved.")
        }
        GecosWriteError::AuthenticationCancelled => {
            gettext("Authentication was cancelled. Your changes were not saved.")
        }
        GecosWriteError::AuthorizationDenied => {
            gettext("Authorization was denied. Your changes were not saved.")
        }
        GecosWriteError::StaleData => gettext(
            "The account changed after it was loaded. Reopen the profile editor and try again.",
        ),
        GecosWriteError::AccountNotLocal => {
            gettext("This account is not stored locally and cannot be changed here.")
        }
        GecosWriteError::AmbiguousAccount => {
            gettext("The local account record is ambiguous and was not changed.")
        }
        _ => gettext("The profile could not be saved. Your changes are still in the form."),
    }
}

fn show_warnings(label: &gtk::Label, warnings: &[LoadWarning]) {
    if warnings.is_empty() {
        label.set_visible(false);
        return;
    }

    label.remove_css_class("dim-label");
    label.add_css_class("warning");
    label.set_label(&gettext("Some profile information could not be loaded."));
    label.set_tooltip_text(Some(
        &warnings
            .iter()
            .map(localized_warning)
            .collect::<Vec<_>>()
            .join("\n\n"),
    ));
}

fn localized_warning(warning: &LoadWarning) -> String {
    match warning {
        LoadWarning::AccountsServiceUnavailable(_) => {
            gettext("AccountsService profile information could not be read.")
        }
        LoadWarning::AvatarUnavailable { path, .. } => format!(
            "{}\n{}",
            gettext("A profile image could not be used."),
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_window_constructs_from_its_resource_template() {
        if gtk::init().is_err() {
            eprintln!("skipping GTK window test because no display is available");
            return;
        }

        gio::resources_register_include!("xfce4_profile_editor.gresource")
            .expect("failed to register test resources");
        crate::i18n::init();
        crate::ui::load_styles();

        let application = gtk::Application::builder()
            .application_id("io.github.camelofcode.Xfce4ProfileEditor.Tests")
            .build();
        application
            .register(None::<&gio::Cancellable>)
            .expect("failed to register test application");
        let window = ProfileWindow::new(&application);

        assert!(window.title().is_some_and(|title| !title.is_empty()));
        window.present();
        assert!(window.is_visible());
        window.close();
    }
}
