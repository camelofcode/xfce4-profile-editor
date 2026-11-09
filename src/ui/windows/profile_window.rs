// SPDX-License-Identifier: GPL-2.0-or-later

use std::cell::{Cell, RefCell};
use std::time::Duration;

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::core::{
    AvatarRemovalError, AvatarRemovalOutcome, AvatarSaveError, AvatarSaveOutcome,
    CameraAvailability, CameraDiscoveryError, EmailState, EmailWriteError, GecosFields, GecosState,
    GecosWriteError, LoadWarning, LoadedProfile, Profile, ProfileSaveError, ProfileSaveOutcome,
    ProfileSaveRequest, ProfileStore, SUPPORTED_AVATAR_MIME_TYPES, SUPPORTED_AVATAR_SUFFIXES,
    discover_cameras, remove_current_avatar, save_current_avatar, save_current_profile,
};
#[cfg(feature = "camera")]
use crate::core::{CameraDevice, CameraFrame, save_current_camera_avatar};
#[cfg(feature = "camera")]
use crate::ui::windows::CameraWindow;

const VALIDATION_DEBOUNCE: Duration = Duration::from_millis(500);

mod imp {
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/camelofcode/Xfce4ProfileEditor/ui/windows/profile-window.ui")]
    pub struct ProfileWindow {
        #[template_child]
        pub avatar_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub avatar_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub avatar_popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub choose_photo_button: TemplateChild<gtk::Button>,
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
        pub remove_photo_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub take_photo_button: TemplateChild<gtk::Button>,
        pub(super) email_state: RefCell<Option<EmailState>>,
        pub(super) gecos_state: RefCell<Option<GecosState>>,
        pub(super) camera_availability:
            RefCell<Option<Result<CameraAvailability, CameraDiscoveryError>>>,
        pub(super) load_warnings: RefCell<Vec<LoadWarning>>,
        pub(super) validation_timeout: RefCell<Option<glib::SourceId>>,
        pub(super) save_running: Cell<bool>,
        pub(super) close_after_save: Cell<bool>,
        pub(super) avatar_present: Cell<bool>,
        pub(super) image_operation_running: Cell<bool>,
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

            let weak_window = window.downgrade();
            self.choose_photo_button.connect_clicked(move |_| {
                if let Some(window) = weak_window.upgrade() {
                    window.choose_photo();
                }
            });

            #[cfg(feature = "camera")]
            {
                let weak_window = window.downgrade();
                self.take_photo_button.connect_clicked(move |_| {
                    if let Some(window) = weak_window.upgrade() {
                        window.take_photo();
                    }
                });
            }

            let weak_window = window.downgrade();
            self.remove_photo_button.connect_clicked(move |_| {
                if let Some(window) = weak_window.upgrade() {
                    window.remove_photo();
                }
            });

            for entry in [
                self.first_name_entry.get(),
                self.last_name_entry.get(),
                self.home_phone_entry.get(),
                self.email_entry.get(),
                self.office_phone_entry.get(),
            ] {
                let weak_window = window.downgrade();
                entry.connect_changed(move |_| {
                    if let Some(window) = weak_window.upgrade() {
                        window.profile_changed();
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
                        window.validate_profile_edits();
                    }
                });
                entry.add_controller(focus);
            }
            window.load_profile();
            window.load_camera_availability();
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

    fn load_camera_availability(&self) {
        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(discover_cameras).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };

