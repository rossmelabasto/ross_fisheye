#!/usr/bin/env bash
# Record the demo of ross-fisheye without anyone's desktop: a headless Sway shows a picture
# (the background), the lens opens on it and the cursor is moved by swaymsg along a path while
# wf-recorder records. Software rendering, so it runs on a CI machine with no GPU.
#   packaging/demo.sh <background.png> <out-dir>      (ross-fisheye installed; needs sway, grim,
#                                                      wf-recorder, ffmpeg, Mesa)
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

cursor() { swaymsg -q "seat seat0 cursor set $1 $2"; }

cursor 420 300
grim "$out/background.png"
ross-fisheye >"$out/fisheye.log" 2>&1 &
lens=$!
sleep 4
grim "$out/still-1.png"

wf-recorder -y -f "$out/raw.mp4" -c libx264 -p crf=20 -r 30 >"$out/recorder.log" 2>&1 &
rec=$!
sleep 1
# A slow loop over the page: left to right across the top, then down and back.
path() {
  local x0=$1 y0=$2 x1=$3 y1=$4 steps=$5 i
  for ((i = 0; i <= steps; i++)); do
    cursor $((x0 + (x1 - x0) * i / steps)) $((y0 + (y1 - y0) * i / steps))
    sleep 0.04
  done
}
path 420 300 860 260 60
path 860 260 980 480 50
grim "$out/still-2.png"
path 980 480 520 520 70
path 520 520 420 300 50
sleep 0.5
kill -INT $rec
wait $rec || true
grim "$out/still-3.png"

# Run again: the open lens closes (the same shortcut toggles it).
ross-fisheye
sleep 1
if kill -0 $lens 2>/dev/null; then echo "the lens didn't close on the second launch" >&2; exit 1; fi

ffmpeg -loglevel error -y -i "$out/raw.mp4" -vf "scale=1280:-2,format=yuv420p" -c:v libx264 -crf 24 -preset slow -movflags +faststart -an "$out/fisheye.mp4"
ffmpeg -loglevel error -y -i "$out/raw.mp4" -vf "scale=1280:-2" -c:v libvpx-vp9 -crf 38 -b:v 0 -an "$out/fisheye.webm"
rm -f "$out/raw.mp4"
ls -la "$out"
