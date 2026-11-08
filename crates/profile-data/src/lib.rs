// SPDX-License-Identifier: GPL-2.0-or-later

//! GTK-independent profile data parsing, validation, and transformation.

#![deny(unsafe_code)]

use thiserror::Error;

const FULL_NAME: usize = 0;
const OFFICE_PHONE: usize = 2;
const HOME_PHONE: usize = 3;

pub const MAX_EDITABLE_FIELD_BYTES: usize = 256;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Gecos {
    fields: Vec<String>,
}

impl Gecos {
    #[must_use]
    pub fn parse(value: &str) -> Self {
        Self {
            fields: value.split(',').map(str::to_owned).collect(),
        }
    }

    #[must_use]
    pub fn full_name(&self) -> &str {
        self.field(FULL_NAME)
    }

    #[must_use]
    pub fn office_phone(&self) -> &str {
        self.field(OFFICE_PHONE)
    }

    #[must_use]
    pub fn home_phone(&self) -> &str {
        self.field(HOME_PHONE)
    }

    pub fn apply(&mut self, update: &GecosUpdate) -> Result<(), ValidationError> {
        update.validate()?;
        self.set_field(FULL_NAME, &update.full_name);
        self.set_field(OFFICE_PHONE, &update.office_phone);
        self.set_field(HOME_PHONE, &update.home_phone);
        Ok(())
    }

    fn field(&self, index: usize) -> &str {
        self.fields.get(index).map_or("", String::as_str)
    }

    fn set_field(&mut self, index: usize, value: &str) {
        if self.fields.len() <= index {
            self.fields.resize(index + 1, String::new());
        }
        self.fields[index] = value.to_owned();
    }
}

impl std::fmt::Display for Gecos {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.fields.join(","))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GecosUpdate {
    pub full_name: String,
    pub office_phone: String,
    pub home_phone: String,
}

impl GecosUpdate {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_field(ProfileField::FullName, &self.full_name)?;
        validate_field(ProfileField::OfficePhone, &self.office_phone)?;
        validate_field(ProfileField::HomePhone, &self.home_phone)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileField {
    FullName,
    OfficePhone,
    HomePhone,
}

impl std::fmt::Display for ProfileField {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::FullName => "full name",
            Self::OfficePhone => "office phone",
            Self::HomePhone => "home phone",
        })
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ValidationError {
    #[error("{field} exceeds the {MAX_EDITABLE_FIELD_BYTES}-byte limit")]
    TooLong { field: ProfileField },
    #[error("{field} has leading or trailing whitespace")]
    SurroundingWhitespace { field: ProfileField },
    #[error("{field} contains the reserved character {character:?}")]
    ReservedCharacter {
        field: ProfileField,
        character: char,
    },
    #[error("{field} contains a control character")]
    ControlCharacter { field: ProfileField },
}

fn validate_field(field: ProfileField, value: &str) -> Result<(), ValidationError> {
    if value.len() > MAX_EDITABLE_FIELD_BYTES {
        return Err(ValidationError::TooLong { field });
    }
    if value.trim() != value {
        return Err(ValidationError::SurroundingWhitespace { field });
    }
    for character in value.chars() {
        if matches!(character, ',' | ':' | '\0') {
            return Err(ValidationError::ReservedCharacter { field, character });
        }
        if character.is_control() {
            return Err(ValidationError::ControlCharacter { field });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update() -> GecosUpdate {
        GecosUpdate {
            full_name: "Grace Hopper".to_owned(),
            office_phone: "+1 212 555 0100".to_owned(),
            home_phone: "+1 212 555 0199".to_owned(),
        }
    }

    #[test]
    fn round_trips_short_empty_hidden_and_extra_fields() {
        for value in [
            "Ada Lovelace,Room 1,+44 20 1,+44 20 2,notes",
            "Ada Lovelace,,,+44 20 2,",
            "Ada Lovelace",
            "",
            "Ada,room,work,home,other,extra,fields",
        ] {
            assert_eq!(Gecos::parse(value).to_string(), value);
        }
    }

    #[test]
    fn missing_fields_read_as_empty_without_padding_serialisation() {
        let gecos = Gecos::parse("Grace Hopper");

        assert_eq!(gecos.full_name(), "Grace Hopper");
        assert_eq!(gecos.office_phone(), "");
        assert_eq!(gecos.home_phone(), "");
        assert_eq!(gecos.to_string(), "Grace Hopper");
    }

    #[test]
    fn update_preserves_every_unexposed_and_extra_field() {
        let mut gecos = Gecos::parse("Ada,Room 42,work,home,other,extra,,tail");

        gecos.apply(&update()).unwrap();

        assert_eq!(
            gecos.to_string(),
            "Grace Hopper,Room 42,+1 212 555 0100,+1 212 555 0199,other,extra,,tail"
        );
    }

    #[test]
    fn update_pads_only_the_missing_intervening_fields() {
        let mut gecos = Gecos::parse("Ada");

        gecos.apply(&update()).unwrap();

        assert_eq!(
            gecos.to_string(),
            "Grace Hopper,,+1 212 555 0100,+1 212 555 0199"
        );
    }

    #[test]
    fn empty_values_clear_only_editable_fields() {
        let mut gecos = Gecos::parse("Ada,Room 42,work,home,other,");

        gecos.apply(&GecosUpdate::default()).unwrap();

        assert_eq!(gecos.to_string(), ",Room 42,,,other,");
    }

    #[test]
    fn validation_rejects_delimiters_controls_whitespace_and_long_values() {
        for (value, expected) in [
            (
                " leading",
                ValidationError::SurroundingWhitespace {
                    field: ProfileField::FullName,
                },
            ),
            (
                "trailing ",
                ValidationError::SurroundingWhitespace {
                    field: ProfileField::FullName,
                },
            ),
            (
                "comma,value",
                ValidationError::ReservedCharacter {
                    field: ProfileField::FullName,
                    character: ',',
                },
            ),
            (
                "colon:value",
                ValidationError::ReservedCharacter {
                    field: ProfileField::FullName,
                    character: ':',
                },
            ),
            (
                "line\nbreak",
                ValidationError::ControlCharacter {
                    field: ProfileField::FullName,
                },
            ),
        ] {
            let mut candidate = update();
            candidate.full_name = value.to_owned();
            assert_eq!(candidate.validate(), Err(expected));
        }

        let mut candidate = update();
        candidate.full_name = "x".repeat(MAX_EDITABLE_FIELD_BYTES + 1);
        assert_eq!(
            candidate.validate(),
            Err(ValidationError::TooLong {
                field: ProfileField::FullName,
            })
        );
    }
}
