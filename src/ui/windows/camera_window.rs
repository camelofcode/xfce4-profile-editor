// SPDX-License-Identifier: GPL-2.0-or-later

use std::cell::RefCell;
use std::time::Duration;

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::core::{
    CameraCapture, CameraCaptureError, CameraCaptureStatus, CameraDevice, CameraFrame,
    start_camera_capture,
};

const PREVIEW_INTERVAL: Duration = Duration::from_millis(33);
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
            let weak_window = window.downgrade();
            self.cancel_button.connect_clicked(move |_| {
                if let Some(window) = weak_window.upgrade() {
                    window.close();
                }
            });

            let weak_window = window.downgrade();
            self.capture_button.connect_clicked(move |_| {
                if let Some(window) = weak_window.upgrade() {
                    window.capture_current_frame();
                }
            });

            let weak_window = window.downgrade();
            self.retake_button.connect_clicked(move |_| {
                if let Some(window) = weak_window.upgrade() {
                    window.retake_photo();
                }
            });

            let weak_window = window.downgrade();
            self.use_photo_button.connect_clicked(move |_| {
                if let Some(window) = weak_window.upgrade() {
                    window.accept_photo();
                }
            });
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
        let window: Self = glib::Object::builder()
            .property("transient-for", parent)
            .build();
        window.set_application(parent.as_ref().application().as_ref());
        window.set_title(Some(&format!(
            "{} — {}",
            gettext("Take photo"),
            device.name()
        )));
        window.imp().device.replace(Some(device));
        window
            .imp()
            .accept_handler
            .replace(Some(Box::new(on_accept)));
        window.start_capture();
        window
    }

    fn start_capture(&self) {
        let imp = self.imp();
        let Some(device) = imp.device.borrow().clone() else {
            return;
        };
        imp.captured_frame.borrow_mut().take();
        imp.latest_frame.borrow_mut().take();
        imp.last_status.borrow_mut().take();
        imp.preview_picture
            .set_paintable(None::<&gtk::gdk::Paintable>);
        imp.preview_picture.set_visible(false);
        imp.status_box.set_visible(true);
        imp.status_label.set_label(&gettext("Starting camera…"));
        imp.status_label.set_tooltip_text(None);
        imp.loading_indicator.set_spinning(true);
        imp.loading_indicator.set_visible(true);
        imp.capture_button.set_sensitive(false);
        imp.capture_button.set_visible(true);
        imp.retake_button.set_sensitive(true);
        imp.retake_button.set_visible(false);
        imp.use_photo_button.set_visible(false);
        imp.session.replace(Some(start_camera_capture(device)));
        self.start_polling();
    }

    fn start_polling(&self) {
        self.stop_polling();
        let weak_window = self.downgrade();
        let source = glib::timeout_add_local(PREVIEW_INTERVAL, move || {
            let Some(window) = weak_window.upgrade() else {
                return glib::ControlFlow::Break;
            };
            window.poll_capture();
            glib::ControlFlow::Continue
        });
        self.imp().poll_source.replace(Some(source));
    }

    fn stop_polling(&self) {
        if let Some(source) = self.imp().poll_source.borrow_mut().take() {
            source.remove();
        }
    }

    fn poll_capture(&self) {
        let (status, frame) = {
            let session = self.imp().session.borrow();
            let Some(session) = session.as_ref() else {
                return;
            };
            (session.status(), session.take_latest_frame())
        };

        if self.imp().last_status.borrow().as_ref() != Some(&status) {
            self.show_capture_status(&status);
            self.imp().last_status.replace(Some(status));
        }
        if let Some(frame) = frame {
            self.show_frame(frame);
        }
    }

    fn show_capture_status(&self, status: &CameraCaptureStatus) {
        let imp = self.imp();
        match status {
            CameraCaptureStatus::Starting => {}
            CameraCaptureStatus::Running => {}
            CameraCaptureStatus::Stopped => {
                imp.capture_button.set_sensitive(false);
            }
            CameraCaptureStatus::Failed(error) => {
                imp.loading_indicator.set_spinning(false);
                imp.loading_indicator.set_visible(false);
                imp.status_box.set_visible(true);
                imp.status_label.set_label(&localized_camera_error(error));
                imp.status_label.set_tooltip_text(Some(&error.to_string()));
                imp.capture_button.set_visible(false);
                imp.retake_button.set_visible(true);
                imp.use_photo_button.set_visible(false);
            }
        }
    }

    fn show_frame(&self, frame: CameraFrame) {
        let width = i32::try_from(frame.width()).expect("validated camera width fits in i32");
        let height = i32::try_from(frame.height()).expect("validated camera height fits in i32");
        let stride = usize::try_from(frame.width()).expect("camera width fits in usize") * 3;
        let bytes = glib::Bytes::from_owned(frame.rgb().to_vec());
        let texture = gtk::gdk::MemoryTexture::new(
            width,
            height,
            gtk::gdk::MemoryFormat::R8g8b8,
            &bytes,
            stride,
        );
        let imp = self.imp();
        imp.preview_picture.set_paintable(Some(&texture));
        imp.preview_picture.set_visible(true);
        imp.status_box.set_visible(false);
        imp.loading_indicator.set_spinning(false);
        imp.capture_button.set_sensitive(true);
        imp.latest_frame.replace(Some(frame));
    }

    fn capture_current_frame(&self) {
        let Some(frame) = self.imp().latest_frame.borrow().clone() else {
            return;
        };
        self.stop_polling();
        let imp = self.imp();
        imp.captured_frame.replace(Some(frame));
        imp.capture_button.set_visible(false);
        imp.retake_button.set_visible(true);
        imp.use_photo_button.set_visible(true);
        imp.use_photo_button.grab_focus();
    }

    fn retake_photo(&self) {
        let failed = self
            .imp()
            .session
            .borrow()
            .as_ref()
            .is_some_and(|session| matches!(session.status(), CameraCaptureStatus::Failed(_)));
        if failed {
            self.restart_capture();
            return;
        }

        let imp = self.imp();
        imp.captured_frame.borrow_mut().take();
        imp.capture_button.set_visible(true);
        imp.retake_button.set_visible(false);
        imp.use_photo_button.set_visible(false);
        self.start_polling();
    }

    fn restart_capture(&self) {
        self.stop_polling();
        let imp = self.imp();
        imp.retake_button.set_sensitive(false);
        imp.status_box.set_visible(true);
        imp.status_label.set_label(&gettext("Starting camera…"));
        imp.status_label.set_tooltip_text(None);
        imp.loading_indicator.set_visible(true);
        imp.loading_indicator.set_spinning(true);
        let old_session = imp.session.borrow_mut().take();
        let weak_window = self.downgrade();
        glib::spawn_future_local(async move {
            if let Some(session) = old_session {
                let _ = gio::spawn_blocking(move || session.shutdown()).await;
            }
            if let Some(window) = weak_window.upgrade() {
                window.start_capture();
            }
        });
    }

    fn accept_photo(&self) {
        let Some(frame) = self.imp().captured_frame.borrow_mut().take() else {
            return;
        };
        if let Some(handler) = self.imp().accept_handler.borrow().as_ref() {
            handler(frame);
        }
        self.close();
    }

    fn release_capture(&self) {
        self.stop_polling();
        let Some(session) = self.imp().session.borrow_mut().take() else {
            return;
        };
        session.request_stop();
        let _ = std::thread::Builder::new()
            .name("profile-camera-shutdown".to_owned())
            .spawn(move || {
                let _ = session.shutdown();
            });
    }

    #[cfg(test)]
    pub(crate) fn exercise_capture_controls_for_test(&self) {
        assert!(!self.imp().status_label.is_selectable());

        let frame = CameraFrame::new(2, 1, vec![80; 6]).unwrap();
        self.show_frame(frame);
        assert!(self.imp().capture_button.is_sensitive());

        self.capture_current_frame();
        assert!(self.imp().retake_button.is_visible());
        assert!(self.imp().use_photo_button.is_visible());

        self.retake_photo();
        assert!(self.imp().capture_button.is_visible());
        assert!(!self.imp().use_photo_button.is_visible());
    }
}

fn localized_camera_error(error: &CameraCaptureError) -> String {
    match error {
        CameraCaptureError::Open { .. } => gettext("The camera could not be opened."),
        CameraCaptureError::Configure { .. } | CameraCaptureError::Decode { .. } => {
            gettext("The camera uses an unsupported image format.")
        }
        CameraCaptureError::Stream { .. } => gettext("The camera is busy or unavailable."),
        CameraCaptureError::Frame { .. } => gettext("The camera stopped providing images."),
        CameraCaptureError::InvalidFrame { .. } => gettext("The camera provided an invalid image."),
        CameraCaptureError::WorkerPanicked => gettext("Camera capture stopped unexpectedly."),
        CameraCaptureError::WorkerStart { .. } => gettext("Camera capture could not be started."),
    }
}
