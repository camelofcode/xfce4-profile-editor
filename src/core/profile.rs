// SPDX-License-Identifier: GPL-2.0-or-later

use std::path::PathBuf;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Profile {
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) home_phone: String,
    pub(crate) email: String,
    pub(crate) office_phone: String,
    /// Ordered local image candidates. The UI uses the first candidate it can decode.
    pub(crate) avatars: Vec<Avatar>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Avatar {
    pub(crate) bytes: Vec<u8>,
    source: AvatarSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AvatarSource {
    Face(PathBuf),
    AccountsService(PathBuf),
}

impl Avatar {
    pub(super) fn new(bytes: Vec<u8>, source: AvatarSource) -> Self {
        Self { bytes, source }
    }

    pub(crate) fn source_path(&self) -> &std::path::Path {
        match &self.source {
            AvatarSource::Face(path) | AvatarSource::AccountsService(path) => path,
        }
    }
}

pub(crate) fn split_full_name(full_name: &str) -> (String, String) {
    let full_name = full_name.trim();
    match full_name.find(char::is_whitespace) {
        Some(boundary) => (
            full_name[..boundary].to_owned(),
            full_name[boundary..].trim_start().to_owned(),
        ),
        None => (full_name.to_owned(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_first_name_from_the_remaining_name() {
        assert_eq!(
            split_full_name("  Ada Augusta Lovelace  "),
            ("Ada".to_owned(), "Augusta Lovelace".to_owned())
        );
        assert_eq!(
            split_full_name("Prince"),
            ("Prince".to_owned(), String::new())
        );
        assert_eq!(split_full_name("  "), (String::new(), String::new()));
    }
}
