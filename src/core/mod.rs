// SPDX-License-Identifier: GPL-2.0-or-later

//! GTK-independent profile models and system services.

mod account_service;
mod gecos;
mod profile;
mod profile_store;
mod user;

pub(crate) use profile::Profile;
pub(crate) use profile_store::{LoadWarning, LoadedProfile, ProfileStore};
