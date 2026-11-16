// SPDX-License-Identifier: GPL-2.0-or-later

//! Profile window type, template binding, and lifecycle wiring.

use std::cell::{Cell, RefCell};

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::core::{CameraAvailability, CameraDiscoveryError, DisplayNameState, LoadWarning};

mod avatar;
mod camera;
mod display_name;
mod loading;
mod status;

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
        pub display_name_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub loading_indicator: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub remove_photo_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub take_photo_button: TemplateChild<gtk::Button>,
        pub(super) display_name_state: RefCell<Option<DisplayNameState>>,
        pub(super) camera_availability: RefCell<Option<Result<CameraAvailability, CameraDiscoveryError>>>,
        pub(super) load_warnings: RefCell<Vec<LoadWarning>>,
        pub(super) edit_status_timeout: RefCell<Option<glib::SourceId>>,
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
            self.close_button.connect_clicked(glib::clone!(
                #[weak]
                window,
                move |_| window.close()
            ));
            self.choose_photo_button.connect_clicked(glib::clone!(
                #[weak]
                window,
                move |_| window.choose_photo()
            ));

            #[cfg(feature = "camera")]
            self.take_photo_button.connect_clicked(glib::clone!(
                #[weak]
                window,
                move |_| window.take_photo()
            ));
            self.remove_photo_button.connect_clicked(glib::clone!(
                #[weak]
                window,
                move |_| window.remove_photo()
            ));

            self.display_name_entry.connect_changed(glib::clone!(
                #[weak]
                window,
                move |_| window.profile_changed()
            ));

            let focus = gtk::EventControllerFocus::new();
            focus.connect_leave(glib::clone!(
                #[weak]
                window,
                move |_| {
                    window.cancel_edit_status_timeout();
                    window.show_profile_edit_status();
                }
            ));
            self.display_name_entry.add_controller(focus);

            window.load_profile();
            window.load_camera_availability();
        }

        fn dispose(&self) {
            if let Some(source) = self.edit_status_timeout.borrow_mut().take() {
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
        glib::Object::builder().property("application", application).build()
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

        gio::resources_register_include!("xfce4_profile_editor.gresource").expect("failed to register test resources");
        crate::i18n::init();
        crate::ui::load_styles();

        let application = gtk::Application::builder()
            .application_id("io.github.camelofcode.Xfce4ProfileEditor.Tests")
            .build();
        application
            .register(None::<&gio::Cancellable>)
            .expect("failed to register test application");
        let window = ProfileWindow::new(&application);

        assert!(window.imp().display_name_entry.secondary_icon_name().is_none());

        assert!(window.title().is_some_and(|title| !title.is_empty()));
        window.present();
        assert!(window.is_visible());
        #[cfg(feature = "camera")]
        {
            use crate::ui::windows::CameraWindow;

            let camera: CameraWindow = glib::Object::builder().property("transient-for", &window).build();
            camera.present();
            assert!(camera.is_visible());
            camera.exercise_capture_controls_for_test();
            camera.close();
        }
        window.close();
    }
}
