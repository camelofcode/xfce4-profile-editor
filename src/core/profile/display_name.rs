// SPDX-License-Identifier: GPL-2.0-or-later

//! AccountsService-backed display-name state and persistence.

use super::accounts::{AccountsError, set_display_name};

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
) -> Result<DisplayNameSaveOutcome, AccountsError> {
    save_with(set_display_name, state, display_name)
}

fn save_with(
    write: impl FnOnce(u32, &str) -> Result<(), AccountsError>,
    state: DisplayNameState,
    display_name: String,
) -> Result<DisplayNameSaveOutcome, AccountsError> {
    let display_name = normalize(&display_name);
    if state.display_name == display_name {
        return Ok(DisplayNameSaveOutcome {
            state,
            display_name,
            saved: false,
        });
    }

    write(state.uid, &display_name)?;
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[test]
    fn unchanged_name_does_not_call_accounts_service() {
        let call = RefCell::new(None);
        let outcome = save_with(
            |uid, display_name| {
                call.replace(Some((uid, display_name.to_owned())));
                Ok(())
            },
            DisplayNameState::new(1000, "Ada Lovelace"),
            "  Ada Lovelace  ".to_owned(),
        )
        .unwrap();

        assert!(!outcome.saved);
        assert!(call.borrow().is_none());
    }

    #[test]
    fn changed_name_is_normalized_and_written_for_the_loaded_uid() {
        let call = RefCell::new(None);
        let outcome = save_with(
            |uid, display_name| {
                call.replace(Some((uid, display_name.to_owned())));
                Ok(())
            },
            DisplayNameState::new(1000, "Ada Lovelace"),
            "  Grace Hopper  ".to_owned(),
        )
        .unwrap();
        let (state, display_name, saved) = outcome.into_parts();

        assert!(saved);
        assert_eq!(display_name, "Grace Hopper");
        assert!(state.matches("Grace Hopper"));
        assert_eq!(call.into_inner(), Some((1000, "Grace Hopper".to_owned())));
    }

    #[test]
    fn display_name_rules_are_delegated_to_accounts_service() {
        let call = RefCell::new(None);

        save_with(
            |uid, display_name| {
                call.replace(Some((uid, display_name.to_owned())));
                Ok(())
            },
            DisplayNameState::new(1000, "Ada"),
            "Lovelace, Ada: Countess".to_owned(),
        )
        .unwrap();

        assert_eq!(
            call.into_inner(),
            Some((1000, "Lovelace, Ada: Countess".to_owned()))
        );
    }
}
