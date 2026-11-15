// SPDX-License-Identifier: GPL-2.0-or-later

use std::path::PathBuf;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Profile {
    pub(crate) display_name: String,
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
