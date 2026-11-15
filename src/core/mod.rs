// SPDX-License-Identifier: GPL-2.0-or-later

//! GTK-independent profile models and system services.

mod account_service;
mod avatar_persistence;
mod camera;
mod current_user;
mod display_name_persistence;
mod image;
mod profile_loader;

pub(crate) use account_service::AccountsError;
#[cfg(feature = "camera")]
pub(crate) use avatar_persistence::save_current_camera_avatar;
pub(crate) use avatar_persistence::{
    AvatarRemovalError, AvatarSaveError, AvatarSaveOutcome, remove_current_avatar,
    save_current_avatar,
};
pub(crate) use camera::{CameraAvailability, CameraDiscoveryError, discover_cameras};
#[cfg(feature = "camera")]
pub(crate) use camera::{
    CameraCapture, CameraCaptureError, CameraCaptureStatus, CameraDevice, CameraFrame,
    start_camera_capture,
};
pub(crate) use display_name_persistence::{
    DisplayNameSaveOutcome, DisplayNameState, save_current_display_name,
};
pub(crate) use image::{SUPPORTED_AVATAR_MIME_TYPES, SUPPORTED_AVATAR_SUFFIXES};
pub(crate) use profile_loader::{LoadWarning, LoadedProfile, load_current_profile};
