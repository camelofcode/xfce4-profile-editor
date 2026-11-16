// SPDX-License-Identifier: GPL-2.0-or-later

//! Loaded profile-avatar data.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Avatar {
    pub(crate) bytes: Vec<u8>,
    source_path: PathBuf,
}

impl Avatar {
    pub(in crate::core::profile) fn new(bytes: Vec<u8>, source_path: PathBuf) -> Self {
        Self { bytes, source_path }
    }

    pub(crate) fn source_path(&self) -> &Path {
        &self.source_path
    }
}
