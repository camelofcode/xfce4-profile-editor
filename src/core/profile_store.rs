// SPDX-License-Identifier: GPL-2.0-or-later

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::account_service::{AccountDetails, AccountsSource, SystemAccountsSource};
use super::display_name_persistence::DisplayNameState;
use super::image::MAX_SOURCE_BYTES;
use super::profile::{Avatar, AvatarSource, Profile};
use super::user::{NssUserSource, UserLookupError, UserRecord, UserSource};

type AvatarCandidate = (PathBuf, fn(PathBuf) -> AvatarSource);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct LoadedProfile {
    pub(crate) profile: Profile,
    pub(crate) warnings: Vec<LoadWarning>,
    pub(crate) display_name_state: Option<DisplayNameState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LoadWarning {
    AccountsServiceUnavailable(String),
    AvatarUnavailable { path: PathBuf, reason: String },
}

impl std::fmt::Display for LoadWarning {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AccountsServiceUnavailable(reason) => formatter.write_str(reason),
            Self::AvatarUnavailable { path, reason } => {
                write!(formatter, "could not use {}: {reason}", path.display())
            }
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum ProfileLoadError {
    #[error("the current user account could not be read: {0}")]
    User(#[from] UserLookupError),
}

#[derive(Debug)]
pub(crate) struct ProfileStore<U = NssUserSource, A = SystemAccountsSource> {
    users: U,
    accounts: A,
}

impl Default for ProfileStore {
    fn default() -> Self {
        Self {
            users: NssUserSource,
            accounts: SystemAccountsSource,
        }
    }
}

impl ProfileStore {
    pub(crate) fn load_current() -> Result<LoadedProfile, ProfileLoadError> {
        Self::default().load()
    }
}

impl<U: UserSource, A: AccountsSource> ProfileStore<U, A> {
    #[cfg(test)]
    fn new(users: U, accounts: A) -> Self {
        Self { users, accounts }
    }

    fn load(&self) -> Result<LoadedProfile, ProfileLoadError> {
        let user = self.users.current_user()?;
        let mut warnings = Vec::new();
        let (account, accounts_available) = match self.accounts.details_for_uid(user.uid) {
            Ok(details) => (details, true),
            Err(error) => {
                warnings.push(LoadWarning::AccountsServiceUnavailable(error.to_string()));
                (AccountDetails::default(), false)
            }
        };

        let avatars = load_avatar_candidates(&user, &account, &mut warnings);
        let display_name = if accounts_available {
            account.display_name
        } else {
            user.display_name
        };
        let profile = Profile {
            display_name: display_name.clone(),
            avatars,
        };
        let display_name_state =
            accounts_available.then(|| DisplayNameState::new(user.uid, &display_name));

        Ok(LoadedProfile {
            profile,
            warnings,
            display_name_state,
        })
    }
}

fn load_avatar_candidates(
    user: &UserRecord,
    account: &AccountDetails,
    warnings: &mut Vec<LoadWarning>,
) -> Vec<Avatar> {
    let face_path = user.home_directory.join(".face");
    let mut candidates: Vec<AvatarCandidate> = vec![(face_path.clone(), AvatarSource::Face)];
    if let Some(path) = &account.icon_file
        && path != &face_path
    {
        candidates.push((path.clone(), AvatarSource::AccountsService));
    }

    candidates
        .into_iter()
        .filter_map(|(path, source)| match read_avatar(&path) {
            Ok(bytes) => Some(Avatar::new(bytes, source(path))),
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
    use crate::core::account_service::AccountsError;
    #[derive(Clone)]
    struct FakeUsers(UserRecord);

    impl UserSource for FakeUsers {
        fn current_user(&self) -> Result<UserRecord, UserLookupError> {
            Ok(self.0.clone())
        }
    }

    enum FakeAccounts {
        Available(AccountDetails),
        Unavailable,
    }

    impl AccountsSource for FakeAccounts {
        fn details_for_uid(&self, _uid: u32) -> Result<AccountDetails, AccountsError> {
            match self {
                Self::Available(details) => Ok(details.clone()),
                Self::Unavailable => Err(AccountsError::test_unavailable()),
            }
        }
    }

    fn user() -> UserRecord {
        UserRecord {
            uid: 1000,
            home_directory: PathBuf::from("/path/that/does/not/exist"),
            display_name: "Fallback Name".to_owned(),
        }
    }

    #[test]
    fn combines_nss_and_accounts_service_data() {
        let store = ProfileStore::new(
            FakeUsers(user()),
            FakeAccounts::Available(AccountDetails {
                display_name: "Ada Lovelace".to_owned(),
                icon_file: None,
            }),
        );

        let loaded = store.load().unwrap();

        assert_eq!(loaded.profile.display_name, "Ada Lovelace");
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
        let store = ProfileStore::new(FakeUsers(user()), FakeAccounts::Unavailable);

        let loaded = store.load().unwrap();

        assert_eq!(loaded.profile.display_name, "Fallback Name");
        assert!(loaded.display_name_state.is_none());
        assert!(matches!(
            loaded.warnings.as_slice(),
            [LoadWarning::AccountsServiceUnavailable(_)]
        ));
    }
}
