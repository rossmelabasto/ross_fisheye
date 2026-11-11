#!/usr/bin/env bash
# Screenshots of ross-fisheye without anyone's desktop: a headless Sway shows a picture (the
# background) and the lens opens on it at a few spots. Software rendering, so it runs on a CI
# machine with no GPU.
#   packaging/demo.sh <background.png> <out-dir>      (ross-fisheye installed; needs sway, grim,
#                                                      Mesa)
set -euo pipefail
bg=$(realpath "$1")
out=$(realpath -m "$2")
mkdir -p "$out"

export XDG_RUNTIME_DIR
XDG_RUNTIME_DIR=$(mktemp -d)
chmod 700 "$XDG_RUNTIME_DIR"
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 LIBGL_ALWAYS_SOFTWARE=1
W=1280 H=720

conf="$XDG_RUNTIME_DIR/sway.conf"
cat > "$conf" <<EOF
output HEADLESS-1 resolution ${W}x${H} position 0 0 bg $bg fill
default_border none
for_window [title="^ross-fisheye$"] fullscreen enable
EOF
sway -c "$conf" >"$out/sway.log" 2>&1 &
sway_pid=$!
trap 'kill $sway_pid 2>/dev/null || true' EXIT
for _ in $(seq 1 50); do
  sock=$(ls "$XDG_RUNTIME_DIR"/sway-ipc.* 2>/dev/null | head -1 || true)
  [ -n "$sock" ] && [ -S "$XDG_RUNTIME_DIR/wayland-1" ] && break
  sleep 0.2
done
export SWAYSOCK=$sock WAYLAND_DISPLAY=wayland-1
swaymsg -t get_outputs | head -5

# Headless Sway has no mouse, so windows never get pointer events. wlrctl adds a virtual one
# (wlr-virtual-pointer) and moves it with real motion; "cursor set" only places it.
cx=0 cy=0
cursor() { swaymsg -q "seat seat0 cursor set $1 $2"; cx=$1 cy=$2; }
path() {
  local x0=$1 y0=$2 x1=$3 y1=$4 steps=$5 i nx ny
  for ((i = 1; i <= steps; i++)); do
    nx=$((x0 + (x1 - x0) * i / steps)) ny=$((y0 + (y1 - y0) * i / steps))
    wlrctl pointer move $((nx - cx)) $((ny - cy))
    cx=$nx cy=$ny
    sleep 0.04
  done
}

wlrctl pointer move 1 1
cursor 420 300
grim "$out/background.png"
# One still per spot: WOOMER_TEST_CURSOR pins the lens there (pointer motion doesn't reach
# windows on a headless Sway).
shot() {
  local name=$1 at=$2
  WOOMER_TEST_CURSOR="$at" ross-fisheye >>"$out/fisheye.log" 2>&1 &
  local lens=$!
  for _ in $(seq 1 120); do
    swaymsg -t get_tree | grep -q '"name": "ross-fisheye"' && break
    sleep 0.5
  done
  sleep 3
  grim "$out/$name.png"
  # Running it again closes the open lens (one shortcut toggles it).
  ross-fisheye
  sleep 2
  if kill -0 $lens 2>/dev/null; then echo "the lens didn't close on the second launch" >&2; exit 1; fi
}
shot fisheye-1 "300,170"
shot fisheye-2 "760,420"
shot fisheye-3 "1000,250"
ls -la "$out"
