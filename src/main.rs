mod mesh;
mod raster;
mod raster_z;
mod shader;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nalgebra_glm as glm;

use mesh::Mesh;
use raster::rgb;
use raster_z::tri_fill_z;
use shader::{Uniforms, TriInput, Shader, MetalLambert};

const WIDTH: usize  = 900;
const HEIGHT: usize = 700;

fn main() -> Result<(), String> {
    let mut window = Window::new(
        "OBJ – Ortográfico + Sólido (Z-buffer) – Look OVNI ajustable",
        WIDTH, HEIGHT,
        WindowOptions::default(),
    ).map_err(|e| e.to_string())?;

    let mut color_buf = vec![rgb(8,10,14); WIDTH * HEIGHT];
    let mut depth_buf = vec![f32::INFINITY; WIDTH * HEIGHT];

    // Encadre por bounding box (esta escala ajusta el tamaño del modelo)
    let mesh = Mesh::load_obj("assets/model.obj", (WIDTH.min(HEIGHT) as f32) * 0.48)?;

    // Rotación y escala en vista
    let mut angle_x: f32 = 0.0;
    let mut angle_y: f32 = 0.0;
    let mut view_scale: f32 = 1.0;    // multiplicador adicional (teclado)
    let mut ufo_scale_on: bool = false; // aplanado opcional (toggle con C)

    // Paleta metálica fría (puedes cambiar base_color si quieres otro tinte)
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

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = std::time::Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;

        // Rotación
        if window.is_key_down(Key::A) { angle_y += 1.6 * dt; }
        if window.is_key_down(Key::D) { angle_y -= 1.6 * dt; }
        if window.is_key_down(Key::W) { angle_x += 1.0 * dt; }
        if window.is_key_down(Key::S) { angle_x -= 1.0 * dt; }

        // Escala en tiempo real: '-' reduce, '=' aumenta
        if window.is_key_down(Key::Minus) { view_scale = (view_scale - 0.75 * dt).max(0.5); }
        if window.is_key_down(Key::Equal) { view_scale = (view_scale + 0.75 * dt).min(2.0); }

        // Toggle de aplanado “ovni”
        if window.is_key_pressed(Key::C, KeyRepeat::No) {
            ufo_scale_on = !ufo_scale_on;
        }

        color_buf.fill(rgb(8,10,14));
        depth_buf.fill(f32::INFINITY);

        // Modelo: rotación + (opcional) aplanado leve en Y
        let rot_y = glm::rotation(angle_y, &glm::vec3(0.0, 1.0, 0.0));
        let rot_x = glm::rotation(angle_x, &glm::vec3(1.0, 0.0, 0.0));
        let model = if ufo_scale_on {
            let s = glm::scaling(&glm::vec3(1.10, 0.75, 1.10));
            rot_y * rot_x * s
        } else {
            rot_y * rot_x
        };

        // Rango z del frame para normalizar a [0,1] (ortográfica)
        let mut z_min = f32::INFINITY;
        let mut z_max = f32::NEG_INFINITY;
        for f in &mesh.indices {
            let p0 = mesh.positions[f[0] as usize];
            let p1 = mesh.positions[f[1] as usize];
            let p2 = mesh.positions[f[2] as usize];

            let q0 = (model * glm::vec4(p0.x, p0.y, p0.z, 1.0)).xyz();
            let q1 = (model * glm::vec4(p1.x, p1.y, p1.z, 1.0)).xyz();
            let q2 = (model * glm::vec4(p2.x, p2.y, p2.z, 1.0)).xyz();

            z_min = z_min.min(q0.z).min(q1.z).min(q2.z);
            z_max = z_max.max(q0.z).max(q1.z).max(q2.z);
        }
        if (z_max - z_min).abs() < 1e-9 { z_max = z_min + 1e-6; }
        let nz = |z: f32| (z - z_min) / (z_max - z_min);

        // Dibujar sólido con z-buffer (sin wireframe para que no se vean vértices)
        for f in &mesh.indices {
            let p0 = mesh.positions[f[0] as usize];
            let p1 = mesh.positions[f[1] as usize];
            let p2 = mesh.positions[f[2] as usize];

            let q0 = (model * glm::vec4(p0.x, p0.y, p0.z, 1.0)).xyz();
            let q1 = (model * glm::vec4(p1.x, p1.y, p1.z, 1.0)).xyz();
            let q2 = (model * glm::vec4(p2.x, p2.y, p2.z, 1.0)).xyz();

            // color por cara
            let tri_in = TriInput { p0: q0, p1: q1, p2: q2 };
            let (r,g,b) = shader.shade(&uniforms, &tri_in);
            let color = rgb(r,g,b);

            // proyección ortográfica a pantalla con multiplicador de escala en vista
            let s0 = mesh.to_screen_scaled(q0, WIDTH, HEIGHT, view_scale);
            let s1 = mesh.to_screen_scaled(q1, WIDTH, HEIGHT, view_scale);
            let s2 = mesh.to_screen_scaled(q2, WIDTH, HEIGHT, view_scale);

            let v0 = (s0.0 as f32, s0.1 as f32, nz(q0.z));
            let v1 = (s1.0 as f32, s1.1 as f32, nz(q1.z));
            let v2 = (s2.0 as f32, s2.1 as f32, nz(q2.z));
            tri_fill_z(color, &mut color_buf, &mut depth_buf, WIDTH, HEIGHT, [v0, v1, v2]);
        }

        window.update_with_buffer(&color_buf, WIDTH, HEIGHT)
              .map_err(|e| e.to_string())?;
    }

    Ok(())
}
