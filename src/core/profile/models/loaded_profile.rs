// SPDX-License-Identifier: GPL-2.0-or-later

//! Profile data prepared for presentation.

use super::super::display_name::DisplayNameState;
use super::{Avatar, LoadWarning};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LoadedProfile {
    pub(crate) display_name: String,
    pub(crate) avatars: Vec<Avatar>,
    pub(crate) warnings: Vec<LoadWarning>,
    pub(crate) display_name_state: Option<DisplayNameState>,
}
