// SPDX-License-Identifier: GPL-2.0-or-later

//! Frame presentation and capture/retake/accept interactions.

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::core::{CameraCaptureStatus, CameraFrame};

use super::CameraWindow;

impl CameraWindow {
    pub(super) fn show_frame(&self, frame: CameraFrame) {
        let width = i32::try_from(frame.width()).expect("validated camera width fits in i32");
        let height = i32::try_from(frame.height()).expect("validated camera height fits in i32");
        let stride = usize::try_from(frame.width()).expect("camera width fits in usize") * 3;
        let bytes = glib::Bytes::from_owned(frame.rgb().to_vec());
        let texture = gtk::gdk::MemoryTexture::new(width, height, gtk::gdk::MemoryFormat::R8g8b8, &bytes, stride);
        let imp = self.imp();
        imp.preview_picture.set_paintable(Some(&texture));
        imp.preview_picture.set_visible(true);
        imp.status_box.set_visible(false);
        imp.loading_indicator.set_spinning(false);
        imp.capture_button.set_sensitive(true);
        imp.latest_frame.replace(Some(frame));
    }

    pub(super) fn capture_current_frame(&self) {
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

    pub(super) fn retake_photo(&self) {
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

    pub(super) fn accept_photo(&self) {
        let Some(frame) = self.imp().captured_frame.borrow_mut().take() else {
            return;
        };
        if let Some(handler) = self.imp().accept_handler.borrow().as_ref() {
            handler(frame);
        }
        self.close();
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
