use nalgebra_glm as glm;

pub struct Uniforms {
    pub base_color: (u8,u8,u8),   // color base
    pub light_dir: glm::Vec3,     // dirección de luz (espacio de vista)
    pub ambient: f32,             // 0..1
    pub spec_power: f32,          // dureza del brillo
    pub spec_strength: f32,       // intensidad especular 0..1
    pub rim_strength: f32,        // borde/silueta 0..1
}

pub struct TriInput {
    // posiciones del triángulo en “vista” (aquí usamos el modelo rotado)
    pub p0: glm::Vec3,
    pub p1: glm::Vec3,
    pub p2: glm::Vec3,
}

pub trait Shader {
    fn shade(&self, u:&Uniforms, tri:&TriInput) -> (u8,u8,u8);
}

// Shader metálico: ambient + lambert + especular (Blinn-Phong) + rim sutil
pub struct MetalLambert;

fn clamp01(x:f32)->f32 { x.max(0.0).min(1.0) }

impl Shader for MetalLambert {
    fn shade(&self, u:&Uniforms, tri:&TriInput) -> (u8,u8,u8) {
        let n_raw = (tri.p1 - tri.p0).cross(&(tri.p2 - tri.p0));
        let n = if n_raw.magnitude() > 1e-9 { n_raw.normalize() } else { glm::vec3(0.0,0.0,1.0) };

        let l = -u.light_dir.normalize();
        let v = glm::vec3(0.0, 0.0, 1.0); // cámara mirando -Z

        let ndotl = clamp01(n.dot(&l));

        // rim: resalta silueta sin wireframe
        let ndotv = clamp01(n.dot(&v));
        let rim = (1.0 - ndotv).powf(1.3) * u.rim_strength;

        // especular Blinn-Phong
        let h = (l + v).normalize();
        let spec = if ndotl > 0.0 {
            u.spec_strength * clamp01(n.dot(&h)).powf(u.spec_power)
        } else { 0.0 };

        let i = clamp01(u.ambient + (1.0 - u.ambient) * ndotl + rim + spec);

        let (r,g,b) = u.base_color;
        let rf = (r as f32 * i).clamp(0.0, 255.0) as u8;
        let gf = (g as f32 * i).clamp(0.0, 255.0) as u8;
        let bf = (b as f32 * i).clamp(0.0, 255.0) as u8;
        (rf,gf,bf)
    }
}
