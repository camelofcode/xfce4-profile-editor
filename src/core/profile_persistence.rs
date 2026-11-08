// SPDX-License-Identifier: GPL-2.0-or-later

//! Coordination of the independent stores updated by the profile form.

use thiserror::Error;

use super::email_persistence::{EmailSaveOutcome, EmailState, EmailWriteError, save_current_email};
use super::gecos_helper::GecosWriteError;
use super::gecos_persistence::{GecosFields, GecosSaveOutcome, GecosState, save_current_gecos};

#[derive(Clone, Debug)]
pub(crate) struct ProfileSaveRequest {
    pub(crate) gecos_state: GecosState,
    pub(crate) gecos_fields: GecosFields,
    pub(crate) email_state: Option<EmailState>,
    pub(crate) email: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProfileSaveOutcome {
    gecos: GecosSaveOutcome,
    email: Option<EmailSaveOutcome>,
    unchanged_email: String,
}

impl ProfileSaveOutcome {
    #[must_use]
    pub(crate) fn into_parts(self) -> (GecosState, GecosFields, Option<EmailState>, String, bool) {
        let (gecos_state, gecos_fields, gecos_saved) = self.gecos.into_parts();
        let (email_state, email, email_saved) = match self.email {
            Some(outcome) => {
                let (state, email, saved) = outcome.into_parts();
                (Some(state), email, saved)
            }
            None => (None, self.unchanged_email, false),
        };
        (
            gecos_state,
            gecos_fields,
            email_state,
            email,
            gecos_saved || email_saved,
        )
    }
}

#[derive(Debug, Error)]
pub(crate) enum ProfileSaveError {
    #[error(transparent)]
    Gecos(#[from] GecosWriteError),
    #[error("the email address could not be saved: {source}")]
    Email {
        #[source]
        source: EmailWriteError,
        completed_gecos: Option<Box<GecosSaveOutcome>>,
    },
}

impl ProfileSaveError {
    #[must_use]
    pub(crate) fn completed_gecos(&self) -> Option<&GecosSaveOutcome> {
        match self {
            Self::Gecos(_) => None,
            Self::Email {
                completed_gecos, ..
            } => completed_gecos.as_deref(),
        }
    }
}

pub(crate) fn save_current_profile(
    request: ProfileSaveRequest,
) -> Result<ProfileSaveOutcome, ProfileSaveError> {
    save_with(request, save_current_gecos, save_current_email)
}

fn save_with(
    request: ProfileSaveRequest,
    save_gecos: impl FnOnce(GecosState, GecosFields) -> Result<GecosSaveOutcome, GecosWriteError>,
    save_email: impl FnOnce(EmailState, String) -> Result<EmailSaveOutcome, EmailWriteError>,
) -> Result<ProfileSaveOutcome, ProfileSaveError> {
    if !request.gecos_state.matches(&request.gecos_fields) {
        request
            .gecos_fields
            .validate()
            .map_err(GecosWriteError::from)?;
    }
    if let Some(state) = &request.email_state
        && !state.matches(&request.email)
        && let Err(source) = state.validate(&request.email)
    {
        return Err(ProfileSaveError::Email {
            source: source.into(),
            completed_gecos: None,
        });
    }

    let gecos = save_gecos(request.gecos_state, request.gecos_fields)?;
    let email = match request.email_state {
        Some(state) => match save_email(state, request.email.clone()) {
            Ok(outcome) => Some(outcome),
            Err(source) => {
                return Err(ProfileSaveError::Email {
                    source,
                    completed_gecos: Some(Box::new(gecos)),
                });
            }
        },
        None => None,
    };

    Ok(ProfileSaveOutcome {
        gecos,
        email,
        unchanged_email: request.email,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use xfce4_profile_data::Gecos;

    use super::*;
    use crate::core::account_service::AccountsError;
    use crate::core::profile::Profile;

    fn request() -> ProfileSaveRequest {
        let profile = Profile {
            first_name: "Ada".to_owned(),
            last_name: "Lovelace".to_owned(),
            home_phone: "home".to_owned(),
            email: "ada@example.test".to_owned(),
            office_phone: "work".to_owned(),
            avatars: Vec::new(),
        };
        ProfileSaveRequest {
            gecos_state: GecosState::new(
                Gecos::parse("Ada Lovelace,room,work,home,hidden"),
                &profile,
            ),
            gecos_fields: GecosFields {
                first_name: "Grace".to_owned(),
                last_name: "Hopper".to_owned(),
                office_phone: "new-work".to_owned(),
                home_phone: "new-home".to_owned(),
            },
            email_state: Some(EmailState::new(1000, &profile.email)),
            email: "grace@example.test".to_owned(),
        }
    }

    fn saved_gecos(state: GecosState, fields: GecosFields) -> GecosSaveOutcome {
        GecosSaveOutcome::Saved { state, fields }
    }

    fn saved_email(_state: EmailState, email: String) -> EmailSaveOutcome {
        EmailSaveOutcome::Saved {
            state: EmailState::new(1000, &email),
            email,
        }
    }

    #[test]
    fn invalid_email_is_rejected_before_either_store_is_called() {
        let gecos_calls = Cell::new(0);
        let email_calls = Cell::new(0);
        let mut request = request();
        request.email = "invalid".to_owned();

        let result = save_with(
            request,
            |state, fields| {
                gecos_calls.set(gecos_calls.get() + 1);
                Ok(saved_gecos(state, fields))
            },
            |state, email| {
                email_calls.set(email_calls.get() + 1);
                Ok(saved_email(state, email))
            },
        );

        assert!(matches!(result, Err(ProfileSaveError::Email { .. })));
        assert_eq!(gecos_calls.get(), 0);
        assert_eq!(email_calls.get(), 0);
    }

    #[test]
    fn gecos_failure_prevents_the_email_write() {
        let email_calls = Cell::new(0);

        let result = save_with(
            request(),
            |_state, _fields| Err(GecosWriteError::AuthenticationCancelled),
            |state, email| {
                email_calls.set(email_calls.get() + 1);
                Ok(saved_email(state, email))
            },
        );

        assert!(matches!(result, Err(ProfileSaveError::Gecos(_))));
        assert_eq!(email_calls.get(), 0);
    }

    #[test]
    fn email_failure_returns_the_completed_gecos_update() {
        let result = save_with(
            request(),
            |state, fields| Ok(saved_gecos(state, fields)),
            |_state, _email| Err(EmailWriteError::Accounts(AccountsError::test_unavailable())),
        );

        let error = result.unwrap_err();
        assert!(matches!(error, ProfileSaveError::Email { .. }));
        assert!(error.completed_gecos().is_some());
    }

    #[test]
    fn both_stores_complete_as_one_profile_transaction() {
        let outcome = save_with(
            request(),
            |state, fields| Ok(saved_gecos(state, fields)),
            |state, email| Ok(saved_email(state, email)),
        )
        .unwrap();

        let (_gecos_state, fields, email_state, email, saved) = outcome.into_parts();
        assert!(saved);
        assert_eq!(fields.first_name, "Grace");
        assert_eq!(email, "grace@example.test");
        assert!(email_state.is_some_and(|state| state.matches(&email)));
    }
}
