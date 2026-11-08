# SPDX-License-Identifier: GPL-2.0-or-later

policy_configuration = configuration_data()
policy_configuration.set('libexecdir', libexecdir)

configure_file(
  input: 'polkit/io.github.camelofcode.Xfce4ProfileEditor.policy.in',
  output: 'io.github.camelofcode.Xfce4ProfileEditor.policy',
  configuration: policy_configuration,
  install: true,
  install_dir: policydir,
)
