// SPDX-License-Identifier: GPL-2.0-or-later

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::account_service::{AccountDetails, AccountsError, load_account_details};
use super::current_user::{CurrentUser, CurrentUserError, current_user};
use super::display_name_persistence::DisplayNameState;
use super::image::MAX_SOURCE_BYTES;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LoadedProfile {
    pub(crate) display_name: String,
    pub(crate) avatars: Vec<Avatar>,
    pub(crate) warnings: Vec<LoadWarning>,
    pub(crate) display_name_state: Option<DisplayNameState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Avatar {
    pub(crate) bytes: Vec<u8>,
    source_path: PathBuf,
}

impl Avatar {
    pub(crate) fn source_path(&self) -> &Path {
        &self.source_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LoadWarning {
    AccountsServiceUnavailable(String),
    AvatarUnavailable { path: PathBuf, reason: String },
}

impl LoadWarning {
    pub(crate) fn diagnostic(&self) -> &str {
        match self {
            Self::AccountsServiceUnavailable(reason) | Self::AvatarUnavailable { reason, .. } => {
                reason
            }
        }
    }
}

pub(crate) fn load_current_profile() -> Result<LoadedProfile, CurrentUserError> {
    load_with(current_user()?, load_account_details)
}

fn load_with(
    user: CurrentUser,
    details_for_uid: impl FnOnce(u32) -> Result<AccountDetails, AccountsError>,
) -> Result<LoadedProfile, CurrentUserError> {
    let mut warnings = Vec::new();
    let (display_name, icon_file, display_name_state) = match details_for_uid(user.uid) {
        Ok(account) => {
            let state = DisplayNameState::new(user.uid, &account.display_name);
            (account.display_name, account.icon_file, Some(state))
        }
        Err(error) => {
            warnings.push(LoadWarning::AccountsServiceUnavailable(error.to_string()));
            (String::new(), None, None)
        }
    };
    let avatars = load_avatar_candidates(&user, icon_file.as_ref(), &mut warnings);

    Ok(LoadedProfile {
        display_name,
        avatars,
        warnings,
        display_name_state,
    })
}

fn load_avatar_candidates(
    user: &CurrentUser,
    account_icon_file: Option<&PathBuf>,
    warnings: &mut Vec<LoadWarning>,
) -> Vec<Avatar> {
    let face_path = user.home_directory.join(".face");
    let mut candidates = vec![face_path.clone()];
    if let Some(path) = account_icon_file
        && path != &face_path
    {
        candidates.push(path.clone());
    }

    candidates
        .into_iter()
        .filter_map(|path| match read_avatar(&path) {
            Ok(bytes) => Some(Avatar {
                bytes,
                source_path: path,
            }),
            Err(AvatarReadError::NotFound) => None,
            Err(error) => {
                warnings.push(LoadWarning::AvatarUnavailable {
                    path,
                    reason: error.to_string(),
                });
                None
            }
        })
        .collect()
}

#[derive(Debug, Error)]
enum AvatarReadError {
    #[error("file not found")]
    NotFound,
    #[error("not a regular file")]
    NotAFile,
    #[error("file exceeds the {MAX_SOURCE_BYTES}-byte safety limit")]
    TooLarge,
    #[error("could not read file: {0}")]
    Read(#[source] std::io::Error),
}

fn read_avatar(path: &Path) -> Result<Vec<u8>, AvatarReadError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(AvatarReadError::NotFound);
        }
        Err(error) => return Err(AvatarReadError::Read(error)),
    };
    if !metadata.is_file() {
        return Err(AvatarReadError::NotAFile);
    }
    if metadata.len() > MAX_SOURCE_BYTES as u64 {
        return Err(AvatarReadError::TooLarge);
    }

    fs::read(path).map_err(AvatarReadError::Read)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> CurrentUser {
        CurrentUser {
            uid: 1000,
            home_directory: PathBuf::from("/path/that/does/not/exist"),
        }
    }

    #[test]
    fn loads_accounts_service_data_for_the_current_uid() {
        let loaded = load_with(user(), |_| {
            Ok(AccountDetails {
                display_name: "Ada Lovelace".to_owned(),
                icon_file: None,
            })
        })
        .unwrap();

        assert_eq!(loaded.display_name, "Ada Lovelace");
        assert!(
            loaded
                .display_name_state
                .as_ref()
                .is_some_and(|state| state.matches("Ada Lovelace"))
        );
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn degrades_gracefully_when_accounts_service_is_unavailable() {
        let loaded = load_with(user(), |_| Err(AccountsError::TestUnavailable)).unwrap();

        assert!(loaded.display_name.is_empty());
        assert!(loaded.display_name_state.is_none());
        assert!(matches!(
            loaded.warnings.as_slice(),
            [LoadWarning::AccountsServiceUnavailable(_)]
        ));
    }
}
