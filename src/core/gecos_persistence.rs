// SPDX-License-Identifier: GPL-2.0-or-later

//! Lossless application-side coordination of privileged GECOS writes.

use xfce4_profile_data::{Gecos, GecosUpdate};

use super::gecos_helper::{GecosChange, GecosClient, GecosHelperClient, GecosWriteError};
use super::profile::Profile;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct GecosFields {
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) office_phone: String,
    pub(crate) home_phone: String,
}

impl GecosFields {
    #[must_use]
    pub(crate) fn normalized(&self) -> Self {
        Self {
            first_name: self.first_name.trim().to_owned(),
            last_name: self.last_name.trim().to_owned(),
            office_phone: self.office_phone.trim().to_owned(),
            home_phone: self.home_phone.trim().to_owned(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), xfce4_profile_data::ValidationError> {
        self.normalized().update().validate()
    }

    fn full_name(&self) -> String {
        match (self.first_name.is_empty(), self.last_name.is_empty()) {
            (false, false) => format!("{} {}", self.first_name, self.last_name),
            (false, true) => self.first_name.clone(),
            (true, false) => self.last_name.clone(),
            (true, true) => String::new(),
        }
    }

    fn update(&self) -> GecosUpdate {
        GecosUpdate {
            full_name: self.full_name(),
            office_phone: self.office_phone.clone(),
            home_phone: self.home_phone.clone(),
        }
    }
}

impl From<&Profile> for GecosFields {
    fn from(profile: &Profile) -> Self {
        Self {
            first_name: profile.first_name.clone(),
            last_name: profile.last_name.clone(),
            office_phone: profile.office_phone.clone(),
            home_phone: profile.home_phone.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct GecosState {
    snapshot: Gecos,
    fields: GecosFields,
}

impl GecosState {
    pub(super) fn new(snapshot: Gecos, profile: &Profile) -> Self {
        Self {
            snapshot,
            fields: GecosFields::from(profile).normalized(),
        }
    }

    #[must_use]
    pub(crate) fn matches(&self, fields: &GecosFields) -> bool {
        self.fields == fields.normalized()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GecosSaveOutcome {
    Unchanged {
        state: GecosState,
        fields: GecosFields,
    },
    Saved {
        state: GecosState,
        fields: GecosFields,
    },
}

impl GecosSaveOutcome {
    #[must_use]
    pub(crate) fn into_parts(self) -> (GecosState, GecosFields, bool) {
        match self {
            Self::Unchanged { state, fields } => (state, fields, false),
            Self::Saved { state, fields } => (state, fields, true),
        }
    }
}

pub(crate) fn save_current_gecos(
    state: GecosState,
    fields: GecosFields,
) -> Result<GecosSaveOutcome, GecosWriteError> {
    save_with(&GecosHelperClient::default(), state, fields)
}

fn save_with(
    client: &impl GecosClient,
    state: GecosState,
    fields: GecosFields,
) -> Result<GecosSaveOutcome, GecosWriteError> {
    let fields = fields.normalized();
    if state.fields == fields {
        return Ok(GecosSaveOutcome::Unchanged { state, fields });
    }

    let update = fields.update();
    update.validate()?;
    let change = GecosChange {
        expected_gecos: state.snapshot.to_string(),
        full_name: update.full_name.clone(),
        office_phone: update.office_phone.clone(),
        home_phone: update.home_phone.clone(),
    };
    client.update(&change)?;

    let mut snapshot = state.snapshot;
    snapshot.apply(&update)?;
    Ok(GecosSaveOutcome::Saved {
        state: GecosState {
            snapshot,
            fields: fields.clone(),
        },
        fields,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Default)]
    struct FakeClient {
        calls: Cell<usize>,
        change: RefCell<Option<GecosChange>>,
    }

    impl GecosClient for FakeClient {
        fn update(&self, change: &GecosChange) -> Result<(), GecosWriteError> {
            self.calls.set(self.calls.get() + 1);
            self.change.replace(Some(change.clone()));
            Ok(())
        }
    }

    fn profile() -> Profile {
        Profile {
            first_name: "Ada".to_owned(),
            last_name: "Lovelace".to_owned(),
            home_phone: " home ".to_owned(),
            email: String::new(),
            office_phone: " work ".to_owned(),
            avatars: Vec::new(),
        }
    }

    fn state() -> GecosState {
        GecosState::new(
            Gecos::parse("Ada  Lovelace,Room 42, work , home ,hidden,extra,,tail"),
            &profile(),
        )
    }

    #[test]
    fn an_unchanged_form_does_not_normalize_or_start_the_helper() {
        let client = FakeClient::default();

        let outcome = save_with(&client, state(), GecosFields::from(&profile())).unwrap();

        assert!(matches!(outcome, GecosSaveOutcome::Unchanged { .. }));
        assert_eq!(client.calls.get(), 0);
    }

    #[test]
    fn a_write_uses_the_exact_snapshot_and_preserves_unexposed_fields() {
        let client = FakeClient::default();
        let fields = GecosFields {
            first_name: "  Grace ".to_owned(),
            last_name: " Hopper  ".to_owned(),
            office_phone: " +1 212 555 0100 ".to_owned(),
            home_phone: String::new(),
        };

        let outcome = save_with(&client, state(), fields).unwrap();
        let change = client.change.borrow();
        let change = change.as_ref().unwrap();
        assert_eq!(
            change.expected_gecos,
            "Ada  Lovelace,Room 42, work , home ,hidden,extra,,tail"
        );
        assert_eq!(change.full_name, "Grace Hopper");
        assert_eq!(change.office_phone, "+1 212 555 0100");
        assert!(change.home_phone.is_empty());

        let (state, fields, saved) = outcome.into_parts();
        assert!(saved);
        assert_eq!(fields.first_name, "Grace");
        assert!(state.matches(&fields));
        assert_eq!(
            state.snapshot.to_string(),
            "Grace Hopper,Room 42,+1 212 555 0100,,hidden,extra,,tail"
        );
    }

    #[test]
    fn invalid_fields_are_rejected_before_starting_the_helper() {
        let client = FakeClient::default();
        let fields = GecosFields {
            first_name: "hostile,name".to_owned(),
            ..GecosFields::from(&profile())
        };

        assert!(matches!(
            save_with(&client, state(), fields),
            Err(GecosWriteError::Validation(_))
        ));
        assert_eq!(client.calls.get(), 0);
    }
}
