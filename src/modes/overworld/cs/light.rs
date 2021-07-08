use macroquad::prelude::*;

/// Component for things that emit light.
pub struct Illuminator {
    /// Base light color (as a vec3)
    color: Vec3,
    kind: LightFalloffKind,
}

/// Stealing this entirely from Upper Crust
pub enum LightFalloffKind {
    /// Normal circular light that falls off equally in all directions.
    /// `m` controls how harshly the light falls off; lower is wider.
    Circular { m: f32 },
    /// Cone-shaped light
    Cone { m: f32, spread: f32, facing: f32 },
}

impl Illuminator {
    pub fn new(color: Vec3, kind: LightFalloffKind) -> Self {
        Self { color, kind }
    }

    /// Given the position of this light and the target position in the world,
    /// get the amount of light that this contributes to that target.
    ///
    /// (As a RGB vector.)
    ///
    /// I ripped this directly from Upper Crust and have forgotten how it works
    pub fn get_color(&self, pos: Vec2, target: Vec2) -> Vec3 {
        match self.kind {
            LightFalloffKind::Circular { m } => {
                let dist = target.distance(pos);
                let l = -m * dist + 1.0;
                self.color * l
            }
            LightFalloffKind::Cone { m, spread, facing } => {
                // Untransform the position by the facing dir
                let rotmat = Mat2::from_scale_angle(vec2(1.0, 1.0), -facing);
                let delta = rotmat * (target - pos);

                let l = if delta.x >= 0.0 {
                    (1.0 - m * delta.x - delta.y.abs() / delta.x / spread).max(0.0)
                } else {
                    0.0
                } + (1.0 - m * delta.length_squared() * 5.0).max(0.0);
                self.color * l.clamp(0.0, 1.0)
            }
        }
    }
}
