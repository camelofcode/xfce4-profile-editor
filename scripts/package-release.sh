#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later

set -eu

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_root=$(CDPATH= cd -- "$script_directory/.." && pwd)
stage_root="$project_root/target/staged-install"
packaging_root="$project_root/target/release-packaging"
output_root="$project_root/target/release-artifacts"
package_name=xfce4-profile-editor

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

require_program() {
    command -v "$1" >/dev/null 2>&1 || fail "$1 is required to create release packages"
}

[ "$#" -eq 1 ] || fail "usage: scripts/package-release.sh VERSION"
version=$1
printf '%s\n' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || {
    fail "VERSION must contain three numeric components, for example 0.1.0"
}

require_program dpkg
require_program dpkg-deb
require_program dpkg-shlibdeps
require_program rpmbuild
require_program rpm
require_program sed
require_program sha256sum
require_program tar

staged_binary="$stage_root/usr/bin/xfce4-profile-editor"
[ -x "$staged_binary" ] || fail "run scripts/build-install-launch.sh stage first"
[ -f "$stage_root/usr/share/doc/xfce4-profile-editor/LICENSE" ] || {
    fail "the staged licence is missing; rebuild the staged installation"
}
staged_metainfo="$stage_root/usr/share/metainfo/io.github.camelofcode.Xfce4ProfileEditor.metainfo.xml"
staged_version=$(
    sed -n 's/.*<release version="\([^"]*\)".*/\1/p' "$staged_metainfo" |
        sed -n '1p'
)
[ "$version" = "$staged_version" ] || fail "VERSION does not match the staged AppStream release"

machine_architecture=$(uname -m)
[ "$machine_architecture" = x86_64 ] || fail "only x86_64 release packaging is currently supported"
debian_architecture=$(dpkg --print-architecture)
[ "$debian_architecture" = amd64 ] || fail "expected Debian architecture amd64, got $debian_architecture"
rpm_architecture=$(rpm --eval '%{_arch}')
[ "$rpm_architecture" = x86_64 ] || fail "expected RPM architecture x86_64, got $rpm_architecture"

[ "$packaging_root" = "$project_root/target/release-packaging" ] || fail "unsafe packaging path"
[ "$output_root" = "$project_root/target/release-artifacts" ] || fail "unsafe output path"
rm -rf -- "$packaging_root" "$output_root"
mkdir -p "$packaging_root/debian" "$packaging_root/rpm" "$output_root"

cp "$project_root/packaging/debian/shlibdeps-control" "$packaging_root/debian/control"
shlibdeps_output=$(cd "$packaging_root" && dpkg-shlibdeps -O "$staged_binary")
debian_dependencies=$(printf '%s\n' "$shlibdeps_output" | sed -n 's/^shlibs:Depends=//p')
[ -n "$debian_dependencies" ] || fail "dpkg-shlibdeps returned no runtime dependencies"

debian_root="$packaging_root/debian-root"
mkdir -p "$debian_root/DEBIAN"
cp -a "$stage_root/." "$debian_root/"
installed_size=$(du -sk "$debian_root/usr" | sed 's/[[:space:]].*//')
sed \
    -e '/^# SPDX-License-Identifier:/d' \
    -e "s#@VERSION@#$version#g" \
    -e "s#@ARCHITECTURE@#$debian_architecture#g" \
    -e "s#@INSTALLED_SIZE@#$installed_size#g" \
    -e "s#@SHLIB_DEPENDS@#$debian_dependencies#g" \
    "$project_root/packaging/debian/control.in" > "$debian_root/DEBIAN/control"
chmod 0644 "$debian_root/DEBIAN/control"
debian_package="$output_root/${package_name}_${version}_${debian_architecture}.deb"
dpkg-deb --root-owner-group --build "$debian_root" "$debian_package"

rpm_topdir="$packaging_root/rpm"
mkdir -p \
    "$rpm_topdir/BUILD" \
    "$rpm_topdir/BUILDROOT" \
    "$rpm_topdir/RPMS" \
    "$rpm_topdir/SOURCES" \
    "$rpm_topdir/SPECS" \
    "$rpm_topdir/SRPMS"
rpm_spec="$rpm_topdir/SPECS/${package_name}.spec"
sed \
    -e "s#@VERSION@#$version#g" \
    -e "s#@ARCHITECTURE@#$rpm_architecture#g" \
    -e "s#@STAGE_ROOT@#$stage_root#g" \
    "$project_root/packaging/rpm/xfce4-profile-editor.spec.in" > "$rpm_spec"
rpmbuild \
    --define "_topdir $rpm_topdir" \
    --define "_rpmdir $output_root" \
    -bb "$rpm_spec"
rpm_package=$(find "$output_root" -type f -name "${package_name}-${version}-1*.rpm" -print -quit)
[ -n "$rpm_package" ] || fail "rpmbuild produced no binary package"
expected_rpm="$output_root/${package_name}-${version}-1.${rpm_architecture}.rpm"
if [ "$rpm_package" != "$expected_rpm" ]; then
    mv "$rpm_package" "$expected_rpm"
fi
find "$output_root" -mindepth 1 -type d -empty -delete

source_date_epoch=${SOURCE_DATE_EPOCH:-$(git -C "$project_root" show -s --format=%ct HEAD)}
tar_package="$output_root/${package_name}-${version}-linux-${machine_architecture}.tar.xz"
tar \
    --sort=name \
    --mtime="@$source_date_epoch" \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    -C "$stage_root" \
    -cJf "$tar_package" \
    usr

(cd "$output_root" && sha256sum ./*.deb ./*.rpm ./*.tar.xz > SHA256SUMS)
(cd "$output_root" && sha256sum --check SHA256SUMS)
printf 'Release packages created under %s\n' "$output_root"
