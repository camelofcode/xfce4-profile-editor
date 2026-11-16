// SPDX-License-Identifier: GPL-2.0-or-later

//! GTK-independent profile and camera domains.

mod camera;
mod profile;

pub(crate) use camera::{CameraAvailability, CameraDiscoveryError, discover_cameras};
#[cfg(feature = "camera")]
pub(crate) use camera::{
    CameraCapture, CameraCaptureError, CameraCaptureStatus, CameraDevice, CameraFrame, start_camera_capture,
};
#[cfg(feature = "camera")]
pub(crate) use profile::save_current_camera_avatar;
pub(crate) use profile::{
    AccountsError, AvatarRemovalError, AvatarSaveError, AvatarSaveOutcome, DisplayNameSaveOutcome, DisplayNameState,
    LoadWarning, LoadedProfile, SUPPORTED_AVATAR_MIME_TYPES, SUPPORTED_AVATAR_SUFFIXES, load_current_profile,
    remove_current_avatar, save_current_avatar, save_current_display_name,
};
