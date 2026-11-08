// SPDX-License-Identifier: GPL-2.0-or-later

//! Pure protocol and update preparation for the privileged profile helper.

#![deny(unsafe_code)]

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::{Command, ExitCode, Stdio};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use xfce4_profile_data::{Gecos, GecosUpdate, ValidationError};

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_REQUEST_BYTES: usize = 4096;
pub const MAX_PASSWD_BYTES: usize = 1024 * 1024;
pub const USERMOD_PATH: &str = "/usr/sbin/usermod";
pub const UPDATE_GECOS_OPERATION: &str = "update-gecos";

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
    NotPrivileged,
    CommandFailed,
    VerificationFailed,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandResult {
    pub success: bool,
}

pub trait AccountBackend {
    type Error: std::fmt::Display;

    fn read_passwd(&mut self) -> Result<String, Self::Error>;
    fn run_command(&mut self, command: &CommandSpec) -> Result<CommandResult, Self::Error>;
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

pub fn process_request<B: AccountBackend>(
    input: &[u8],
    caller_uid: u32,
    backend: &mut B,
) -> HelperResponse {
    let request = match parse_request(input) {
        Ok(request) => request,
        Err(error) => return request_error_response(&error),
    };
    let passwd = match backend.read_passwd() {
        Ok(passwd) => passwd,
        Err(error) => return system_error("the local account database could not be read", &error),
    };
    let prepared = match prepare_update(&request, caller_uid, &passwd) {
        Ok(prepared) => prepared,
        Err(error) => return prepare_error_response(&error),
    };

    match backend.run_command(&prepared.command) {
        Ok(CommandResult { success: true }) => {}
        Ok(CommandResult { success: false }) => {
            return HelperResponse::Error {
                kind: ErrorKind::CommandFailed,
                message: "the system account tool rejected the update".to_owned(),
            };
        }
        Err(error) => return system_error("the system account tool could not be run", &error),
    }

    let updated_passwd = match backend.read_passwd() {
        Ok(passwd) => passwd,
        Err(error) => {
            return system_error(
                "the updated local account database could not be verified",
                &error,
            );
        }
    };
    match find_local_account(&updated_passwd, caller_uid) {
        Ok(account) if account.gecos == prepared.new_gecos => HelperResponse::Success,
        Ok(_) | Err(_) => HelperResponse::Error {
            kind: ErrorKind::VerificationFailed,
            message: "the account did not contain the expected data after the update".to_owned(),
        },
    }
}

#[derive(Debug, Error)]
enum SystemBackendError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("the local account database exceeds the safety limit")]
    PasswdTooLarge,
    #[error("the local account database is not UTF-8")]
    PasswdEncoding,
}

impl From<BoundedReadError> for SystemBackendError {
    fn from(error: BoundedReadError) -> Self {
        match error {
            BoundedReadError::TooLarge => Self::PasswdTooLarge,
            BoundedReadError::Io(error) => Self::Io(error),
        }
    }
}

#[derive(Debug, Default)]
struct SystemBackend;

impl AccountBackend for SystemBackend {
    type Error = SystemBackendError;

    fn read_passwd(&mut self) -> Result<String, Self::Error> {
        let bytes = read_bounded(File::open("/etc/passwd")?, MAX_PASSWD_BYTES)?;
        String::from_utf8(bytes).map_err(|_| SystemBackendError::PasswdEncoding)
    }

    fn run_command(&mut self, command: &CommandSpec) -> Result<CommandResult, Self::Error> {
        let status = Command::new(command.program)
            .args(&command.arguments)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        Ok(CommandResult {
            success: status.success(),
        })
    }
}

pub fn run_process() -> ExitCode {
    let response = run_process_inner();
    let exit_code = response_exit_code(&response);
    let mut stdout = io::stdout().lock();
    if serde_json::to_writer(&mut stdout, &response).is_err() || stdout.write_all(b"\n").is_err() {
        return ExitCode::from(5);
    }
    ExitCode::from(exit_code)
}

