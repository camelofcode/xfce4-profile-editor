// SPDX-License-Identifier: GPL-2.0-or-later

//! Plain values shared by profile-domain operations and the UI façade.

mod account_details;
mod avatar;
mod avatar_save_outcome;
mod current_user;
mod display_name_save_outcome;
mod load_warning;
mod loaded_profile;

pub(super) use account_details::AccountDetails;
pub(crate) use avatar::Avatar;
pub(crate) use avatar_save_outcome::AvatarSaveOutcome;
pub(super) use current_user::CurrentUser;
pub(crate) use display_name_save_outcome::DisplayNameSaveOutcome;
pub(crate) use load_warning::LoadWarning;
pub(crate) use loaded_profile::LoadedProfile;
