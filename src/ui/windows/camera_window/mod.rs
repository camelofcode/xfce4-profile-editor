// SPDX-License-Identifier: GPL-2.0-or-later

//! Camera window type, template binding, and signal wiring.

use std::cell::RefCell;

use gettextrs::gettext;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::core::{CameraCapture, CameraCaptureStatus, CameraDevice, CameraFrame};

mod capture;
mod preview;

type AcceptHandler = Box<dyn Fn(CameraFrame) + 'static>;

mod imp {
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/camelofcode/Xfce4ProfileEditor/ui/windows/camera-window.ui")]
    pub struct CameraWindow {
        #[template_child]
        pub cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub capture_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub loading_indicator: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub preview_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub retake_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub status_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub use_photo_button: TemplateChild<gtk::Button>,
        pub(super) accept_handler: RefCell<Option<AcceptHandler>>,
        pub(super) captured_frame: RefCell<Option<CameraFrame>>,
        pub(super) device: RefCell<Option<CameraDevice>>,
        pub(super) latest_frame: RefCell<Option<CameraFrame>>,
        pub(super) last_status: RefCell<Option<CameraCaptureStatus>>,
        pub(super) poll_source: RefCell<Option<glib::SourceId>>,
        pub(super) session: RefCell<Option<CameraCapture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CameraWindow {
        const NAME: &'static str = "Xfce4ProfileEditorCameraWindow";
        type Type = super::CameraWindow;
        type ParentType = gtk::Window;

        fn class_init(class: &mut Self::Class) {
            class.bind_template();
        }

        fn instance_init(instance: &InitializingObject<Self>) {
            instance.init_template();
        }
    }

    impl ObjectImpl for CameraWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let window = self.obj();
            self.cancel_button.connect_clicked(glib::clone!(
                #[weak]
                window,
                move |_| window.close()
            ));
            self.capture_button.connect_clicked(glib::clone!(
                #[weak]
                window,
                move |_| window.capture_current_frame()
            ));
            self.retake_button.connect_clicked(glib::clone!(
                #[weak]
                window,
                move |_| window.retake_photo()
            ));
            self.use_photo_button.connect_clicked(glib::clone!(
                #[weak]
                window,
                move |_| window.accept_photo()
            ));
        }

        fn dispose(&self) {
            self.obj().release_capture();
        }
    }

    impl WidgetImpl for CameraWindow {}

    impl WindowImpl for CameraWindow {
        fn close_request(&self) -> glib::Propagation {
            self.obj().release_capture();
            self.parent_close_request()
        }
    }
}

glib::wrapper! {
    pub struct CameraWindow(ObjectSubclass<imp::CameraWindow>)
        @extends gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
                    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl CameraWindow {
    pub(crate) fn new(
        parent: &impl IsA<gtk::Window>,
        device: CameraDevice,
        on_accept: impl Fn(CameraFrame) + 'static,
    ) -> Self {
        let window: Self = glib::Object::builder().property("transient-for", parent).build();
        window.set_application(parent.as_ref().application().as_ref());
        window.set_title(Some(&format!("{} — {}", gettext("Take photo"), device.name())));
        window.imp().device.replace(Some(device));
        window.imp().accept_handler.replace(Some(Box::new(on_accept)));
        window.start_capture();
        window
    }
}
