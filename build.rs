// SPDX-License-Identifier: GPL-2.0-or-later

use std::env;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const SCSS_ENTRY_POINT: &str = "ui/sass/app.scss";
const SCSS_PARTIALS: &[&str] = &["ui/sass/_profile-window.scss"];
const RESOURCE_MANIFEST: &str = "resources.gresource.xml";
const RESOURCE_OUTPUT: &str = "xfce4_profile_editor.gresource";

fn main() {
    println!("cargo:rerun-if-env-changed=GETTEXT_PACKAGE");
    println!("cargo:rerun-if-env-changed=LOCALEDIR");
    println!("cargo:rerun-if-changed={SCSS_ENTRY_POINT}");
    for partial in SCSS_PARTIALS {
        println!("cargo:rerun-if-changed={partial}");
    }

    let output_directory = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"));
    let compiled_css = output_directory.join("app.css");

    compile_scss(Path::new(SCSS_ENTRY_POINT), &compiled_css);

    let resource_sources = [PathBuf::from("."), output_directory];
    glib_build_tools::compile_resources(&resource_sources, RESOURCE_MANIFEST, RESOURCE_OUTPUT);
}

fn compile_scss(input: &Path, output: &Path) {
    match Command::new("sass")
        .args(["--no-source-map", "--style=expanded"])
        .arg(input)
        .arg(output)
        .status()
    {
        Ok(status) => require_success("sass", status),
        Err(error) if error.kind() == ErrorKind::NotFound => compile_with_sassc(input, output),
        Err(error) => panic!("failed to run sass: {error}"),
    }
}

fn compile_with_sassc(input: &Path, output: &Path) {
    let status = Command::new("sassc")
        .args(["--style", "expanded"])
        .arg(input)
        .arg(output)
        .status()
        .unwrap_or_else(|error| {
            panic!("neither sass nor sassc could be run to compile {SCSS_ENTRY_POINT}: {error}")
        });

    require_success("sassc", status);
}

fn require_success(program: &str, status: ExitStatus) {
    assert!(status.success(), "{program} exited with status {status}");
}
