// SPDX-License-Identifier: GPL-2.0-or-later

//! Lossless parsing of the comma-separated GECOS account fields.

const FULL_NAME: usize = 0;
const OFFICE_PHONE: usize = 2;
const HOME_PHONE: usize = 3;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Gecos {
    fields: Vec<String>,
}

impl Gecos {
    pub(crate) fn parse(value: &str) -> Self {
        Self {
            fields: value.split(',').map(str::to_owned).collect(),
        }
    }

    pub(crate) fn full_name(&self) -> &str {
        self.field(FULL_NAME)
    }

    pub(crate) fn office_phone(&self) -> &str {
        self.field(OFFICE_PHONE)
    }

    pub(crate) fn home_phone(&self) -> &str {
        self.field(HOME_PHONE)
    }

    fn field(&self, index: usize) -> &str {
        self.fields.get(index).map_or("", String::as_str)
    }
}

impl std::fmt::Display for Gecos {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.fields.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_standard_empty_and_hidden_fields() {
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
}
