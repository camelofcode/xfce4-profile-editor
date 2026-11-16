// SPDX-License-Identifier: GPL-2.0-or-later

//! Capture-worker lifecycle, polling, and failure presentation.

use std::time::Duration;

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::core::{CameraCaptureError, CameraCaptureStatus, start_camera_capture};

use super::CameraWindow;

const PREVIEW_INTERVAL: Duration = Duration::from_millis(33);

impl CameraWindow {
    pub(super) fn start_capture(&self) {
        let imp = self.imp();
        let Some(device) = imp.device.borrow().clone() else {
            return;
        };
        imp.captured_frame.borrow_mut().take();
        imp.latest_frame.borrow_mut().take();
        imp.last_status.borrow_mut().take();
        imp.preview_picture.set_paintable(None::<&gtk::gdk::Paintable>);
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

    pub(super) fn start_polling(&self) {
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

    pub(super) fn stop_polling(&self) {
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
            CameraCaptureStatus::Starting | CameraCaptureStatus::Running => {}
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

    pub(super) fn restart_capture(&self) {
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

    pub(super) fn release_capture(&self) {
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
