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

#[derive(Debug)]
pub struct Camera3D {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f32,
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

impl Camera for Camera3D {
    fn matrix(&self, screen: &wgpu::SurfaceConfiguration) -> Mat4 {
        let aspect = screen.width as f32 / screen.height as f32;

        let view = Mat4::look_at_rh(self.position, self.target, self.up);
        let proj = Mat4::perspective_rh_gl(self.fov_y, aspect, 0.01, 10000.0);

        return proj * view;
    }
}

impl Default for Camera3D {
    fn default() -> Camera3D {
        Camera3D {
            position: Vec3::new(0., 0., -35.),
            target: Vec3::new(0., 0., 0.),
            up: Vec3::NEG_Y,
            fov_y: 45.0_f32.to_radians(),
        }
    }
}
