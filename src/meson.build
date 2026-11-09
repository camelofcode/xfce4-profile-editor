# SPDX-License-Identifier: GPL-2.0-or-later

cargo = find_program('cargo')
sh = find_program('sh')
cargo_build_script = files('../build-aux/cargo-build.sh')
cargo_target_dir = meson.project_source_root() / 'target'
camera_enabled = get_option('camera') ? 'true' : 'false'

cargo_environment = environment()
cargo_environment.set('GETTEXT_PACKAGE', gettext_package)
cargo_environment.set('LOCALEDIR', localedir)

custom_target(
  'cargo-build',
  output: [meson.project_name(), meson.project_name() + '-helper'],
  command: [
    sh,
    cargo_build_script,
    cargo,
    meson.project_source_root(),
    cargo_target_dir,
    get_option('profile'),
    camera_enabled,
    '@OUTPUT0@',
    '@OUTPUT1@',
  ],
  env: cargo_environment,
  console: true,
  build_always_stale: true,
  build_by_default: true,
  install: true,
  install_dir: [bindir, libexecdir],
  install_mode: 'rwxr-xr-x',
)