fn run_process_inner() -> HelperResponse {
    if rustix::process::geteuid().as_raw() != 0 {
        return HelperResponse::Error {
            kind: ErrorKind::NotPrivileged,
            message: "the profile helper must run through the installed Polkit action".to_owned(),
        };
    }
    if !has_exact_operation_argument(std::env::args_os().skip(1)) {
        return HelperResponse::Error {
            kind: ErrorKind::InvalidRequest,
            message: "the helper operation is missing or unsupported".to_owned(),
        };
    }
    let caller_uid = match std::env::var("PKEXEC_UID")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
    {
        Some(uid) => uid,
        None => {
            return HelperResponse::Error {
                kind: ErrorKind::InvalidRequest,
                message: "pkexec did not provide a valid caller UID".to_owned(),
            };
        }
    };
    let input = match read_bounded(io::stdin().lock(), MAX_REQUEST_BYTES) {
        Ok(input) => input,
        Err(BoundedReadError::TooLarge) => {
            return HelperResponse::Error {
                kind: ErrorKind::InvalidRequest,
                message: format!("the helper request exceeds the {MAX_REQUEST_BYTES}-byte limit"),
            };
        }
        Err(BoundedReadError::Io(error)) => {
            return system_error("the helper request could not be read", &error);
        }
    };

    process_request(&input, caller_uid, &mut SystemBackend)
}

fn has_exact_operation_argument(mut arguments: impl Iterator<Item = std::ffi::OsString>) -> bool {
    arguments.next().as_deref() == Some(OsStr::new(UPDATE_GECOS_OPERATION))
        && arguments.next().is_none()
}

