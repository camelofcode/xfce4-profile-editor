// SPDX-License-Identifier: GPL-2.0-or-later

//! Profile details returned by AccountsService.

use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::core::profile) struct AccountDetails {
    pub(in crate::core::profile) display_name: String,
    pub(in crate::core::profile) icon_file: Option<PathBuf>,
}
