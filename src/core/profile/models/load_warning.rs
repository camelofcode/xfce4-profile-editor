// SPDX-License-Identifier: GPL-2.0-or-later

//! Recoverable warnings encountered while loading a profile.

use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LoadWarning {
    AccountsServiceUnavailable(String),
    AvatarUnavailable { path: PathBuf, reason: String },
}

impl LoadWarning {
    pub(crate) fn diagnostic(&self) -> &str {
        match self {
            Self::AccountsServiceUnavailable(reason) | Self::AvatarUnavailable { reason, .. } => reason,
        }
    }
}
