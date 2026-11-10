use std::{
    env, fs,
    path::PathBuf,
    process::{self, Command},
};

use libwayshot::WayshotConnection;
use raylib::{
    ffi::{Image as FfiImage, SetWindowMonitor, ToggleFullscreen},
    prelude::*,
};
const SPOTLIGHT_TINT: Color = Color::new(0x00, 0x00, 0x00, 190);
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn lock_path() -> PathBuf {
    let dir = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(env::temp_dir);
    dir.join("ross-fisheye.pid")
}

/// A second launch closes the lens that's open instead of opening another one, so a single
/// keyboard shortcut toggles it.
fn close_running() -> bool {
    let Ok(pid) = fs::read_to_string(lock_path()) else { return false };
    let pid = pid.trim().to_string();
    if pid.is_empty() || pid == process::id().to_string() {
        return false;
    }
    // The pid file may be stale: only close it if that process is still ross-fisheye.
    let is_ours = fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|c| c.trim() == "ross-fisheye");
    is_ours && Command::new("kill").arg(&pid).status().is_ok_and(|s| s.success())
}

/// The output that has the focus, asked to the compositor (niri, Hyprland or Sway).
fn focused_output() -> Option<String> {
    let run = |cmd: &str, args: &[&str]| -> Option<serde_json::Value> {
        let out = Command::new(cmd).args(args).output().ok()?;
        out.status.success().then(|| serde_json::from_slice(&out.stdout).ok()).flatten()
    };
    let focused_in = |list: serde_json::Value| -> Option<String> {
        let m = list.as_array()?.iter().find(|m| m["focused"].as_bool() == Some(true))?;
        m["name"].as_str().map(String::from)
    };
    if env::var_os("NIRI_SOCKET").is_some() {
        return run("niri", &["msg", "-j", "focused-output"])?["name"].as_str().map(String::from);
    }
    if env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        return focused_in(run("hyprctl", &["-j", "monitors"])?);
    }
    if env::var_os("SWAYSOCK").is_some() {
        return focused_in(run("swaymsg", &["-t", "get_outputs", "-r"])?);
    }
    None
}

