// SPDX-License-Identifier: GPL-2.0-or-later

//! Loading and persistence for the Whisker-visible user profile.

mod accounts;
mod avatar_persistence;
mod current_user_lookup;
mod display_name;
mod image;
mod loader;
mod models;

pub(crate) use accounts::AccountsError;
#[cfg(feature = "camera")]
pub(crate) use avatar_persistence::save_current_camera_avatar;
pub(crate) use avatar_persistence::{AvatarRemovalError, AvatarSaveError, remove_current_avatar, save_current_avatar};
pub(crate) use display_name::{DisplayNameState, save_current_display_name};
pub(crate) use image::{SUPPORTED_AVATAR_MIME_TYPES, SUPPORTED_AVATAR_SUFFIXES};
pub(crate) use loader::load_current_profile;
pub(crate) use models::{AvatarSaveOutcome, DisplayNameSaveOutcome, LoadWarning, LoadedProfile};
