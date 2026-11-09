// SPDX-License-Identifier: GPL-2.0-or-later

#[cfg(feature = "camera")]
mod camera_window;
mod profile_window;

#[cfg(feature = "camera")]
pub(crate) use camera_window::CameraWindow;
pub(crate) use profile_window::ProfileWindow;
