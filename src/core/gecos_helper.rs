// SPDX-License-Identifier: GPL-2.0-or-later

//! Typed client for the installed privileged GECOS helper.

use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;

use thiserror::Error;
use xfce4_profile_data::{GecosUpdate, ValidationError};
use xfce4_profile_editor_helper::{
    ErrorKind, HelperResponse, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, PROTOCOL_VERSION,
    UPDATE_GECOS_OPERATION, UpdateGecosRequest, response_exit_code,
};

const PKEXEC_PATH: &str = "/usr/bin/pkexec";
const HELPER_PATH: &str = "/usr/libexec/xfce4-profile-editor-helper";
const PKEXEC_CANCELLED: i32 = 126;
const PKEXEC_NOT_AUTHORIZED: i32 = 127;
const MAX_DIAGNOSTIC_BYTES: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct GecosChange {
    pub(super) expected_gecos: String,
    pub(super) full_name: String,
    pub(super) office_phone: String,
    pub(super) home_phone: String,
}

impl GecosChange {
    fn update(&self) -> GecosUpdate {
        GecosUpdate {
            full_name: self.full_name.clone(),
            office_phone: self.office_phone.clone(),
            home_phone: self.home_phone.clone(),
        }
    }

    fn helper_request(&self) -> UpdateGecosRequest {
        UpdateGecosRequest {
            version: PROTOCOL_VERSION,
            expected_gecos: self.expected_gecos.clone(),
            full_name: self.full_name.clone(),
            office_phone: self.office_phone.clone(),
            home_phone: self.home_phone.clone(),
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum GecosWriteError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("authentication was cancelled")]
    AuthenticationCancelled,
    #[error("authorization was denied")]
    AuthorizationDenied,
    #[error("the account changed after it was loaded")]
    StaleData,
    #[error("the current account is not local")]
    AccountNotLocal,
    #[error("the current UID has ambiguous local account records")]
    AmbiguousAccount,
    #[error("the helper did not run with the required privilege: {0}")]
    NotPrivileged(String),
    #[error("the helper rejected its request: {0}")]
    InvalidRequest(String),
    #[error("the helper protocol version is unsupported: {0}")]
    UnsupportedVersion(String),
    #[error("the helper rejected a value: {0}")]
    HelperValidation(String),
    #[error("the system account tool rejected the update: {0}")]
    CommandFailed(String),
    #[error("the updated account could not be verified: {0}")]
    VerificationFailed(String),
    #[error("the helper reported a system failure: {0}")]
    System(String),
    #[error("the helper request could not be encoded: {0}")]
    EncodeRequest(#[source] serde_json::Error),
    #[error("the helper request exceeded the {MAX_REQUEST_BYTES}-byte limit")]
    RequestTooLarge,
    #[error("the helper response is not valid protocol JSON: {0}")]
    InvalidResponse(#[source] serde_json::Error),
    #[error("the helper response exceeded the {MAX_RESPONSE_BYTES}-byte limit")]
    ResponseTooLarge,
    #[error("the helper terminated without an exit code")]
    Terminated,
    #[error("the helper response implied exit {expected}, but the process exited {actual}")]
    ExitMismatch { expected: u8, actual: i32 },
    #[error(transparent)]
    Transport(#[from] HelperTransportError),
}

#[derive(Debug, Error)]
pub(crate) enum HelperTransportError {
    #[error("could not start pkexec: {0}")]
    Start(#[source] io::Error),
    #[error("pkexec did not expose its {0} pipe")]
    MissingPipe(&'static str),
    #[error("the helper request could not be written: {0}")]
    WriteRequest(#[source] io::Error),
    #[error("the helper process could not be waited for: {0}")]
    Wait(#[source] io::Error),
    #[error("the helper response could not be read: {0}")]
    ReadResponse(#[source] io::Error),
    #[error("the helper diagnostic output could not be read: {0}")]
    ReadDiagnostic(#[source] io::Error),
    #[error("a helper output reader stopped unexpectedly")]
    ReaderStopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ProcessOutput {
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stdout_truncated: bool,
    stderr: Vec<u8>,
    stderr_truncated: bool,
}

pub(super) trait HelperRunner {
    fn run(&self, request: &[u8]) -> Result<ProcessOutput, HelperTransportError>;
}

pub(super) trait GecosClient {
    fn update(&self, change: &GecosChange) -> Result<(), GecosWriteError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SystemHelperRunner;

impl HelperRunner for SystemHelperRunner {
    fn run(&self, request: &[u8]) -> Result<ProcessOutput, HelperTransportError> {
        let (program, arguments) = helper_command();
        let mut child = Command::new(program)
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(HelperTransportError::Start)?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or(HelperTransportError::MissingPipe("standard-input"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(HelperTransportError::MissingPipe("standard-output"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or(HelperTransportError::MissingPipe("standard-error"))?;

        thread::scope(|scope| {
            let stdout_reader = scope.spawn(move || read_limited(stdout, MAX_RESPONSE_BYTES));
            let stderr_reader = scope.spawn(move || read_limited(stderr, MAX_DIAGNOSTIC_BYTES));

            let write_result = stdin
                .write_all(request)
                .map_err(HelperTransportError::WriteRequest);
            drop(stdin);
            let wait_result = child.wait().map_err(HelperTransportError::Wait);
            let stdout_result = stdout_reader
                .join()
                .map_err(|_| HelperTransportError::ReaderStopped)?
                .map_err(HelperTransportError::ReadResponse);
            let stderr_result = stderr_reader
                .join()
                .map_err(|_| HelperTransportError::ReaderStopped)?
                .map_err(HelperTransportError::ReadDiagnostic);

            write_result?;
            let status = wait_result?;
            let (stdout, stdout_truncated) = stdout_result?;
            let (stderr, stderr_truncated) = stderr_result?;
            Ok(ProcessOutput {
                exit_code: status.code(),
                stdout,
                stdout_truncated,
                stderr,
                stderr_truncated,
            })
        })
    }
}

#[derive(Debug)]
pub(super) struct GecosHelperClient<R = SystemHelperRunner> {
    runner: R,
}

impl Default for GecosHelperClient {
    fn default() -> Self {
        Self {
            runner: SystemHelperRunner,
        }
    }
}

impl<R: HelperRunner> GecosHelperClient<R> {
    #[cfg(test)]
    fn new(runner: R) -> Self {
        Self { runner }
    }

    pub(super) fn update(&self, change: &GecosChange) -> Result<(), GecosWriteError> {
        change.update().validate()?;
        let request =
            serde_json::to_vec(&change.helper_request()).map_err(GecosWriteError::EncodeRequest)?;
        if request.len() > MAX_REQUEST_BYTES {
            return Err(GecosWriteError::RequestTooLarge);
        }
        let output = self.runner.run(&request)?;
        map_output(output)
    }
}

impl<R: HelperRunner> GecosClient for GecosHelperClient<R> {
    fn update(&self, change: &GecosChange) -> Result<(), GecosWriteError> {
        Self::update(self, change)
    }
}

fn helper_command() -> (&'static str, [&'static str; 2]) {
    (PKEXEC_PATH, [HELPER_PATH, UPDATE_GECOS_OPERATION])
}

fn read_limited(reader: impl Read, maximum: usize) -> Result<(Vec<u8>, bool), io::Error> {
    let mut bytes = Vec::with_capacity(maximum.min(8192));
    reader.take((maximum + 1) as u64).read_to_end(&mut bytes)?;
    let truncated = bytes.len() > maximum;
    bytes.truncate(maximum);
    Ok((bytes, truncated))
}

fn map_output(output: ProcessOutput) -> Result<(), GecosWriteError> {
    match output.exit_code {
        Some(PKEXEC_CANCELLED) => return Err(GecosWriteError::AuthenticationCancelled),
        Some(PKEXEC_NOT_AUTHORIZED) => return Err(GecosWriteError::AuthorizationDenied),
        None => return Err(GecosWriteError::Terminated),
        Some(_) => {}
    }
    if output.stdout_truncated {
        return Err(GecosWriteError::ResponseTooLarge);
    }

    let response: HelperResponse =
        serde_json::from_slice(&output.stdout).map_err(GecosWriteError::InvalidResponse)?;
    let actual = output.exit_code.expect("an exit code was checked above");
    let expected = response_exit_code(&response);
    if actual != i32::from(expected) {
        return Err(GecosWriteError::ExitMismatch { expected, actual });
    }

    match response {
        HelperResponse::Success => Ok(()),
        HelperResponse::Error { kind, message } => Err(match kind {
            ErrorKind::InvalidRequest => GecosWriteError::InvalidRequest(message),
            ErrorKind::UnsupportedVersion => GecosWriteError::UnsupportedVersion(message),
            ErrorKind::Validation => GecosWriteError::HelperValidation(message),
            ErrorKind::StaleData => GecosWriteError::StaleData,
            ErrorKind::AccountNotLocal => GecosWriteError::AccountNotLocal,
            ErrorKind::AmbiguousAccount => GecosWriteError::AmbiguousAccount,
            ErrorKind::NotPrivileged => GecosWriteError::NotPrivileged(message),
            ErrorKind::CommandFailed => GecosWriteError::CommandFailed(message),
            ErrorKind::VerificationFailed => GecosWriteError::VerificationFailed(message),
            ErrorKind::System => GecosWriteError::System(message),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::io::Cursor;

    use super::*;

    struct FakeRunner {
        output: RefCell<Option<Result<ProcessOutput, HelperTransportError>>>,
        request: RefCell<Vec<u8>>,
        calls: Cell<usize>,
    }

    impl FakeRunner {
        fn with_output(output: ProcessOutput) -> Self {
            Self {
                output: RefCell::new(Some(Ok(output))),
                request: RefCell::new(Vec::new()),
                calls: Cell::new(0),
            }
        }
    }

    impl HelperRunner for FakeRunner {
        fn run(&self, request: &[u8]) -> Result<ProcessOutput, HelperTransportError> {
            self.calls.set(self.calls.get() + 1);
            self.request.replace(request.to_vec());
            self.output
                .borrow_mut()
                .take()
                .expect("fake runner was called more than once")
        }
    }

    fn change() -> GecosChange {
        GecosChange {
            expected_gecos: "Ada,room,work,home,other,extra".to_owned(),
            full_name: "Ada Lovelace".to_owned(),
            office_phone: "+44 20 1".to_owned(),
            home_phone: "+44 20 2".to_owned(),
        }
    }

    fn output(exit_code: i32, response: HelperResponse) -> ProcessOutput {
        ProcessOutput {
            exit_code: Some(exit_code),
            stdout: serde_json::to_vec(&response).unwrap(),
            stdout_truncated: false,
            stderr: Vec::new(),
            stderr_truncated: false,
        }
    }

    fn error_output(kind: ErrorKind) -> ProcessOutput {
        let response = HelperResponse::Error {
            kind,
            message: "diagnostic".to_owned(),
        };
        output(i32::from(response_exit_code(&response)), response)
    }

    #[test]
    fn command_is_fixed_to_the_installed_policy_binding() {
        assert_eq!(
            helper_command(),
            (
                "/usr/bin/pkexec",
                ["/usr/libexec/xfce4-profile-editor-helper", "update-gecos"]
            )
        );
    }

    #[test]
    fn validates_and_serializes_the_versioned_request() {
        let runner = FakeRunner::with_output(output(0, HelperResponse::Success));
        let client = GecosHelperClient::new(runner);

        client.update(&change()).unwrap();

        let request: UpdateGecosRequest =
            serde_json::from_slice(&client.runner.request.borrow()).unwrap();
        assert_eq!(request.version, PROTOCOL_VERSION);
        assert_eq!(request.expected_gecos, "Ada,room,work,home,other,extra");
        assert_eq!(request.full_name, "Ada Lovelace");
        assert_eq!(request.office_phone, "+44 20 1");
        assert_eq!(request.home_phone, "+44 20 2");
        assert_eq!(client.runner.calls.get(), 1);
    }

    #[test]
    fn rejects_invalid_values_before_starting_the_runner() {
        let runner = FakeRunner::with_output(output(0, HelperResponse::Success));
        let client = GecosHelperClient::new(runner);
        let mut invalid = change();
        invalid.full_name = "hostile,value".to_owned();

        assert!(matches!(
            client.update(&invalid),
            Err(GecosWriteError::Validation(_))
        ));
        assert_eq!(client.runner.calls.get(), 0);
    }

    #[test]
    fn rejects_an_oversized_request_before_starting_the_runner() {
        let runner = FakeRunner::with_output(output(0, HelperResponse::Success));
        let client = GecosHelperClient::new(runner);
        let mut oversized = change();
        oversized.expected_gecos = "x".repeat(MAX_REQUEST_BYTES);

        assert!(matches!(
            client.update(&oversized),
            Err(GecosWriteError::RequestTooLarge)
        ));
        assert_eq!(client.runner.calls.get(), 0);
    }

    #[test]
    fn maps_pkexec_cancellation_and_denial_without_parsing_stdout() {
        for (exit_code, expected_cancelled) in [(126, true), (127, false)] {
            let runner = FakeRunner::with_output(ProcessOutput {
                exit_code: Some(exit_code),
                stdout: b"not JSON".to_vec(),
                stdout_truncated: false,
                stderr: b"pkexec diagnostic".to_vec(),
                stderr_truncated: false,
            });
            let result = GecosHelperClient::new(runner).update(&change());
            if expected_cancelled {
                assert!(matches!(
                    result,
                    Err(GecosWriteError::AuthenticationCancelled)
                ));
            } else {
                assert!(matches!(result, Err(GecosWriteError::AuthorizationDenied)));
            }
        }
    }

    #[test]
    fn maps_recoverable_helper_outcomes_to_distinct_errors() {
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(ErrorKind::StaleData)))
                .update(&change()),
            Err(GecosWriteError::StaleData)
        ));
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(
                ErrorKind::AccountNotLocal
            )))
            .update(&change()),
            Err(GecosWriteError::AccountNotLocal)
        ));
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(ErrorKind::Validation)))
                .update(&change()),
            Err(GecosWriteError::HelperValidation(message)) if message == "diagnostic"
        ));
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(
                ErrorKind::AmbiguousAccount
            )))
            .update(&change()),
            Err(GecosWriteError::AmbiguousAccount)
        ));
    }

    #[test]
    fn maps_protocol_helper_outcomes_to_distinct_errors() {
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(
                ErrorKind::InvalidRequest
            )))
            .update(&change()),
            Err(GecosWriteError::InvalidRequest(message)) if message == "diagnostic"
        ));
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(
                ErrorKind::UnsupportedVersion
            )))
            .update(&change()),
            Err(GecosWriteError::UnsupportedVersion(message)) if message == "diagnostic"
        ));
    }

    #[test]
    fn maps_privilege_and_system_helper_outcomes_to_distinct_errors() {
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(
                ErrorKind::NotPrivileged
            )))
            .update(&change()),
            Err(GecosWriteError::NotPrivileged(message)) if message == "diagnostic"
        ));
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(
                ErrorKind::CommandFailed
            )))
            .update(&change()),
            Err(GecosWriteError::CommandFailed(message)) if message == "diagnostic"
        ));
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(
                ErrorKind::VerificationFailed
            )))
            .update(&change()),
            Err(GecosWriteError::VerificationFailed(message)) if message == "diagnostic"
        ));
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(error_output(ErrorKind::System)))
                .update(&change()),
            Err(GecosWriteError::System(message)) if message == "diagnostic"
        ));
    }

    #[test]
    fn rejects_malformed_oversized_and_exit_mismatched_responses() {
        let malformed = ProcessOutput {
            exit_code: Some(0),
            stdout: b"not JSON".to_vec(),
            stdout_truncated: false,
            stderr: Vec::new(),
            stderr_truncated: false,
        };
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(malformed)).update(&change()),
            Err(GecosWriteError::InvalidResponse(_))
        ));

        let oversized = ProcessOutput {
            exit_code: Some(0),
            stdout: vec![b'x'; MAX_RESPONSE_BYTES],
            stdout_truncated: true,
            stderr: Vec::new(),
            stderr_truncated: false,
        };
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(oversized)).update(&change()),
            Err(GecosWriteError::ResponseTooLarge)
        ));

        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(output(5, HelperResponse::Success)))
                .update(&change()),
            Err(GecosWriteError::ExitMismatch {
                expected: 0,
                actual: 5
            })
        ));

        let terminated = ProcessOutput {
            exit_code: None,
            stdout: Vec::new(),
            stdout_truncated: false,
            stderr: Vec::new(),
            stderr_truncated: false,
        };
        assert!(matches!(
            GecosHelperClient::new(FakeRunner::with_output(terminated)).update(&change()),
            Err(GecosWriteError::Terminated)
        ));
    }

    #[test]
    fn bounded_reader_reports_truncation_without_unbounded_allocation() {
        assert_eq!(
            read_limited(Cursor::new(b"1234"), 4).unwrap(),
            (b"1234".to_vec(), false)
        );
        assert_eq!(
            read_limited(Cursor::new(b"12345"), 4).unwrap(),
            (b"1234".to_vec(), true)
        );
    }
}
