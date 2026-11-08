// SPDX-License-Identifier: GPL-2.0-or-later

#![deny(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    xfce4_profile_editor_helper::run_process()
}
