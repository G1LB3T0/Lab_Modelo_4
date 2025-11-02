mod mesh;
mod raster;
mod raster_z;
mod shader;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nalgebra_glm as glm;
use image; // para guardar PNG

use mesh::Mesh;
use raster::rgb;
use raster_z::tri_fill_z;
use shader::{Uniforms, TriInput, Shader, MetalLambert};

const WIDTH: usize  = 900;
const HEIGHT: usize = 700;

// Convierte el framebuffer ARGB (0xAARRGGBB) a PNG RGBA y guarda.
fn save_png(path: &str, buf: &[u32], w: usize, h: usize) -> Result<(), String> {
    let mut img = image::RgbaImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let px = buf[y * w + x];
            let a = ((px >> 24) & 0xFF) as u8;
            let r = ((px >> 16) & 0xFF) as u8;
            let g = ((px >> 8 ) & 0xFF) as u8;
            let b = ( px        & 0xFF) as u8;
            img.put_pixel(x as u32, y as u32, image::Rgba([r, g, b, a]));
        }
    }
    img.save(path).map_err(|e| e.to_string())
}

fn main() -> Result<(), String> {
    let mut window = Window::new(
        "OBJ – Ortográfico + ZBuffer – Optimizado + PNG",
        WIDTH, HEIGHT,
        WindowOptions::default(),
    ).map_err(|e| e.to_string())?;

    let mut color_buf = vec![rgb(8,10,14); WIDTH * HEIGHT];
    let mut depth_buf = vec![f32::INFINITY; WIDTH * HEIGHT];

    // Carga y encuadre por bounding box
    let mesh = Mesh::load_obj("assets/model.obj", (WIDTH.min(HEIGHT) as f32) * 0.48)?;

    // Estado
    let mut angle_x: f32 = 0.0;
    let mut angle_y: f32 = 0.0;
    let mut view_scale: f32 = 1.0;       // multiplicador adicional de escala en vista
    let mut ufo_scale_on: bool = false;  // aplanado opcional (toggle C)
    let mut cull_backfaces: bool = false; // toggle B

    // Luz y material (metal frío)
    let uniforms = Uniforms {
        base_color: (128, 160, 220),
        light_dir: glm::normalize(&glm::vec3(-0.35, 0.75, 0.25)),
        ambient: 0.22,
        spec_power: 32.0,
        spec_strength: 0.15,
        rim_strength: 0.12,
    };
    let shader = MetalLambert;

    let mut last = std::time::Instant::now();

    // Cachés por vértice
    let mut v_view:   Vec<glm::Vec3> = vec![glm::vec3(0.0,0.0,0.0); mesh.positions.len()];
    let mut v_screen: Vec<(f32,f32)> = vec![(0.0,0.0);             mesh.positions.len()];
    let mut v_znorm:  Vec<f32>       = vec![0.0;                   mesh.positions.len()];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = std::time::Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;

        // Controles
        if window.is_key_down(Key::A) { angle_y += 1.6 * dt; }
        if window.is_key_down(Key::D) { angle_y -= 1.6 * dt; }
        if window.is_key_down(Key::W) { angle_x += 1.0 * dt; }
        if window.is_key_down(Key::S) { angle_x -= 1.0 * dt; }

        if window.is_key_down(Key::Minus) { view_scale = (view_scale - 0.75 * dt).max(0.5); }
        if window.is_key_down(Key::Equal) { view_scale = (view_scale + 0.75 * dt).min(2.0); }

        if window.is_key_pressed(Key::C, KeyRepeat::No) { ufo_scale_on = !ufo_scale_on; }
        if window.is_key_pressed(Key::B, KeyRepeat::No) { cull_backfaces = !cull_backfaces; }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            angle_x = 0.0; angle_y = 0.0; view_scale = 1.0; ufo_scale_on = false;
        }

        color_buf.fill(rgb(8,10,14));
        depth_buf.fill(f32::INFINITY);

        // Modelo: rotación + aplanado opcional
        let rot_y = glm::rotation(angle_y, &glm::vec3(0.0, 1.0, 0.0));
        let rot_x = glm::rotation(angle_x, &glm::vec3(1.0, 0.0, 0.0));
        let model = if ufo_scale_on {
            let s = glm::scaling(&glm::vec3(1.10, 0.75, 1.10));
            rot_y * rot_x * s
        } else {
            rot_y * rot_x
        };

        // PASS 1: transformar vértices y obtener z_min/z_max
        let mut z_min = f32::INFINITY;
        let mut z_max = f32::NEG_INFINITY;
        for (i, v) in mesh.positions.iter().enumerate() {
            let q = (model * glm::vec4(v.x, v.y, v.z, 1.0)).xyz();
            v_view[i] = q;
            z_min = z_min.min(q.z);
            z_max = z_max.max(q.z);
        }
        if (z_max - z_min).abs() < 1e-9 { z_max = z_min + 1e-6; }
        let nz = |z: f32| (z - z_min) / (z_max - z_min);

        // PASS 2: proyectar a pantalla y normalizar Z (una vez por vértice)
        for (i, q) in v_view.iter().enumerate() {
            let s = mesh.to_screen_scaled(*q, WIDTH, HEIGHT, view_scale);
            v_screen[i] = (s.0 as f32, s.1 as f32);
            v_znorm[i]  = nz(q.z);
        }

        // PASS 3: raster por triángulo
        for f in &mesh.indices {
            let i0 = f[0] as usize;
            let i1 = f[1] as usize;
            let i2 = f[2] as usize;

            let q0 = v_view[i0];
            let q1 = v_view[i1];
            let q2 = v_view[i2];

            if cull_backfaces {
                let n = (q1 - q0).cross(&(q2 - q0));
                if n.z >= 0.0 { continue; } // cámara mira -Z en este “espacio vista”
            }

            let tri_in = TriInput { p0: q0, p1: q1, p2: q2 };
            let (r,g,b) = shader.shade(&uniforms, &tri_in);
            let color = rgb(r,g,b);

            let s0 = v_screen[i0];
            let s1 = v_screen[i1];
            let s2 = v_screen[i2];

            let v0 = (s0.0, s0.1, v_znorm[i0]);
            let v1 = (s1.0, s1.1, v_znorm[i1]);
            let v2 = (s2.0, s2.1, v_znorm[i2]);

            tri_fill_z(color, &mut color_buf, &mut depth_buf, WIDTH, HEIGHT, [v0, v1, v2]);
        }

        // Guardar PNG al presionar P
        if window.is_key_pressed(Key::P, KeyRepeat::No) {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            let filename = format!("render_{}.png", ts);
            if let Err(e) = save_png(&filename, &color_buf, WIDTH, HEIGHT) {
                eprintln!("Error al guardar PNG: {}", e);
            } else {
                println!("PNG guardado: {}", filename);
            }
        }

        // Presentar en pantalla
        window.update_with_buffer(&color_buf, WIDTH, HEIGHT)
              .map_err(|e| e.to_string())?;
    }

    Ok(())
}
