// SPDX-License-Identifier: GPL-2.0-or-later

//! Current-user identity needed by the profile domain.

use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::core::profile) struct CurrentUser {
    pub(in crate::core::profile) uid: u32,
    pub(in crate::core::profile) home_directory: PathBuf,
}
