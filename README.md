# ross_fisheye

A fisheye magnifier for Wayland. Press a shortcut and the screen freezes under a lens that bulges
it like cloth pushed up by a glass ball: no hard edge, the middle stretched, the slope lit from
above. Move the mouse to look around; press the shortcut again to close it.

It's a fork of [woomer](https://github.com/coffeeispower/woomer) by Tiago Dinis Ribeiro dos Santos,
which keeps working as the classic zoom (`--zoom`).

Install it from [repo.rossmel.com](https://repo.rossmel.com/fisheye/) · [Español](README.es.md)

## Install

Signed packages for pacman (CachyOS, Arch), APT (Ubuntu, Debian) and DNF (Fedora). Add the
repository once ([instructions](https://repo.rossmel.com/repository/)), then:

```sh
sudo pacman -S ross-fisheye-bin   # CachyOS, Arch
sudo apt install ross-fisheye     # Ubuntu, Debian
sudo dnf install ross-fisheye     # Fedora
```

It needs a Wayland compositor with the wlr-screencopy protocol: **niri, Hyprland, Sway**, river,
Wayfire and others. GNOME and KDE Plasma don't offer it, so it won't open there.

## Use

```sh
ross-fisheye               # the fisheye lens on the focused screen; run it again to close it
ross-fisheye --zoom        # woomer's classic zoom: drag, scroll, hold Ctrl for a spotlight
ross-fisheye --monitor DP-1
```

| In the fisheye lens | |
| --- | --- |
| Scroll wheel | Magnification |
| <kbd>Shift</kbd> + scroll wheel | Lens radius |
| <kbd>Esc</kbd>, right click or the shortcut again | Close |

The lens takes a still picture of the screen when it opens: it doesn't follow what changes
underneath while it's open.

## Shortcut and full screen

The lens has to cover the whole screen. With fractional scaling (1.25, 1.5…) it can't ask for
full screen by itself, so a window rule does it. Add both to your compositor:

**niri** (`~/.config/niri/config.kdl`)

```kdl
binds {
    Mod+Shift+Z hotkey-overlay-title="Magnifier" { spawn "ross-fisheye"; }
}
window-rule {
    match title="^ross-fisheye$"
    open-fullscreen true
}
```

**Hyprland** (`~/.config/hypr/hyprland.conf`)

```ini
bind = SUPER SHIFT, Z, exec, ross-fisheye
windowrulev2 = fullscreen, title:^(ross-fisheye)$
```

**Sway** (`~/.config/sway/config`)

```
bindsym Mod4+Shift+z exec ross-fisheye
for_window [title="^ross-fisheye$"] fullscreen enable
```

## Build

You need Rust, cmake, clang, pkg-config, GLFW 3.4 and the Wayland, xkbcommon, EGL and X11 development
files. Then:

```sh
cargo build --release   # target/release/ross-fisheye
```

`WOOMER_SHADER=shaders/fisheye.fs ross-fisheye` loads the lens shader from a file, to try changes
without rebuilding. Other looks I tried are in `shaders/variantes/`.

## License

MIT, like woomer. See [LICENSE](LICENSE).
