// SPDX-License-Identifier: GPL-2.0-or-later

//! Current-user lookup through the system's NSS configuration.

use std::path::PathBuf;
use std::process::Command;

use thiserror::Error;

use super::gecos::Gecos;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UserRecord {
    pub(crate) uid: u32,
    pub(crate) username: String,
    pub(crate) home_directory: PathBuf,
    pub(crate) gecos: Gecos,
}

pub(crate) trait UserSource {
    fn current_user(&self) -> Result<UserRecord, UserLookupError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NssUserSource;

impl UserSource for NssUserSource {
    fn current_user(&self) -> Result<UserRecord, UserLookupError> {
        let uid = rustix::process::geteuid().as_raw();
        let output = Command::new("getent")
            .args(["passwd", &uid.to_string()])
            .output()
            .map_err(UserLookupError::StartGetent)?;

        if !output.status.success() {
            return Err(UserLookupError::GetentFailed(output.status.code()));
        }

        let stdout = std::str::from_utf8(&output.stdout)?;
        parse_passwd_record(stdout.trim_end_matches(['\r', '\n']), uid)
    }
}

#[derive(Debug, Error)]
pub(crate) enum UserLookupError {
    #[error("could not run getent: {0}")]
    StartGetent(#[source] std::io::Error),
    #[error("getent failed with exit status {0:?}")]
    GetentFailed(Option<i32>),
    #[error("getent returned non-UTF-8 account data: {0}")]
    InvalidEncoding(#[from] std::str::Utf8Error),
    #[error("getent returned a malformed passwd record")]
    MalformedRecord,
    #[error("getent returned UID {actual}, expected {expected}")]
    WrongUid { expected: u32, actual: u32 },
}

fn parse_passwd_record(value: &str, expected_uid: u32) -> Result<UserRecord, UserLookupError> {
    let mut fields = value.split(':');
    let username = fields.next().ok_or(UserLookupError::MalformedRecord)?;
    let _password = fields.next().ok_or(UserLookupError::MalformedRecord)?;
    let uid = fields
        .next()
        .ok_or(UserLookupError::MalformedRecord)?
        .parse()
        .map_err(|_| UserLookupError::MalformedRecord)?;
    let _gid = fields.next().ok_or(UserLookupError::MalformedRecord)?;
    let gecos = fields.next().ok_or(UserLookupError::MalformedRecord)?;
    let home_directory = fields.next().ok_or(UserLookupError::MalformedRecord)?;
    let _shell = fields.next().ok_or(UserLookupError::MalformedRecord)?;

    if fields.next().is_some() || username.is_empty() || home_directory.is_empty() {
        return Err(UserLookupError::MalformedRecord);
    }
    if uid != expected_uid {
        return Err(UserLookupError::WrongUid {
            expected: expected_uid,
            actual: uid,
        });
    }

    Ok(UserRecord {
        uid,
        username: username.to_owned(),
        home_directory: PathBuf::from(home_directory),
        gecos: Gecos::parse(gecos),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_complete_passwd_record() {
        let user = parse_passwd_record(
            "ada:x:1000:1000:Ada Lovelace,Room 1,0123,0456,Notes:/home/ada:/bin/bash",
            1000,
        )
        .unwrap();

        assert_eq!(user.username, "ada");
        assert_eq!(user.home_directory, PathBuf::from("/home/ada"));
        assert_eq!(user.gecos.full_name(), "Ada Lovelace");
        assert_eq!(user.gecos.office_phone(), "0123");
        assert_eq!(user.gecos.home_phone(), "0456");
    }

    #[test]
    fn rejects_extra_colon_delimited_fields() {
        let error = parse_passwd_record("ada:x:1000:1000:Ada:/home/ada:/bin/sh:extra", 1000);
        assert!(matches!(error, Err(UserLookupError::MalformedRecord)));
    }

    #[test]
    fn rejects_a_record_for_a_different_uid() {
        let error = parse_passwd_record("ada:x:1001:1000:Ada:/home/ada:/bin/sh", 1000);
        assert!(matches!(error, Err(UserLookupError::WrongUid { .. })));
    }
}
