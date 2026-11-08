// SPDX-License-Identifier: GPL-2.0-or-later

//! Application-side validation and AccountsService email persistence.

use thiserror::Error;

use super::account_service::{AccountsError, EmailClient, SystemAccountsSource};

const MAX_EMAIL_BYTES: usize = 320;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EmailState {
    uid: u32,
    email: String,
}

impl EmailState {
    pub(super) fn new(uid: u32, email: &str) -> Self {
        Self {
            uid,
            email: normalize_email(email),
        }
    }

    #[must_use]
    pub(crate) fn matches(&self, email: &str) -> bool {
        self.email == normalize_email(email)
    }

    pub(crate) fn validate(&self, email: &str) -> Result<(), EmailValidationError> {
        validate_email(&normalize_email(email))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EmailSaveOutcome {
    Unchanged { state: EmailState, email: String },
    Saved { state: EmailState, email: String },
}

impl EmailSaveOutcome {
    #[must_use]
    pub(crate) fn into_parts(self) -> (EmailState, String, bool) {
        match self {
            Self::Unchanged { state, email } => (state, email, false),
            Self::Saved { state, email } => (state, email, true),
        }
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub(crate) enum EmailValidationError {
    #[error("email address exceeds the {MAX_EMAIL_BYTES}-byte limit")]
    TooLong,
    #[error("email address contains whitespace or a control character")]
    InvalidCharacter,
    #[error("email address must contain one @ with text on both sides")]
    InvalidStructure,
}

#[derive(Debug, Error)]
pub(crate) enum EmailWriteError {
    #[error(transparent)]
    Validation(#[from] EmailValidationError),
    #[error(transparent)]
    Accounts(#[from] AccountsError),
}

pub(crate) fn save_current_email(
    state: EmailState,
    email: String,
) -> Result<EmailSaveOutcome, EmailWriteError> {
    save_with(&SystemAccountsSource, state, email)
}

fn save_with(
    client: &impl EmailClient,
    state: EmailState,
    email: String,
) -> Result<EmailSaveOutcome, EmailWriteError> {
    let email = normalize_email(&email);
    if state.email == email {
        return Ok(EmailSaveOutcome::Unchanged { state, email });
    }

    validate_email(&email)?;
    client.set_email(state.uid, &email)?;
    Ok(EmailSaveOutcome::Saved {
        state: EmailState {
            uid: state.uid,
            email: email.clone(),
        },
        email,
    })
}

fn normalize_email(email: &str) -> String {
    email.trim().to_owned()
}

fn validate_email(email: &str) -> Result<(), EmailValidationError> {
    if email.len() > MAX_EMAIL_BYTES {
        return Err(EmailValidationError::TooLong);
    }
    if email.is_empty() {
        return Ok(());
    }
    if email
        .chars()
        .any(|character| character.is_whitespace() || character.is_control() || character == '\0')
    {
        return Err(EmailValidationError::InvalidCharacter);
    }

    let mut parts = email.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    if local.is_empty() || domain.is_empty() || parts.next().is_some() {
        return Err(EmailValidationError::InvalidStructure);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Default)]
    struct FakeEmailClient {
        calls: Cell<usize>,
        value: RefCell<Option<(u32, String)>>,
    }

    struct FailingEmailClient;

    impl EmailClient for FailingEmailClient {
        fn set_email(&self, _uid: u32, _email: &str) -> Result<(), AccountsError> {
            Err(AccountsError::test_unavailable())
        }
    }

    impl EmailClient for FakeEmailClient {
        fn set_email(&self, uid: u32, email: &str) -> Result<(), AccountsError> {
            self.calls.set(self.calls.get() + 1);
            self.value.replace(Some((uid, email.to_owned())));
            Ok(())
        }
    }

    #[test]
    fn unchanged_email_does_not_call_accounts_service() {
        let client = FakeEmailClient::default();
        let state = EmailState::new(1000, "legacy-value");

        let outcome = save_with(&client, state, " legacy-value ".to_owned()).unwrap();

        assert!(matches!(outcome, EmailSaveOutcome::Unchanged { .. }));
        assert_eq!(client.calls.get(), 0);
    }

    #[test]
    fn changed_and_empty_email_values_are_written_for_the_loaded_uid() {
        for candidate in [" grace@example.test ", ""] {
            let client = FakeEmailClient::default();
            let state = EmailState::new(1000, "ada@example.test");

            let outcome = save_with(&client, state, candidate.to_owned()).unwrap();
            let expected = candidate.trim();

            assert_eq!(
                client.value.borrow().as_ref(),
                Some(&(1000, expected.to_owned()))
            );
            let (state, email, saved) = outcome.into_parts();
            assert!(saved);
            assert_eq!(email, expected);
            assert!(state.matches(expected));
        }
    }

    #[test]
    fn invalid_email_is_rejected_before_calling_accounts_service() {
        for candidate in ["missing-at", "@example.test", "a@@example.test", "a @b"] {
            let client = FakeEmailClient::default();
            let state = EmailState::new(1000, "ada@example.test");

            assert!(matches!(
                save_with(&client, state, candidate.to_owned()),
                Err(EmailWriteError::Validation(_))
            ));
            assert_eq!(client.calls.get(), 0);
        }
    }

    #[test]
    fn accounts_service_failure_is_a_typed_write_error() {
        let state = EmailState::new(1000, "ada@example.test");

        assert!(matches!(
            save_with(&FailingEmailClient, state, "grace@example.test".to_owned()),
            Err(EmailWriteError::Accounts(_))
        ));
    }
}
