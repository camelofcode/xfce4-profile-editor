// SPDX-License-Identifier: GPL-2.0-or-later

//! Camera device identity.

/// GTK-independent information needed to open a Linux camera.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CameraDevice {
    index: u32,
    name: String,
}

impl CameraDevice {
    pub(super) fn new(index: u32, name: String) -> Self {
        Self { index, name }
    }

    pub(super) fn index(&self) -> u32 {
        self.index
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}