            if let Ok(availability) = result {
                window.set_camera_availability(availability);
            }
        });
    }

    fn set_camera_availability(
        &self,
        availability: Result<CameraAvailability, CameraDiscoveryError>,
    ) {
        #[cfg(feature = "camera")]
        let available = matches!(
            &availability,
            Ok(CameraAvailability::Available(devices)) if !devices.is_empty()
        );
        #[cfg(not(feature = "camera"))]
        let available = false;

        self.imp().take_photo_button.set_visible(available);
        self.imp().take_photo_button.set_sensitive(available);
        self.imp().camera_availability.replace(Some(availability));
    }

    #[cfg(feature = "camera")]
    fn camera_device(&self) -> Option<CameraDevice> {
        match self.imp().camera_availability.borrow().as_ref()? {
            Ok(CameraAvailability::Available(devices)) => devices.first().cloned(),
            Ok(CameraAvailability::NoDevices) | Err(_) => None,
        }
    }

    #[cfg(feature = "camera")]
    fn take_photo(&self) {
        let Some(device) = self.camera_device() else {
            return;
        };
        self.imp().avatar_popover.popdown();
        let weak_window = self.downgrade();
        let camera = CameraWindow::new(self, device, move |frame| {
            if let Some(window) = weak_window.upgrade() {
                window.save_camera_photo(frame);
            }
        });
        camera.present();
    }

    fn show_profile(&self, loaded: LoadedProfile) {
        let imp = self.imp();
        let LoadedProfile {
            profile,
            warnings,
            gecos_state,
            email_state,
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
        imp.email_state.replace(email_state);
        imp.load_warnings.replace(warnings);
        self.set_form_editable(true);
        imp.avatar_menu_button.set_sensitive(true);

        imp.avatar_present.set(!avatars.is_empty());
        imp.remove_photo_button
            .set_sensitive(imp.avatar_present.get());
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
        self.show_loaded_warnings();
    }

    fn choose_photo(&self) {
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
                        window.show_save_status(
                            &gettext("Only local image files can be used."),
                            true,
                            None,
                        );
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

    fn save_selected_photo(&self, path: std::path::PathBuf) {
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
    fn save_camera_photo(&self, frame: CameraFrame) {
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
                self.show_save_status(
                    &localized_avatar_save_error(&error),
                    true,
                    Some(&diagnostic),
                );
            }
            Err(_) => self.show_save_status(
                &gettext("Profile image saving stopped unexpectedly."),
                true,
                None,
            ),
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
                            &gettext(
                                "The profile image was saved locally, but AccountsService could not be updated.",
                            ),
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

    fn remove_photo(&self) {
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
        result: Result<
            Result<AvatarRemovalOutcome, AvatarRemovalError>,
            Box<dyn std::any::Any + Send>,
        >,
    ) {
        match result {
            Ok(Ok(outcome)) => {
                let imp = self.imp();
                imp.avatar_picture
                    .set_paintable(None::<&gtk::gdk::Paintable>);
                imp.avatar_picture.set_tooltip_text(None);
                imp.avatar_picture.set_visible(false);
                imp.avatar_present.set(false);
                imp.remove_photo_button.set_sensitive(false);
                match outcome.into_accounts_error() {
                    Some(error) => {
                        let diagnostic = error.to_string();
                        self.show_warning_status(
                            &gettext(
                                "The local profile image was removed, but AccountsService could not be updated.",
                            ),
                            Some(&diagnostic),
                        );
                    }
                    None => self.show_save_status(&gettext("Profile image removed."), false, None),
                }
            }
            Ok(Err(error)) => {
                let diagnostic = error.to_string();
                self.show_save_status(
                    &localized_avatar_removal_error(&error),
                    true,
                    Some(&diagnostic),
                );
            }
            Err(_) => self.show_save_status(
                &gettext("Profile image removal stopped unexpectedly."),
                true,
                None,
            ),
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
        imp.remove_photo_button
            .set_sensitive(imp.avatar_present.get());
        imp.take_photo_button
            .set_sensitive(self.camera_device_is_available());
    }

    fn camera_device_is_available(&self) -> bool {
        #[cfg(feature = "camera")]
        {
            self.camera_device().is_some()
        }
        #[cfg(not(feature = "camera"))]
        {
            false
        }
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

    fn set_form_editable(&self, editable: bool) {
        let imp = self.imp();
        imp.first_name_entry.set_editable(editable);
        imp.last_name_entry.set_editable(editable);
        imp.home_phone_entry.set_editable(editable);
        imp.office_phone_entry.set_editable(editable);
        imp.email_entry
            .set_editable(editable && imp.email_state.borrow().is_some());
    }

    fn profile_changed(&self) {
        let imp = self.imp();
        if imp.save_running.get() {
            return;
        }
        if imp.gecos_state.borrow().is_none() {
            return;
        }
        if !self.profile_is_dirty() {
            self.cancel_validation_timeout();
            self.show_loaded_warnings();
            return;
        }

        self.cancel_validation_timeout();
        let weak_window = self.downgrade();
        let source = glib::timeout_add_local_once(VALIDATION_DEBOUNCE, move || {
            if let Some(window) = weak_window.upgrade() {
                window.imp().validation_timeout.borrow_mut().take();
                window.validate_profile_edits();
            }
        });
        imp.validation_timeout.replace(Some(source));
    }

    fn cancel_validation_timeout(&self) {
        if let Some(source) = self.imp().validation_timeout.borrow_mut().take() {
            source.remove();
        }
    }

    fn validate_profile_edits(&self) {
        let imp = self.imp();
        if imp.save_running.get() {
            return;
        }
        if !self.profile_is_dirty() {
            self.show_loaded_warnings();
            return;
        }
        let fields = self.gecos_fields();
        if imp
            .gecos_state
            .borrow()
            .as_ref()
            .is_some_and(|state| !state.matches(&fields))
            && let Err(error) = fields.validate()
        {
            let diagnostic = error.to_string();
            self.show_save_status(
                &gettext("A name or phone number is not valid."),
                true,
                Some(&diagnostic),
            );
            return;
        }
        if let Some(state) = imp.email_state.borrow().as_ref()
            && !state.matches(imp.email_entry.text().as_str())
            && let Err(error) = state.validate(imp.email_entry.text().as_str())
        {
            let diagnostic = error.to_string();
            self.show_save_status(
                &gettext("The email address is not valid."),
                true,
                Some(&diagnostic),
            );
            return;
        }

        self.show_save_status(&gettext("Unsaved profile changes."), false, None);
    }

    fn show_loaded_warnings(&self) {
        let imp = self.imp();
        show_warnings(&imp.status_label, &imp.load_warnings.borrow());
    }

    fn request_profile_save(&self) {
        self.cancel_validation_timeout();
        let imp = self.imp();
        let Some(state) = imp.gecos_state.borrow().as_ref().cloned() else {
            return;
        };
        let fields = self.gecos_fields();
        if !self.profile_is_dirty() {
            if imp.close_after_save.replace(false) {
                self.close();
            }
            return;
        }
        if imp.save_running.get() {
            return;
        }

        imp.save_running.set(true);
        self.set_form_editable(false);
        self.show_save_status(&gettext("Saving profile…"), false, None);
        let request = ProfileSaveRequest {
            gecos_state: state,
            gecos_fields: fields,
            email_state: imp.email_state.borrow().as_ref().cloned(),
            email: imp.email_entry.text().to_string(),
        };

        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || save_current_profile(request)).await;
            let Some(window) = weak_window.upgrade() else {
                return;
            };
            window.finish_profile_save_result(result);
        });
    }

    fn finish_profile_save_result(
        &self,
        result: Result<Result<ProfileSaveOutcome, ProfileSaveError>, Box<dyn std::any::Any + Send>>,
    ) {
        match result {
            Ok(Ok(outcome)) => {
                let (gecos_state, fields, email_state, email, saved) = outcome.into_parts();
                let imp = self.imp();
                imp.gecos_state.replace(Some(gecos_state));
                imp.email_state.replace(email_state);
                self.set_gecos_fields(&fields);
                imp.email_entry.set_text(&email);
                self.finish_profile_save(saved);
            }
            Ok(Err(error)) => {
                if let Some(outcome) = error.completed_gecos().cloned() {
                    let (state, fields, _saved) = outcome.into_parts();
                    self.imp().gecos_state.replace(Some(state));
                    self.set_gecos_fields(&fields);
                }
                let diagnostic = error.to_string();
                self.finish_profile_save_error(
                    &localized_profile_save_error(&error),
                    Some(&diagnostic),
                );
            }
            Err(_) => {
                self.finish_profile_save_error(
                    &gettext("Profile saving stopped unexpectedly."),
                    None,
                );
            }
        }
    }

    fn finish_profile_save(&self, saved: bool) {
        let imp = self.imp();
        imp.save_running.set(false);
        self.set_form_editable(true);
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
        self.set_form_editable(true);
        self.show_save_status(message, true, diagnostic);
    }

    fn profile_is_dirty(&self) -> bool {
        let imp = self.imp();
        let gecos_dirty = imp
            .gecos_state
            .borrow()
            .as_ref()
            .is_some_and(|state| !state.matches(&self.gecos_fields()));
        let email_dirty = imp
            .email_state
            .borrow()
            .as_ref()
            .is_some_and(|state| !state.matches(imp.email_entry.text().as_str()));
        gecos_dirty || email_dirty
    }

    fn delay_close_for_save(&self) -> bool {
        let imp = self.imp();
        if imp.gecos_state.borrow().is_none() {
            return false;
        }
        if !self.profile_is_dirty() && !imp.save_running.get() {
            return false;
        }

        imp.close_after_save.set(true);
        if !imp.save_running.get() {
            self.request_profile_save();
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

    fn show_warning_status(&self, message: &str, diagnostic: Option<&str>) {
        let label = &self.imp().status_label;
        label.remove_css_class("dim-label");
        label.remove_css_class("warning");
        label.remove_css_class("error");
        label.add_css_class("warning");
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
        AvatarRemovalError::User(_) => gettext(
            "The current user account could not be read. The profile image was not removed.",
        ),
        AvatarRemovalError::Local(_) => gettext("The profile image could not be removed."),
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

fn localized_email_write_error(error: &EmailWriteError) -> String {
    match error {
        EmailWriteError::Validation(_) => {
            gettext("The email address is not valid. Your changes were not saved.")
        }
        EmailWriteError::Accounts(_) => {
            gettext("The email address could not be saved. It is still in the form.")
        }
    }
}

fn localized_profile_save_error(error: &ProfileSaveError) -> String {
    match error {
        ProfileSaveError::Gecos(error) => localized_gecos_write_error(error),
        ProfileSaveError::Email { source, .. } => localized_email_write_error(source),
    }
}

fn show_warnings(label: &gtk::Label, warnings: &[LoadWarning]) {
    label.remove_css_class("dim-label");
    label.remove_css_class("warning");
    label.remove_css_class("error");
    label.set_tooltip_text(None);
    if warnings.is_empty() {
        label.set_visible(false);
        return;
    }

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
        #[cfg(feature = "camera")]
        {
            let camera: CameraWindow = glib::Object::builder()
                .property("transient-for", &window)
                .build();
            camera.present();
            assert!(camera.is_visible());
            camera.exercise_capture_controls_for_test();
            camera.close();
        }
        window.close();
    }
}
