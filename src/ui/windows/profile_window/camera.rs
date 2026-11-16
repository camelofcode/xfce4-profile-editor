// SPDX-License-Identifier: GPL-2.0-or-later

//! Camera discovery and handoff to the capture window.

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

#[cfg(feature = "camera")]
use crate::core::CameraDevice;
use crate::core::{CameraAvailability, CameraDiscoveryError, discover_cameras};
#[cfg(feature = "camera")]
use crate::ui::windows::CameraWindow;

use super::ProfileWindow;

impl ProfileWindow {
    pub(super) fn load_camera_availability(&self) {
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

    fn set_camera_availability(&self, availability: Result<CameraAvailability, CameraDiscoveryError>) {
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
    pub(super) fn take_photo(&self) {
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

    pub(super) fn camera_device_is_available(&self) -> bool {
        #[cfg(feature = "camera")]
        {
            self.camera_device().is_some()
        }
        #[cfg(not(feature = "camera"))]
        {
            false
        }
    }
}