fn main() {
    let mut args = env::args();
    let bin = args.next().unwrap();

    let mut monitor_name: Option<String> = None;
    let mut fisheye = true;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fisheye" => fisheye = true,
            "--zoom" => fisheye = false,
            "--monitor" => {
                monitor_name = args.next().or_else(|| {
                    eprintln!("--monitor needs a value");
                    process::exit(1);
                })
            }
            "-V" | "--version" => {
                println!("ross-fisheye {VERSION}");
                process::exit(0);
            }
            "-h" | "--help" => print_help_and_exit(&bin),
            other => {
                eprintln!("Unknown option: {other}\n");
                print_help_and_exit(&bin);
            }
        }
    }

    if close_running() {
        return;
    }
    let _ = fs::write(lock_path(), process::id().to_string());
    // Without --monitor, the focused output; if the compositor can't say, the first one.
    let asked = monitor_name.is_some();
    let monitor_name = monitor_name.or_else(focused_output);

    let wayshot_connection = WayshotConnection::new().unwrap_or_else(|e| {
        let _ = fs::remove_file(lock_path());
        eprintln!(
            "Can't capture the screen ({e:?}).\nross-fisheye needs a Wayland compositor with wlr-screencopy, like niri, Hyprland or Sway."
        );
        process::exit(1);
    });
    let outputs = wayshot_connection.get_all_outputs();

    if outputs.is_empty() {
        eprintln!("No Wayland outputs found.");
        process::exit(1);
    }

    let selected_output = match monitor_name {
        None => &outputs[0],
        Some(ref name) => match outputs.iter().find(|out| &out.name == name) {
            Some(out) => out,
            None if !asked => &outputs[0],
            None => {
                eprintln!("Output '{}' not found.", name);
                process::exit(1);
            }
        },
    };

    let screenshot_image = wayshot_connection
        .screenshot_all(false)
        .expect("failed to take a screenshot")
        .to_rgba8();
    let (width, height) = screenshot_image.dimensions();
    // La ventana se mide en pixeles logicos (escala fraccional de niri, p. ej. 1.25) pero la captura
    // viene en pixeles fisicos con una sola pantalla y compuesta a tamano logico con varias.
    // Se calcula la escala comparando con el ancho logico total de todas las pantallas.
    let min_x = outputs.iter().map(|o| o.logical_region.inner.position.x).min().unwrap();
    let min_y = outputs.iter().map(|o| o.logical_region.inner.position.y).min().unwrap();
    let max_x = outputs
        .iter()
        .map(|o| o.logical_region.inner.position.x + o.logical_region.inner.size.width as i32)
        .max()
        .unwrap();
    let tex_scale = (max_x - min_x) as f32 / width as f32;
    // Origen de la pantalla elegida dentro de la captura (en pixeles logicos)
    let out_origin = Vector2::new(
        (selected_output.logical_region.inner.position.x - min_x) as f32,
        (selected_output.logical_region.inner.position.y - min_y) as f32,
    );
    if fisheye {
        // Buffer a resolucion real (escala 1.25): sin esto raylib pinta un viewport de tamano logico
        // en una esquina del buffer fisico.
        unsafe {
            raylib::ffi::SetConfigFlags(raylib::ffi::ConfigFlags::FLAG_WINDOW_HIGHDPI as u32);
        }
    }
    let (mut rl, thread) = raylib::init()
        .title(env!("CARGO_BIN_NAME"))
        .size(
            selected_output
                .logical_region
                .inner
                .size
                .width
                .try_into()
                .unwrap(),
            selected_output
                .logical_region
                .inner
                .size
                .height
                .try_into()
                .unwrap(),
        )
        .transparent()
        .undecorated()
        .vsync()
        .build();

    let idx = outputs
        .iter()
        .position(|o| o.name == selected_output.name)
        .expect("Monitor not found");

    // En modo ojo de pez no se pide pantalla completa a raylib: con escala fraccional (1.25) raylib
    // dibuja en el tamano fisico del monitor (2560x1600) pero el buffer real es el logico (2048x1280),
    // y la imagen sale encogida y el cursor desfasado. Lo hace niri con su window-rule (open-fullscreen).
    if !fisheye {
        unsafe {
            ToggleFullscreen();
        }

        unsafe {
            SetWindowMonitor(idx as i32);
        }
    }

    let screenshot_image = unsafe {
        Image::from_raw(FfiImage {
            // We can leak memory here because raylib will free the memory for us
            data: Box::new(screenshot_image.into_vec())
                .leak()
                .as_mut_ptr()
                .cast(),
            format: PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
            mipmaps: 1,
            width: width as i32,
            height: height as i32,
        })
    };
    let screenshot_texture = rl
        .load_texture_from_image(&thread, &screenshot_image)
        .expect("failed to load screenshot into a texture");
    #[cfg(feature = "dev")]
    let mut spotlight_shader = rl
        .load_shader(&thread, None, Some("shaders/spotlight.fs"))
        .expect("Failed to load spotlight shader");

    #[cfg(not(feature = "dev"))]
    let mut spotlight_shader =
        rl.load_shader_from_memory(&thread, None, Some(include_str!("../shaders/spotlight.fs")));
    // WOOMER_SHADER=/ruta/fisheye.fs carga el shader desde archivo (para probar cambios sin recompilar)
    let mut fisheye_shader = match env::var("WOOMER_SHADER") {
        Ok(path) => rl.load_shader(&thread, None, Some(path.as_str())),
        Err(_) => {
            rl.load_shader_from_memory(&thread, None, Some(include_str!("../shaders/fisheye.fs")))
        }
    };
    let fisheye_cursor_loc = fisheye_shader.get_shader_location("cursorPosition");
    let fisheye_radius_loc = fisheye_shader.get_shader_location("lensRadius");
    let fisheye_mag_loc = fisheye_shader.get_shader_location("lensMag");
    if env::var("WOOMER_DEBUG").is_ok() {
        eprintln!(
            "DEBUG tex={}x{} tex_scale={} out_origin=({},{}) screen={}x{} render={}x{} outputs={:?}",
            width, height, tex_scale, out_origin.x, out_origin.y,
            rl.get_screen_width(), rl.get_screen_height(), rl.get_render_width(), rl.get_render_height(),
            outputs.iter().map(|o| (o.name.clone(), o.logical_region.inner.position.x, o.logical_region.inner.position.y, o.logical_region.inner.size.width, o.logical_region.inner.size.height, o.physical_size.width, o.physical_size.height)).collect::<Vec<_>>()
        );
    }
    let mut lens_mag: f32 = 1.5;
    let mut lens_radius: f32 = 300.0;
    let mut lens_open: f32 = 0.0;
    let mut rl_camera = Camera2D::default();
    rl_camera.zoom = 1.0;
    rl_camera.target = Vector2::new(
        selected_output.logical_region.inner.position.x as f32,
        selected_output.logical_region.inner.position.y as f32,
    );

    let mut delta_scale = 0f64;
    let mut scale_pivot = rl.get_mouse_position();
    let mut velocity = Vector2::default();
    let mut spotlight_radius_multiplier = 1.0;
    let mut spotlight_radius_multiplier_delta = 0.0;

    #[cfg(feature = "dev")]
    let mut spotlight_tint_uniform_location;
    #[cfg(feature = "dev")]
    let mut cursor_position_uniform_location;
    #[cfg(feature = "dev")]
    let mut spotlight_radius_multiplier_uniform_location;
    #[cfg(not(feature = "dev"))]
    let spotlight_tint_uniform_location;
    #[cfg(not(feature = "dev"))]
    let cursor_position_uniform_location;
    #[cfg(not(feature = "dev"))]
    let spotlight_radius_multiplier_uniform_location;

    spotlight_tint_uniform_location = spotlight_shader.get_shader_location("spotlightTint");
    cursor_position_uniform_location = spotlight_shader.get_shader_location("cursorPosition");
    spotlight_radius_multiplier_uniform_location =
        spotlight_shader.get_shader_location("spotlightRadiusMultiplier");
    while !rl.window_should_close() {
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
            break;
        }
        if fisheye {
            // Rueda = aumento de la lente, Shift + rueda = radio. Esc o clic derecho = salir.
            let scroll = rl.get_mouse_wheel_move_v().y;
            if scroll != 0.0 {
                if rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT) {
                    lens_radius = (lens_radius + scroll * 25.0).clamp(80.0, 700.0);
                } else {
                    lens_mag = (lens_mag + scroll * 0.25).clamp(1.25, 8.0);
                }
            }
            // Animacion de apertura de la lente
            lens_open = (lens_open + rl.get_frame_time() * 7.0).min(1.0);
            let ease = 1.0 - (1.0 - lens_open).powi(3);
            let mut mouse = rl.get_mouse_position();
            let screen_height = rl.get_screen_height() as f32;
            // Solo para pruebas: WOOMER_TEST_CURSOR="x,y" (pixeles logicos) fija la posicion de la lente
            if let Ok(v) = env::var("WOOMER_TEST_CURSOR") {
                let xy: Vec<f32> = v.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                if xy.len() == 2 {
                    let dpi_t = rl.get_render_width() as f32 / rl.get_screen_width() as f32;
                    mouse = Vector2::new(xy[0] / dpi_t, xy[1] / dpi_t);
                }
            }
            // El raton viene en pixeles logicos y el shader trabaja en pixeles del buffer (HiDPI)
            let dpi = rl.get_render_width() as f32 / rl.get_screen_width() as f32;
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::BLACK);
            // raylib (HIGHDPI, no-Apple) escala el raton por screen/framebuffer (0.8), pero en Wayland las
            // coordenadas de GLFW ya son logicas: se deshace esa escala (mouse * dpi) y luego se pasa a
            // pixeles del buffer (* dpi) con el origen abajo-izquierda que usa gl_FragCoord.
            let logical_x = mouse.x * dpi;
            let logical_y = mouse.y * dpi;
            fisheye_shader.set_shader_value(
                fisheye_cursor_loc,
                Vector2::new(logical_x * dpi, (screen_height - logical_y) * dpi),
            );
            fisheye_shader.set_shader_value(fisheye_radius_loc, lens_radius * dpi * ease);
            fisheye_shader.set_shader_value(fisheye_mag_loc, 1.0 + (lens_mag - 1.0) * ease);
            let mut sm = d.begin_shader_mode(&mut fisheye_shader);
            sm.draw_texture_ex(&screenshot_texture, Vector2::new(-out_origin.x, -out_origin.y), 0.0, tex_scale, Color::WHITE);
            continue;
        }
        #[cfg(feature = "dev")]
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            spotlight_shader = rl
                .load_shader(&thread, None, Some("shaders/spotlight.fs"))
                .expect("Failed to load spotlight shader");
            spotlight_tint_uniform_location = spotlight_shader.get_shader_location("spotlightTint");
            cursor_position_uniform_location =
                spotlight_shader.get_shader_location("cursorPosition");
            spotlight_radius_multiplier_uniform_location =
                spotlight_shader.get_shader_location("spotlightRadiusMultiplier");
        }
        let enable_spotlight =
            rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        let scrolled_amount = rl.get_mouse_wheel_move_v().y;
        if rl.is_key_pressed(KeyboardKey::KEY_LEFT_CONTROL)
            || rl.is_key_pressed(KeyboardKey::KEY_RIGHT_CONTROL)
        {
            spotlight_radius_multiplier = 5.0;
            spotlight_radius_multiplier_delta = -15.0;
        }
        if scrolled_amount != 0.0 {
            match (
                enable_spotlight,
                rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT),
            ) {
                (_, false) => {
                    delta_scale += scrolled_amount as f64;
                }
                (true, true) => {
                    spotlight_radius_multiplier_delta -= scrolled_amount as f64;
                }
                _ => {}
            }
            scale_pivot = rl.get_mouse_position();
        }
        if delta_scale.abs() > 0.5 {
            let p0 = scale_pivot / rl_camera.zoom;
            rl_camera.zoom = (rl_camera.zoom as f64 + delta_scale * rl.get_frame_time() as f64)
                .clamp(1.0, 10.) as f32;
            let p1 = scale_pivot / rl_camera.zoom;
            rl_camera.target += p0 - p1;
            delta_scale -= delta_scale * rl.get_frame_time() as f64 * 4.0
        }
        spotlight_radius_multiplier = (spotlight_radius_multiplier as f64
            + spotlight_radius_multiplier_delta * rl.get_frame_time() as f64)
            .clamp(0.3, 10.) as f32;
        spotlight_radius_multiplier_delta -=
            spotlight_radius_multiplier_delta * rl.get_frame_time() as f64 * 4.0;
        const VELOCITY_THRESHOLD: f32 = 15.0;
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            let delta = rl
                .get_screen_to_world2D(rl.get_mouse_position() - rl.get_mouse_delta(), rl_camera)
                - rl.get_screen_to_world2D(rl.get_mouse_position(), rl_camera);
            rl_camera.target += delta;
            velocity = delta * rl.get_fps().as_f32();
        } else if velocity.length_sqr() > VELOCITY_THRESHOLD * VELOCITY_THRESHOLD {
            rl_camera.target += velocity * rl.get_frame_time();
            velocity -= velocity * rl.get_frame_time() * 6.0;
        }

        let mut d = rl.begin_drawing(&thread);
        let mut mode2d = d.begin_mode2D(rl_camera);
        if enable_spotlight {
            mode2d.clear_background(SPOTLIGHT_TINT);
            let mouse_position = mode2d.get_mouse_position();
            spotlight_shader.set_shader_value(
                spotlight_tint_uniform_location,
                SPOTLIGHT_TINT.color_normalize(),
            );
            let screen_height = mode2d.get_screen_height().as_f32();
            spotlight_shader.set_shader_value(
                cursor_position_uniform_location,
                Vector2::new(mouse_position.x, screen_height - mouse_position.y),
            );
            spotlight_shader.set_shader_value(
                spotlight_radius_multiplier_uniform_location,
                spotlight_radius_multiplier,
            );

            let mut shader_mode = mode2d.begin_shader_mode(&mut spotlight_shader);
            shader_mode.draw_texture_ex(&screenshot_texture, Vector2::zero(), 0.0, tex_scale, Color::WHITE);
        } else {
            mode2d.clear_background(Color::get_color(0));
            mode2d.draw_texture_ex(&screenshot_texture, Vector2::zero(), 0.0, tex_scale, Color::WHITE);
        }
    }
    let _ = fs::remove_file(lock_path());
}

fn print_help_and_exit(bin: &str) -> ! {
    println!(
        "\
ross-fisheye {VERSION} – a fisheye magnifier for Wayland

USAGE:
    {bin} [--zoom] [--monitor <name>]

Run it again (or press Esc, or right-click) to close it, so one shortcut opens and closes it.

OPTIONS:
    --zoom             Classic zoom instead of the fisheye lens (drag, scroll, Ctrl = spotlight)
    --monitor <name>   Wayland output to magnify; by default the focused one on niri, Hyprland
                       and Sway, otherwise the first one
    -V, --version      Show the version
    -h, --help         Show this help

FISHEYE:
    Scroll wheel           magnification
    Shift + scroll wheel   lens radius",
        bin = bin
    );
    process::exit(0);
}
