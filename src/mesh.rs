use nalgebra_glm as glm;
use std::path::Path;

pub struct Mesh {
    pub positions: Vec<glm::Vec3>,
    pub indices:   Vec<[u32; 3]>,
    pub center:    glm::Vec3,
    pub scale:     f32,
}

impl Mesh {
    pub fn load_obj(path: &str, target_pixels: f32) -> Result<Self, String> {
        let (models, _materials) = tobj::load_obj(
            Path::new(path),
            &tobj::LoadOptions { triangulate: true, ..Default::default() },
        ).map_err(|e| format!("tobj error: {e}"))?;

        let mut positions: Vec<glm::Vec3> = Vec::new();
        let mut indices:   Vec<[u32; 3]>  = Vec::new();
        let mut base: u32 = 0;

        for m in models {
            let mesh = m.mesh;
            for i in (0..mesh.positions.len()).step_by(3) {
                positions.push(glm::vec3(
                    mesh.positions[i],
                    mesh.positions[i + 1],
                    mesh.positions[i + 2],
                ));
            }
            for i in (0..mesh.indices.len()).step_by(3) {
                indices.push([
                    base + mesh.indices[i] as u32,
                    base + mesh.indices[i + 1] as u32,
                    base + mesh.indices[i + 2] as u32,
                ]);
            }
            base += (mesh.positions.len() / 3) as u32;
        }

        if positions.is_empty() || indices.is_empty() {
            return Err("OBJ vacío o sin triángulos".into());
        }

        // bounding box → centro y escala a target_pixels
        let mut minv = positions[0];
        let mut maxv = positions[0];
        for p in &positions {
            minv = glm::min2(&minv, p);
            maxv = glm::max2(&maxv, p);
        }
        let center = (minv + maxv) * 0.5;
        let size   = maxv - minv;
        let max_dim = size.x.max(size.y).max(size.z).max(1e-6);
        let scale   = target_pixels / max_dim;

        Ok(Self { positions, indices, center, scale })
    }

    #[inline]
    pub fn to_screen(&self, v: glm::Vec3, width: usize, height: usize) -> (i32, i32) {
        let hw = (width as f32) * 0.5;
        let hh = (height as f32) * 0.5;
        let sx = (v.x - self.center.x) * self.scale + hw;
        let sy = hh - ((v.y - self.center.y) * self.scale);
        (sx.round() as i32, sy.round() as i32)
    }

    // Variante con multiplicador de escala en vista (para ajustar tamaño con teclado)
    #[inline]
    pub fn to_screen_scaled(&self, v: glm::Vec3, width: usize, height: usize, scale_mul: f32) -> (i32, i32) {
        let hw = (width as f32) * 0.5;
        let hh = (height as f32) * 0.5;
        let s  = self.scale * scale_mul;
        let sx = (v.x - self.center.x) * s + hw;
        let sy = hh - ((v.y - self.center.y) * s);
        (sx.round() as i32, sy.round() as i32)
    }
}