#[derive(Debug, Error)]
enum BoundedReadError {
    #[error("input exceeds the safety limit")]
    TooLarge,
    #[error(transparent)]
    Io(#[from] io::Error),
}

fn read_bounded(reader: impl Read, maximum: usize) -> Result<Vec<u8>, BoundedReadError> {
    let mut bytes = Vec::with_capacity(maximum.min(8192));
    reader.take((maximum + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(BoundedReadError::TooLarge);
    }
    Ok(bytes)
}

fn request_error_response(error: &RequestError) -> HelperResponse {
    let kind = match error {
        RequestError::UnsupportedVersion { .. } => ErrorKind::UnsupportedVersion,
        RequestError::TooLarge | RequestError::InvalidJson(_) => ErrorKind::InvalidRequest,
    };
    HelperResponse::Error {
        kind,
        message: error.to_string(),
    }
}

fn prepare_error_response(error: &PrepareError) -> HelperResponse {
    let kind = match error {
        PrepareError::Account(AccountError::NotLocal { .. } | AccountError::RootAccount) => {
            ErrorKind::AccountNotLocal
        }
        PrepareError::Account(AccountError::Ambiguous { .. }) => ErrorKind::AmbiguousAccount,
        PrepareError::Account(AccountError::MalformedDatabase { .. }) => ErrorKind::System,
        PrepareError::StaleData => ErrorKind::StaleData,
        PrepareError::Validation(_) => ErrorKind::Validation,
        PrepareError::UnsupportedVersion { .. } => ErrorKind::UnsupportedVersion,
    };
    HelperResponse::Error {
        kind,
        message: error.to_string(),
    }
}

fn system_error(context: &str, error: &impl std::fmt::Display) -> HelperResponse {
    eprintln!("{context}: {error}");
    HelperResponse::Error {
        kind: ErrorKind::System,
        message: context.to_owned(),
    }
}

fn response_exit_code(response: &HelperResponse) -> u8 {
    match response {
        HelperResponse::Success => 0,
        HelperResponse::Error {
            kind: ErrorKind::InvalidRequest | ErrorKind::UnsupportedVersion | ErrorKind::Validation,
            ..
        } => 2,
        HelperResponse::Error {
            kind: ErrorKind::StaleData,
            ..
        } => 3,
        HelperResponse::Error {
            kind:
                ErrorKind::AccountNotLocal | ErrorKind::AmbiguousAccount | ErrorKind::NotPrivileged,
            ..
        } => 4,
        HelperResponse::Error { .. } => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::ffi::OsString;
    use std::io::Cursor;

    const PASSWD: &str = concat!(
        "root:x:0:0:root:/root:/bin/bash\n",
        "ada:x:1000:1000:Ada,Room 42,work,home,other,extra:/home/ada:/bin/bash\n",
    );
    const UPDATED_PASSWD: &str = concat!(
        "root:x:0:0:root:/root:/bin/bash\n",
        "ada:x:1000:1000:Ada Lovelace,Room 42,+44 20 7946 0001,+44 20 7946 0002,other,extra:/home/ada:/bin/bash\n",
    );

    #[derive(Clone, Copy, Debug)]
    struct FakeError(&'static str);

    impl std::fmt::Display for FakeError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(self.0)
        }
    }

    struct FakeBackend {
        passwd_reads: VecDeque<Result<String, FakeError>>,
        command_result: Result<CommandResult, FakeError>,
        command: Option<CommandSpec>,
    }

    impl FakeBackend {
        fn success(after: &str) -> Self {
            Self {
                passwd_reads: [Ok(PASSWD.to_owned()), Ok(after.to_owned())].into(),
                command_result: Ok(CommandResult { success: true }),
                command: None,
            }
        }
    }

    impl AccountBackend for FakeBackend {
        type Error = FakeError;

        fn read_passwd(&mut self) -> Result<String, Self::Error> {
            self.passwd_reads
                .pop_front()
                .expect("unexpected passwd read")
        }

        fn run_command(&mut self, command: &CommandSpec) -> Result<CommandResult, Self::Error> {
            self.command = Some(command.clone());
            self.command_result
        }
    }

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

    #[test]
    fn complete_flow_runs_the_fixed_command_and_verifies_the_result() {
        let input = serde_json::to_vec(&request()).unwrap();
        let mut backend = FakeBackend::success(UPDATED_PASSWD);

        assert_eq!(
            process_request(&input, 1000, &mut backend),
            HelperResponse::Success
        );
        assert_eq!(backend.command.unwrap().program, USERMOD_PATH);
        assert!(backend.passwd_reads.is_empty());
    }

    #[test]
    fn command_rejection_and_post_write_mismatch_are_distinct() {
        let input = serde_json::to_vec(&request()).unwrap();
        let mut rejected = FakeBackend::success(UPDATED_PASSWD);
        rejected.command_result = Ok(CommandResult { success: false });
        assert!(matches!(
            process_request(&input, 1000, &mut rejected),
            HelperResponse::Error {
                kind: ErrorKind::CommandFailed,
                ..
            }
        ));
        assert_eq!(rejected.passwd_reads.len(), 1);

        let mut mismatched = FakeBackend::success(PASSWD);
        assert!(matches!(
            process_request(&input, 1000, &mut mismatched),
            HelperResponse::Error {
                kind: ErrorKind::VerificationFailed,
                ..
            }
        ));
    }

    #[test]
    fn bounded_reader_stops_after_one_byte_over_the_limit() {
        assert_eq!(read_bounded(Cursor::new(b"1234"), 4).unwrap(), b"1234");
        assert!(matches!(
            read_bounded(Cursor::new(b"12345"), 4),
            Err(BoundedReadError::TooLarge)
        ));
    }

    #[test]
    fn executable_accepts_only_the_policy_bound_operation() {
        assert!(has_exact_operation_argument(
            [OsString::from(UPDATE_GECOS_OPERATION)].into_iter()
        ));
        assert!(!has_exact_operation_argument([].into_iter()));
        assert!(!has_exact_operation_argument(
            [OsString::from("other-operation")].into_iter()
        ));
        assert!(!has_exact_operation_argument(
            [
                OsString::from(UPDATE_GECOS_OPERATION),
                OsString::from("unexpected"),
            ]
            .into_iter()
        ));
    }

    #[test]
    fn response_exit_codes_keep_recoverable_states_distinct() {
        assert_eq!(response_exit_code(&HelperResponse::Success), 0);
        assert_eq!(
            response_exit_code(&HelperResponse::Error {
                kind: ErrorKind::Validation,
                message: String::new(),
            }),
            2
        );
        assert_eq!(
            response_exit_code(&HelperResponse::Error {
                kind: ErrorKind::StaleData,
                message: String::new(),
            }),
            3
        );
        assert_eq!(
            response_exit_code(&HelperResponse::Error {
                kind: ErrorKind::AccountNotLocal,
                message: String::new(),
            }),
            4
        );
        assert_eq!(
            response_exit_code(&HelperResponse::Error {
                kind: ErrorKind::CommandFailed,
                message: String::new(),
            }),
            5
        );
    }
}
