// SPDX-License-Identifier: GPL-2.0-or-later

//! GTK-independent profile models and system services.

mod account_service;
mod email_persistence;
mod gecos;
mod gecos_helper;
mod gecos_persistence;
#[allow(
    dead_code,
    reason = "Phase 4a is reviewed independently before Phase 4b connects the file chooser"
)]
mod image;
mod profile;
mod profile_persistence;
mod profile_store;
mod user;

pub(crate) use email_persistence::{EmailState, EmailWriteError};
pub(crate) use gecos_helper::GecosWriteError;
pub(crate) use gecos_persistence::{GecosFields, GecosState};
pub(crate) use profile::Profile;
pub(crate) use profile_persistence::{
    ProfileSaveError, ProfileSaveOutcome, ProfileSaveRequest, save_current_profile,
};
pub(crate) use profile_store::{LoadWarning, LoadedProfile, ProfileStore};
