// SPDX-License-Identifier: GPL-2.0-or-later

//! GTK-independent profile models and system services.

mod account_service;
mod gecos;
mod gecos_helper;
mod gecos_persistence;
mod profile;
mod profile_store;
mod user;

pub(crate) use gecos_helper::GecosWriteError;
pub(crate) use gecos_persistence::{GecosFields, GecosSaveOutcome, GecosState, save_current_gecos};
pub(crate) use profile::Profile;
pub(crate) use profile_store::{LoadWarning, LoadedProfile, ProfileStore};
