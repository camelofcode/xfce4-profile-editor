#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later

set -eu

cargo=$1
source_root=$2
target_dir=$3
profile=$4
application_output=$5
helper_output=$6

set -- "$cargo" build \
  --manifest-path "$source_root/Cargo.toml" \
  --target-dir "$target_dir" \
  --workspace \
  --locked

case "$profile" in
  development)
    artifact_dir="$target_dir/debug"
    ;;
  release)
    set -- "$@" --release
    artifact_dir="$target_dir/release"
    ;;
  *)
    echo "unsupported Cargo profile: $profile" >&2
    exit 2
    ;;
esac

"$@"
cp "$artifact_dir/xfce4-profile-editor" "$application_output"
cp "$artifact_dir/xfce4-profile-editor-helper" "$helper_output"
