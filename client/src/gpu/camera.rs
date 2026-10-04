use glam::{Mat4, Vec2, Vec3};

pub trait Camera {
    fn matrix(&self, screen: &wgpu::SurfaceConfiguration) -> Mat4;
}

#[derive(Debug)]
pub struct Camera2D {
    pub rotation: f32,
    pub zoom: Vec2,
    pub target: Vec2,
    pub offset: Vec2,
}

impl Camera2D {
    pub fn from_rect(position: Vec2, size: Vec2) -> Camera2D {
        let target = position + (size / 2.);

        Camera2D {
            target,
            zoom: Vec2::new(1. / size.x * 2., -1. / size.y * 2.),
            offset: Vec2::ZERO,
            rotation: 0.,
        }
    }
}

impl Camera for Camera2D {
    fn matrix(&self, _screen: &wgpu::SurfaceConfiguration) -> Mat4 {
        let mat_origin = Mat4::from_translation(Vec3::new(-self.target.x, -self.target.y, 0.0));
        let mat_rotation = Mat4::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), self.rotation);

        let mat_scale = Mat4::from_scale(Vec3::new(self.zoom.x, self.zoom.y, 1.0));
        let mat_translation = Mat4::from_translation(Vec3::new(self.offset.x, self.offset.y, 0.0));

        mat_translation * ((mat_scale * mat_rotation) * mat_origin)
    }
}

#[derive(Debug)]
pub struct TileCamera {
    pub height_tiles: f32,
    pub origin: Vec2,
}

impl TileCamera {
    pub fn tile_px(&self, screen: &wgpu::SurfaceConfiguration) -> f32 {
        (screen.height as f32 / self.height_tiles).floor().max(1.)
    }
}

impl Camera for TileCamera {
    fn matrix(&self, screen: &wgpu::SurfaceConfiguration) -> Mat4 {
        let (w, h) = (screen.width as f32, screen.height as f32);
        let tile_px = self.tile_px(screen);
        let snap = Vec3::new(
            ((w / 2.).floor() - w / 2.) * 2. / w,
            (h / 2. - (h / 2.).floor()) * 2. / h,
            0.,
        );

        Mat4::from_translation(snap)
            * Mat4::from_scale(Vec3::new(2. * tile_px / w, -2. * tile_px / h, 1.))
            * Mat4::from_translation(self.origin.extend(0.))
    }
}
