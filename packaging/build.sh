#!/usr/bin/env bash
# Build ross-fisheye and turn it into deb, rpm and pacman packages.
#   packaging/build.sh <version> <out-dir>     (needs cargo, the build dependencies and nfpm)
set -euo pipefail
version=${1:?version}
out=$(realpath -m "${2:?out-dir}")
cd "$(dirname "$0")/.."
mkdir -p "$out"

cargo build --release --locked
built=$(./target/release/ross-fisheye --version)
[ "$built" = "ross-fisheye $version" ] || { echo "version mismatch: binary says '$built', building $version" >&2; exit 1; }

export VERSION="$version"
for packager in deb rpm archlinux; do
  PKG_NAME=ross-fisheye
  [ "$packager" = archlinux ] && PKG_NAME=ross-fisheye-bin   # Arch names prebuilt packages -bin
  PKG_NAME=$PKG_NAME nfpm package --config packaging/nfpm.yaml --packager "$packager" --target "$out/"
done
ls -la "$out"
