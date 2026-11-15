// SPDX-License-Identifier: GPL-2.0-or-later

//! Current-user identity needed by the profile domain.

use std::ffi::OsString;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CurrentUser {
    pub(crate) uid: u32,
    pub(crate) home_directory: PathBuf,
}

pub(crate) fn current_user() -> Result<CurrentUser, CurrentUserError> {
    current_user_from(
        rustix::process::geteuid().as_raw(),
        std::env::var_os("HOME"),
    )
}

#[derive(Debug, Error)]
pub(crate) enum CurrentUserError {
    #[error("HOME is not set for the current desktop session")]
    MissingHomeDirectory,
    #[error("HOME does not identify an absolute home directory")]
    InvalidHomeDirectory,
}

fn current_user_from(
    uid: u32,
    home_directory: Option<OsString>,
) -> Result<CurrentUser, CurrentUserError> {
    let home_directory = home_directory
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or(CurrentUserError::MissingHomeDirectory)?;
    if !home_directory.is_absolute() {
        return Err(CurrentUserError::InvalidHomeDirectory);
    }

    Ok(CurrentUser {
        uid,
        home_directory,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_uid_and_absolute_home_directory() {
        let user = current_user_from(1000, Some(OsString::from("/home/ada"))).unwrap();

        assert_eq!(user.uid, 1000);
        assert_eq!(user.home_directory, PathBuf::from("/home/ada"));
    }

    #[test]
    fn rejects_missing_empty_and_relative_home_directories() {
        assert!(matches!(
            current_user_from(1000, None),
            Err(CurrentUserError::MissingHomeDirectory)
        ));
        assert!(matches!(
            current_user_from(1000, Some(OsString::new())),
            Err(CurrentUserError::MissingHomeDirectory)
        ));
        assert!(matches!(
            current_user_from(1000, Some(OsString::from("home/ada"))),
            Err(CurrentUserError::InvalidHomeDirectory)
        ));
    }
}
