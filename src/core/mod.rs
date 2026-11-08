// SPDX-License-Identifier: GPL-2.0-or-later

//! GTK-independent profile models and system services.

mod account_service;
mod gecos;
// The production client is intentionally connected to the UI in the next
// checkpoint; this checkpoint verifies its isolated process boundary first.
#[allow(dead_code)]
mod gecos_helper;
mod profile;
mod profile_store;
mod user;

pub(crate) use profile::Profile;
pub(crate) use profile_store::{LoadWarning, LoadedProfile, ProfileStore};
