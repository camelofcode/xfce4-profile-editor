# SPDX-License-Identifier: GPL-2.0-or-later

if host_machine.system() == 'linux'
  desktop_configuration = configuration_data()
  desktop_configuration.set('APP_ID', application_id)

  desktop_template = configure_file(
    input: application_id + '.desktop.in.in',
    output: application_id + '.desktop.in',
    configuration: desktop_configuration,
  )

  i18n.merge_file(
    input: desktop_template,
    output: application_id + '.desktop',
    po_dir: '../po',
    type: 'desktop',
    install: true,
    install_dir: datadir / 'applications',
  )

  i18n.merge_file(
    input: application_id + '.metainfo.xml',
    output: application_id + '.metainfo.xml',
    po_dir: '../po',
    type: 'xml',
    install: true,
    install_dir: datadir / 'metainfo',
  )

  subdir('icons')
endif
