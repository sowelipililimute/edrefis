use glam::{Vec2, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AVertex {
    position: [f32; 3],
    color: [f32; 4],
    uv: [f32; 2],
}

impl AVertex {
    fn new(position: Vec3, color: wgpu::Color, uv: Vec2) -> AVertex {
        AVertex {
            position: position.into(),
            color: [
                color.r as f32,
                color.g as f32,
                color.b as f32,
                color.a as f32,
            ],
            uv: [uv.x, uv.y],
        }
    }
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<AVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 7]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub fn parallelogram(
    position: Vec3,
    edge1: Vec3,
    edge2: Vec3,
    uv_position: Vec2,
    uv_edge1: Vec2,
    uv_edge2: Vec2,
    color: wgpu::Color,
) -> ([AVertex; 4], [u16; 6]) {
    (
        [
            AVertex::new(position, color, uv_position),
            AVertex::new(position + edge1, color, uv_position + uv_edge1),
            AVertex::new(
                position + edge1 + edge2,
                color,
                uv_position + uv_edge1 + uv_edge2,
            ),
            AVertex::new(position + edge2, color, uv_position + uv_edge2),
        ],
        [0, 1, 2, 0, 2, 3],
    )
}

pub fn rectangle(
    position: Vec3,
    width: f32,
    height: f32,
    uv_position: Vec2,
    uv_size: Vec2,
    color: wgpu::Color,
) -> ([AVertex; 4], [u16; 6]) {
    parallelogram(
        position,
        width * Vec3::X,
        height * Vec3::Y,
        uv_position,
        uv_size.x * Vec2::X,
        uv_size.y * Vec2::Y,
        color,
    )
}

pub fn solid_rectangle(position: Vec2, size: Vec2, color: wgpu::Color) -> ([AVertex; 4], [u16; 6]) {
    rectangle(
        Vec3::new(position.x, position.y, 0.),
        size.x,
        size.y,
        Vec2::ZERO,
        Vec2::ONE,
        color,
    )
}
