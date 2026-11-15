// SPDX-License-Identifier: GPL-2.0-or-later

//! AccountsService-backed persistence for the current user's display name.

use thiserror::Error;

use super::account_service::{AccountsError, DisplayNameClient, SystemAccountsSource};

const MAX_DISPLAY_NAME_BYTES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DisplayNameState {
    uid: u32,
    display_name: String,
}

impl DisplayNameState {
    pub(super) fn new(uid: u32, display_name: &str) -> Self {
        Self {
            uid,
            display_name: normalize(display_name),
        }
    }

    pub(crate) fn matches(&self, display_name: &str) -> bool {
        self.display_name == normalize(display_name)
    }

    pub(crate) fn validate(&self, display_name: &str) -> Result<(), DisplayNameValidationError> {
        validate(&normalize(display_name))
    }
}

#[derive(Debug, Error)]
pub(crate) enum DisplayNameValidationError {
    #[error("display name exceeds the {MAX_DISPLAY_NAME_BYTES}-byte limit")]
    TooLong,
    #[error("display name contains a colon, comma, newline, NUL, or control character")]
    InvalidCharacter,
}

#[derive(Debug, Error)]
pub(crate) enum DisplayNameWriteError {
    #[error(transparent)]
    Validation(#[from] DisplayNameValidationError),
    #[error(transparent)]
    Accounts(#[from] AccountsError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DisplayNameSaveOutcome {
    state: DisplayNameState,
    display_name: String,
    saved: bool,
}

impl DisplayNameSaveOutcome {
    pub(crate) fn into_parts(self) -> (DisplayNameState, String, bool) {
        (self.state, self.display_name, self.saved)
    }
}

pub(crate) fn save_current_display_name(
    state: DisplayNameState,
    display_name: String,
) -> Result<DisplayNameSaveOutcome, DisplayNameWriteError> {
    save_with(&SystemAccountsSource, state, display_name)
}

fn save_with(
    client: &impl DisplayNameClient,
    state: DisplayNameState,
    display_name: String,
) -> Result<DisplayNameSaveOutcome, DisplayNameWriteError> {
    let display_name = normalize(&display_name);
    if state.display_name == display_name {
        return Ok(DisplayNameSaveOutcome {
            state,
            display_name,
            saved: false,
        });
    }

    validate(&display_name)?;
    client.set_display_name(state.uid, &display_name)?;
    Ok(DisplayNameSaveOutcome {
        state: DisplayNameState {
            uid: state.uid,
            display_name: display_name.clone(),
        },
        display_name,
        saved: true,
    })
}

fn normalize(display_name: &str) -> String {
    display_name.trim().to_owned()
}

fn validate(display_name: &str) -> Result<(), DisplayNameValidationError> {
    if display_name.len() > MAX_DISPLAY_NAME_BYTES {
        return Err(DisplayNameValidationError::TooLong);
    }
    if display_name
        .chars()
        .any(|character| character == ':' || character == ',' || character.is_control())
    {
        return Err(DisplayNameValidationError::InvalidCharacter);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    struct FakeClient {
        call: RefCell<Option<(u32, String)>>,
    }

    impl DisplayNameClient for FakeClient {
        fn set_display_name(&self, uid: u32, display_name: &str) -> Result<(), AccountsError> {
            self.call.replace(Some((uid, display_name.to_owned())));
            Ok(())
        }
    }

    #[test]
    fn unchanged_name_does_not_call_accounts_service() {
        let client = FakeClient::default();
        let outcome = save_with(
            &client,
            DisplayNameState::new(1000, "Ada Lovelace"),
            "  Ada Lovelace  ".to_owned(),
        )
        .unwrap();

        assert!(!outcome.saved);
        assert!(client.call.borrow().is_none());
    }

    #[test]
    fn changed_name_is_normalized_and_written_for_the_loaded_uid() {
        let client = FakeClient::default();
        let outcome = save_with(
            &client,
            DisplayNameState::new(1000, "Ada Lovelace"),
            "  Grace Hopper  ".to_owned(),
        )
        .unwrap();
        let (state, display_name, saved) = outcome.into_parts();

        assert!(saved);
        assert_eq!(display_name, "Grace Hopper");
        assert!(state.matches("Grace Hopper"));
        assert_eq!(
            client.call.into_inner(),
            Some((1000, "Grace Hopper".to_owned()))
        );
    }

    #[test]
    fn invalid_names_are_rejected_before_accounts_service() {
        for invalid in ["Ada,Admin", "Ada:Admin", "Ada\nAdmin", "Ada\0Admin"] {
            let client = FakeClient::default();
            let result = save_with(
                &client,
                DisplayNameState::new(1000, "Ada"),
                invalid.to_owned(),
            );

            assert!(matches!(result, Err(DisplayNameWriteError::Validation(_))));
            assert!(client.call.borrow().is_none());
        }
    }
}
