# ross_fisheye

Una lupa de ojo de pez para Wayland. Aprietas un atajo y la pantalla se congela bajo una lente que
la abomba como una tela empujada desde abajo por una bola de cristal: sin borde duro, el centro
estirado y la ladera iluminada desde arriba. Mueves el mouse para mirar; aprietas el atajo otra
vez para cerrarla.

Es un fork de [woomer](https://github.com/coffeeispower/woomer), de Tiago Dinis Ribeiro dos
Santos, que sigue funcionando como el zoom clásico (`--zoom`).

Instálala desde [repo.rossmel.com](https://repo.rossmel.com/fisheye/) · [English](README.md)

## Instalar

Paquetes firmados para pacman (CachyOS, Arch), APT (Ubuntu, Debian) y DNF (Fedora). Agrega el
repositorio una vez ([instrucciones](https://repo.rossmel.com/repository/)) y luego:

```sh
sudo pacman -S ross-fisheye-bin   # CachyOS, Arch
sudo apt install ross-fisheye     # Ubuntu, Debian
sudo dnf install ross-fisheye     # Fedora
```

Necesita un compositor Wayland con el protocolo wlr-screencopy: **niri, Hyprland, Sway**, river,
Wayfire y otros. GNOME y KDE Plasma no lo ofrecen, así que ahí no abre.

## Uso

```sh
ross-fisheye               # la lente en la pantalla enfocada; vuelve a correrlo para cerrarla
ross-fisheye --zoom        # el zoom clásico de woomer: arrastrar, rueda, Ctrl para el foco
ross-fisheye --monitor DP-1
```

| En la lente | |
| --- | --- |
| Rueda | Aumento |
| <kbd>Shift</kbd> + rueda | Radio de la lente |
| <kbd>Esc</kbd>, clic derecho o el atajo otra vez | Cerrar |

La lente toma una foto fija de la pantalla al abrirse: no sigue lo que cambia debajo mientras
está abierta.

## Atajo y pantalla completa

La lente tiene que cubrir toda la pantalla. Con escala fraccional (1,25, 1,5…) no puede pedir la
pantalla completa sola, así que lo hace una regla de ventana. Agrega las dos cosas a tu
compositor:

**niri** (`~/.config/niri/config.kdl`)

```kdl
binds {
    Mod+Shift+Z hotkey-overlay-title="Lupa" { spawn "ross-fisheye"; }
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

## Compilar

Necesitas Rust, cmake, clang, pkg-config y los archivos de desarrollo de Wayland, xkbcommon, EGL
y X11. Luego:

```sh
cargo build --release   # target/release/ross-fisheye
```

`WOOMER_SHADER=shaders/fisheye.fs ross-fisheye` carga el shader de la lente desde un archivo, para
probar cambios sin recompilar. Otros aspectos que probé están en `shaders/variantes/`.

## Licencia

MIT, como woomer. Ver [LICENSE](LICENSE).
