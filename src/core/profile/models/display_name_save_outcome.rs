// SPDX-License-Identifier: GPL-2.0-or-later

//! Result data from saving a display name.

use super::super::display_name::DisplayNameState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DisplayNameSaveOutcome {
    pub(in crate::core::profile) state: DisplayNameState,
    pub(in crate::core::profile) display_name: String,
    pub(in crate::core::profile) saved: bool,
}

impl DisplayNameSaveOutcome {
    pub(crate) fn into_parts(self) -> (DisplayNameState, String, bool) {
        (self.state, self.display_name, self.saved)
    }
}
