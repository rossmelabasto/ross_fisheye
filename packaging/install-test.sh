#!/usr/bin/env bash
# Install a built package on a clean system and check that it runs. There's no compositor in
# the container, so this checks the binary and every library it needs, not the lens itself.
#   install-test.sh <deb|rpm|arch> <package-file> <version>    (as root, inside the container)
set -euo pipefail
kind=$1 pkg=$(realpath "$2") version=$3

case "$kind" in
  deb) export DEBIAN_FRONTEND=noninteractive; apt-get update -qq; apt-get install -y -qq "$pkg" >/dev/null ;;
  rpm) dnf install -y -q "$pkg" ;;
  arch) pacman-key --init >/dev/null 2>&1; pacman -Syu --noconfirm >/dev/null; pacman -U --noconfirm "$pkg" >/dev/null ;;
esac

ross-fisheye --version | tee /tmp/version
grep -qx "ross-fisheye $version" /tmp/version
ross-fisheye --help | head -3

ldd /usr/bin/ross-fisheye | grep -i glfw && { echo 'GLFW should be built in' >&2; exit 1; }
if ldd /usr/bin/ross-fisheye | grep 'not found'; then
  echo "missing libraries" >&2; exit 1
fi
# GLFW is built in; these are what it opens at run time.
for lib in libwayland-client.so.0 libwayland-cursor.so.0 libwayland-egl.so.1 libxkbcommon.so.0 libEGL.so.1; do
  ldconfig -p | grep -q "$lib" || { echo "missing $lib" >&2; exit 1; }
done
# Without a compositor it must fail cleanly, not crash.
if XDG_RUNTIME_DIR=/tmp WAYLAND_DISPLAY=nowhere ross-fisheye 2>/tmp/err; then
  echo "it should have failed without a compositor" >&2; exit 1
fi
cat /tmp/err | head -3
echo "Install test passed on $kind"
