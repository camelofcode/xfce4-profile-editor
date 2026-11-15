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
localedir = prefix / get_option('localedir')

subdir('po')
subdir('src')
