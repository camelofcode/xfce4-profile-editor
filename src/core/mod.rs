// SPDX-License-Identifier: GPL-2.0-or-later

//! GTK-independent profile models and system services.

mod account_service;
mod email_persistence;
mod gecos;
mod gecos_helper;
mod gecos_persistence;
mod image;
mod profile;
mod profile_persistence;
mod profile_store;
mod user;

pub(crate) use email_persistence::{EmailState, EmailWriteError};
pub(crate) use gecos_helper::GecosWriteError;
pub(crate) use gecos_persistence::{GecosFields, GecosState};
pub(crate) use image::{
    SUPPORTED_AVATAR_MIME_TYPES, SUPPORTED_AVATAR_SUFFIXES, normalise_avatar_file,
};
pub(crate) use profile::Profile;
pub(crate) use profile_persistence::{
    ProfileSaveError, ProfileSaveOutcome, ProfileSaveRequest, save_current_profile,
};
pub(crate) use profile_store::{LoadWarning, LoadedProfile, ProfileStore};
