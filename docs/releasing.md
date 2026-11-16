<!-- SPDX-License-Identifier: GPL-2.0-or-later -->

# Release process

GitHub Actions builds release packages on one `ubuntu-24.04` runner. Each build
uses the verified Meson staging layout to create these x86_64 artifacts:

- a Debian/Ubuntu `.deb` package with shared-library dependencies calculated by
  `dpkg-shlibdeps`;
- an `.rpm` package with RPM's automatic ELF dependency generation;
- a `linux-x86_64.tar.xz` archive containing the staged `/usr` tree;
- `SHA256SUMS` covering all three packages.

The binary packages are convenience artifacts for manual testing and direct
installation. Distribution maintainers may use the repository's Meson install
and packaging metadata to build natively for other architectures or policies.

## Test a release build

Run the **Release** workflow manually from GitHub Actions. A manual run builds
and validates the packages, then retains them as one workflow artifact for
seven days. It does not create a GitHub release and has read-only repository
permissions.

On an Ubuntu 24.04 development system with `dpkg-dev`, `rpm`, and `xz-utils`
installed, the same packaging step can be repeated locally after staging:

```sh
scripts/build-install-launch.sh stage
scripts/package-release.sh 0.1.0
```

The generated files are written below `target/release-artifacts/`. The supplied
version must match the staged AppStream release entry.

## Prepare a release

1. Update the matching version in `Cargo.toml`, `meson.build`, and the newest
   AppStream release entry. Regenerate `Cargo.lock` if the Cargo version changed.
2. Merge the version change through the normal pull-request validation flow.
3. From the up-to-date `main` branch, create and push an annotated tag:

   ```sh
   git tag -a v0.1.0 -m "Xfce Profile Editor 0.1.0"
   git push origin v0.1.0
   ```

The tag must use `vMAJOR.MINOR.PATCH`, match all recorded project versions, and
point to a commit contained in `main`. A valid tag runs the same package build,
then creates a **draft** GitHub release with generated notes and the verified
artifacts. The workflow never publishes a release.

## Approve a draft

Before publishing manually:

1. Verify `SHA256SUMS` after downloading the assets.
2. Install and launch the `.deb` on a supported Debian/Ubuntu system.
3. Install and launch the `.rpm` on a supported Fedora system.
4. Confirm the Xfce Settings entry, display-name update, image selection and
   removal, and camera flow where hardware is available.
5. Review and edit the generated release notes.

Publish the draft only after those checks pass. A failed or incomplete draft
should remain unpublished while the underlying issue is fixed through another
pull request.
