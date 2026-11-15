#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later

set -eu

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
development_build="$project_root/target/meson-build"
release_build="$project_root/target/meson-release"
stage_root="$project_root/target/staged-install"
mode_detector="$project_root/.agents/skills/resource-aware-builds/scripts/detect-build-mode.sh"
staged_application="$stage_root/usr/bin/xfce4-profile-editor"
installed_application=/usr/bin/xfce4-profile-editor

usage() {
    cat <<'EOF'
Usage: scripts/build-install-launch.sh COMMAND

Commands:
  build    Configure and compile a development build without root access.
  stage    Build release artifacts and stage the installation under target/.
  install  Stage, verify, and explicitly install under /usr using sudo.
  launch   Launch the application already installed under /usr.
  cycle    Install, verify, and launch the system installation.
  help     Show this help.

Only install and cycle modify /usr or request elevation.

This is the authoritative application build workflow. Every command reuses
the workspace Cargo cache under target/; Rust checks use that same cache.
EOF
}

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

read_build_mode() {
    [ -f "$mode_detector" ] || fail "resource-mode detector is missing"
    mode_report=$(sh "$mode_detector")
    build_mode=$(printf '%s\n' "$mode_report" | sed -n 's/^mode=//p')
    build_jobs=$(printf '%s\n' "$mode_report" | sed -n 's/^jobs=//p')
    [ -n "$build_mode" ] || fail "resource-mode detector returned no mode"
    [ -n "$build_jobs" ] || fail "resource-mode detector returned no job count"
    printf '%s\n' "$mode_report"
}

run_compile() {
    case "$build_mode" in
        responsive)
            CARGO_BUILD_JOBS=1 nice -n 10 "$@"
            ;;
        balanced)
            CARGO_BUILD_JOBS="$build_jobs" nice -n 5 "$@"
            ;;
        unrestricted)
            "$@"
            ;;
        *)
            fail "unsupported resource mode: $build_mode"
            ;;
    esac
}

configure() {
    build_directory=$1
    profile=$2

    if [ -f "$build_directory/meson-private/coredata.dat" ]; then
        meson setup --reconfigure "$build_directory" --prefix=/usr -Dprofile="$profile"
    else
        meson setup "$build_directory" --prefix=/usr -Dprofile="$profile"
    fi
}

build_development() {
    read_build_mode
    configure "$development_build" development
    run_compile meson compile -C "$development_build"
}

build_release() {
    read_build_mode
    configure "$release_build" release
    run_compile meson compile -C "$release_build"
}

require_mode() {
    path=$1
    expected_mode=$2
    actual_mode=$(stat -c '%a' "$path")
    [ "$actual_mode" = "$expected_mode" ] || {
        fail "$path has mode $actual_mode; expected $expected_mode"
    }
}

require_root_owner() {
    path=$1
    [ "$(stat -c '%u' "$path")" = 0 ] || fail "$path is not owned by root"
}

verify_layout() {
    root=$1
    application="$root/usr/bin/xfce4-profile-editor"

    [ -x "$application" ] || fail "application is missing from $root"
    require_mode "$application" 755
}

stage_release() {
    build_release
    [ "$stage_root" = "$project_root/target/staged-install" ] || {
        fail "refusing to clear unexpected staging path: $stage_root"
    }
    rm -rf -- "$stage_root"
    DESTDIR="$stage_root" meson install --no-rebuild -C "$release_build"
    verify_layout "$stage_root"
    printf 'Staged installation verified under %s\n' "$stage_root"
}

verify_system_install() {
    [ -x "$installed_application" ] || fail "installed application is missing"
    require_mode "$installed_application" 755
    require_root_owner "$installed_application"
}

install_release() {
    stage_release
    command -v sudo >/dev/null 2>&1 || fail "sudo is required for a system installation"
    printf 'Installing the verified release under /usr; sudo may request authentication.\n'
    sudo install -Dm755 "$staged_application" "$installed_application"
    verify_system_install
    printf 'System installation verified.\n'
}

launch_installed() {
    [ -x "$installed_application" ] || {
        fail "$installed_application is not installed; run the install command first"
    }
    exec "$installed_application"
}

[ "$#" -le 1 ] || fail "expected one command; use help for usage"
command_name=${1:-help}
cd "$project_root"

case "$command_name" in
    build)
        build_development
        ;;
    stage)
        stage_release
        ;;
    install)
        install_release
        ;;
    launch)
        launch_installed
        ;;
    cycle)
        install_release
        launch_installed
        ;;
    help|-h|--help)
        usage
        ;;
    *)
        fail "unknown command '$command_name'; use help for usage"
        ;;
esac
