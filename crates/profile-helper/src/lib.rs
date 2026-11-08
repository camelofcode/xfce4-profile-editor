// SPDX-License-Identifier: GPL-2.0-or-later

//! Pure protocol and update preparation for the privileged profile helper.
//!
//! This crate intentionally does not execute the prepared command yet.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use xfce4_profile_data::{Gecos, GecosUpdate, ValidationError};

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_REQUEST_BYTES: usize = 4096;
pub const USERMOD_PATH: &str = "/usr/sbin/usermod";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGecosRequest {
    pub version: u8,
    pub expected_gecos: String,
    pub full_name: String,
    pub office_phone: String,
    pub home_phone: String,
}

impl UpdateGecosRequest {
    fn update(&self) -> GecosUpdate {
        GecosUpdate {
            full_name: self.full_name.clone(),
            office_phone: self.office_phone.clone(),
            home_phone: self.home_phone.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    InvalidRequest,
    UnsupportedVersion,
    Validation,
    StaleData,
    AccountNotLocal,
    AmbiguousAccount,
    System,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HelperResponse {
    Success,
    Error { kind: ErrorKind, message: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAccount {
    pub username: String,
    pub uid: u32,
    pub gecos: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub program: &'static str,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedUpdate {
    pub account: LocalAccount,
    pub original_gecos: String,
    pub new_gecos: String,
    pub command: CommandSpec,
}

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("the helper request exceeds the {MAX_REQUEST_BYTES}-byte limit")]
    TooLarge,
    #[error("the helper request is not valid JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("protocol version {actual} is unsupported; expected {PROTOCOL_VERSION}")]
    UnsupportedVersion { actual: u8 },
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum AccountError {
    #[error("UID 0 cannot be changed by the profile helper")]
    RootAccount,
    #[error("the local passwd database is malformed at line {line}")]
    MalformedDatabase { line: usize },
    #[error("UID {uid} has no local passwd entry")]
    NotLocal { uid: u32 },
    #[error("UID {uid} has more than one local passwd entry")]
    Ambiguous { uid: u32 },
}

#[derive(Debug, Error)]
pub enum PrepareError {
    #[error(transparent)]
    Account(#[from] AccountError),
    #[error("the account changed after it was loaded")]
    StaleData,
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("protocol version {actual} is unsupported; expected {PROTOCOL_VERSION}")]
    UnsupportedVersion { actual: u8 },
}

pub fn parse_request(input: &[u8]) -> Result<UpdateGecosRequest, RequestError> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(RequestError::TooLarge);
    }
    let request: UpdateGecosRequest =
        serde_json::from_slice(input).map_err(RequestError::InvalidJson)?;
    if request.version != PROTOCOL_VERSION {
        return Err(RequestError::UnsupportedVersion {
            actual: request.version,
        });
    }
    Ok(request)
}

pub fn find_local_account(passwd: &str, caller_uid: u32) -> Result<LocalAccount, AccountError> {
    if caller_uid == 0 {
        return Err(AccountError::RootAccount);
    }

    let mut matching_account = None;
    for (index, line) in passwd.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split(':').collect();
        if fields.len() != 7 || fields[0].is_empty() {
            return Err(AccountError::MalformedDatabase { line: index + 1 });
        }
        let uid = fields[2]
            .parse::<u32>()
            .map_err(|_| AccountError::MalformedDatabase { line: index + 1 })?;
        if uid != caller_uid {
            continue;
        }
        if matching_account.is_some() {
            return Err(AccountError::Ambiguous { uid: caller_uid });
        }
        matching_account = Some(LocalAccount {
            username: fields[0].to_owned(),
            uid,
            gecos: fields[4].to_owned(),
        });
    }

    matching_account.ok_or(AccountError::NotLocal { uid: caller_uid })
}

pub fn prepare_update(
    request: &UpdateGecosRequest,
    caller_uid: u32,
    passwd: &str,
) -> Result<PreparedUpdate, PrepareError> {
    if request.version != PROTOCOL_VERSION {
        return Err(PrepareError::UnsupportedVersion {
            actual: request.version,
        });
    }
    let account = find_local_account(passwd, caller_uid)?;
    if account.gecos != request.expected_gecos {
        return Err(PrepareError::StaleData);
    }

    let original_gecos = account.gecos.clone();
    let mut gecos = Gecos::parse(&original_gecos);
    gecos.apply(&request.update())?;
    let new_gecos = gecos.to_string();
    let command = CommandSpec {
        program: USERMOD_PATH,
        arguments: vec![
            "--comment".to_owned(),
            new_gecos.clone(),
            "--".to_owned(),
            account.username.clone(),
        ],
    };

    Ok(PreparedUpdate {
        account,
        original_gecos,
        new_gecos,
        command,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWD: &str = concat!(
        "root:x:0:0:root:/root:/bin/bash\n",
        "ada:x:1000:1000:Ada,Room 42,work,home,other,extra:/home/ada:/bin/bash\n",
    );

    fn request() -> UpdateGecosRequest {
        UpdateGecosRequest {
            version: PROTOCOL_VERSION,
            expected_gecos: "Ada,Room 42,work,home,other,extra".to_owned(),
            full_name: "Ada Lovelace".to_owned(),
            office_phone: "+44 20 7946 0001".to_owned(),
            home_phone: "+44 20 7946 0002".to_owned(),
        }
    }

    #[test]
    fn parses_a_bounded_versioned_request() {
        let json = serde_json::to_vec(&request()).unwrap();
        assert_eq!(parse_request(&json).unwrap(), request());

        let oversized = vec![b' '; MAX_REQUEST_BYTES + 1];
        assert!(matches!(
            parse_request(&oversized),
            Err(RequestError::TooLarge)
        ));
    }

    #[test]
    fn rejects_unknown_fields_and_versions() {
        let unknown = br#"{
            "version": 1,
            "expected_gecos": "Ada",
            "full_name": "Ada",
            "office_phone": "",
            "home_phone": "",
            "target_username": "root"
        }"#;
        assert!(matches!(
            parse_request(unknown),
            Err(RequestError::InvalidJson(_))
        ));

        let mut unsupported = request();
        unsupported.version += 1;
        let json = serde_json::to_vec(&unsupported).unwrap();
        assert!(matches!(
            parse_request(&json),
            Err(RequestError::UnsupportedVersion { .. })
        ));
    }

    #[test]
    fn selects_exactly_one_non_root_local_account() {
        let account = find_local_account(PASSWD, 1000).unwrap();
        assert_eq!(account.username, "ada");
        assert_eq!(account.gecos, "Ada,Room 42,work,home,other,extra");

        assert_eq!(
            find_local_account(PASSWD, 0),
            Err(AccountError::RootAccount)
        );
        assert_eq!(
            find_local_account(PASSWD, 2000),
            Err(AccountError::NotLocal { uid: 2000 })
        );
    }

    #[test]
    fn rejects_duplicate_and_malformed_local_records() {
        let duplicate = format!("{PASSWD}ada-copy:x:1000:1000:Ada:/home/copy:/bin/sh\n");
        assert_eq!(
            find_local_account(&duplicate, 1000),
            Err(AccountError::Ambiguous { uid: 1000 })
        );
        assert_eq!(
            find_local_account("broken:x:not-a-uid:1::::\n", 1000),
            Err(AccountError::MalformedDatabase { line: 1 })
        );
    }

    #[test]
    fn refuses_stale_data_without_preparing_a_command() {
        let mut stale = request();
        stale.expected_gecos = "older value".to_owned();

        assert!(matches!(
            prepare_update(&stale, 1000, PASSWD),
            Err(PrepareError::StaleData)
        ));
    }

    #[test]
    fn validates_all_new_values_at_the_helper_boundary() {
        let mut invalid = request();
        invalid.office_phone = "unsafe,value".to_owned();

        assert!(matches!(
            prepare_update(&invalid, 1000, PASSWD),
            Err(PrepareError::Validation(_))
        ));
    }

    #[test]
    fn preserves_hidden_fields_and_prepares_only_the_fixed_command() {
        let prepared = prepare_update(&request(), 1000, PASSWD).unwrap();

        assert_eq!(prepared.account.username, "ada");
        assert_eq!(
            prepared.new_gecos,
            "Ada Lovelace,Room 42,+44 20 7946 0001,+44 20 7946 0002,other,extra"
        );
        assert_eq!(prepared.command.program, "/usr/sbin/usermod");
        assert_eq!(
            prepared.command.arguments,
            ["--comment", &prepared.new_gecos, "--", "ada",]
        );
    }

    #[test]
    fn response_protocol_is_structured() {
        let response = HelperResponse::Error {
            kind: ErrorKind::StaleData,
            message: "the account changed after it was loaded".to_owned(),
        };
        let json = serde_json::to_string(&response).unwrap();

        assert_eq!(
            serde_json::from_str::<HelperResponse>(&json).unwrap(),
            response
        );
    }
}
