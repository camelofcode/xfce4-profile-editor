// SPDX-License-Identifier: GPL-2.0-or-later

//! Result data from saving a profile avatar.

use std::path::PathBuf;

use super::super::accounts::AccountsError;

#[derive(Debug)]
pub(crate) struct AvatarSaveOutcome {
    png: Vec<u8>,
    face_path: PathBuf,
    accounts_error: Option<AccountsError>,
}

impl AvatarSaveOutcome {
    pub(in crate::core::profile) fn new(
        png: Vec<u8>,
        face_path: PathBuf,
        accounts_error: Option<AccountsError>,
    ) -> Self {
        Self {
            png,
            face_path,
            accounts_error,
        }
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (Vec<u8>, PathBuf, Option<AccountsError>) {
        (self.png, self.face_path, self.accounts_error)
    }
}
