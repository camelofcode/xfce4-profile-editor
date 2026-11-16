# SPDX-License-Identifier: GPL-2.0-or-later

project(
  'xfce4-profile-editor',
  'rust',
  version: '0.1.0',
  license: 'GPL-2.0-or-later',
  meson_version: '>= 1.4.0',
)

gnome = import('gnome')
i18n = import('i18n')

application_id = 'io.github.camelofcode.Xfce4ProfileEditor'
gettext_package = meson.project_name()
prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')
localedir = prefix / get_option('localedir')

subdir('po')
subdir('data')
subdir('src')

if host_machine.system() == 'linux'
  gnome.post_install(
    gtk_update_icon_cache: true,
    update_desktop_database: true,
  )
endif
