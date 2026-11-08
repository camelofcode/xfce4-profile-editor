# SPDX-License-Identifier: GPL-2.0-or-later

project(
  'xfce4-profile-editor',
  'rust',
  version: '0.1.0',
  license: 'GPL-2.0-or-later',
  meson_version: '>= 1.4.0',
)

i18n = import('i18n')

gettext_package = meson.project_name()
prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
libexecdir = prefix / get_option('libexecdir')
datadir = prefix / get_option('datadir')
localedir = prefix / get_option('localedir')
policydir = datadir / 'polkit-1' / 'actions'

subdir('po')
subdir('data')
subdir('src')
