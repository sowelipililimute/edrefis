// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

// use cgmath::{perspective, Deg, Matrix4, Point3, Rad, SquareMatrix, Vector2, Vector3, Vector4, Zero};
use glam::{Mat4, Vec2, Vec3, Vec3Swizzles};
use glyphon::fontdb;
use std::{borrow::Cow, rc::Rc, sync::Arc};
use wgpu::{util::DeviceExt, UncapturedErrorHandler};

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
    fn desc() -> wgpu::VertexBufferLayout<'static> {
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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MatrixUniform {
    matrix: [[f32; 4]; 4],
}

impl MatrixUniform {
    fn from(mtx: &Mat4) -> MatrixUniform {
        MatrixUniform {
            matrix: (*mtx).to_cols_array_2d(),
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
    uv_width: f32,
    uv_height: f32,
    color: wgpu::Color,
) -> ([AVertex; 4], [u16; 6]) {
    parallelogram(
        position,
        width * Vec3::X,
        height * Vec3::Y,
        uv_position,
        uv_width * Vec2::X,
        uv_height * Vec2::Y,
        color,
    )
}

pub trait Camera {
    fn matrix(&self, screen: &wgpu::SurfaceConfiguration) -> Mat4;
    fn texture(&self) -> Option<Rc<wgpu::TextureView>>;
}

#[derive(Debug)]
pub struct Camera2D {
    pub rotation: f32,
    pub zoom: Vec2,
    pub target: Vec2,
    pub offset: Vec2,
    pub texture: Option<Rc<wgpu::TextureView>>,
}

#[derive(Debug)]
pub struct Camera3D {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f32,
    pub texture: Option<Rc<wgpu::TextureView>>,
}

impl Camera2D {
    pub fn from_rect(
        position: Vec2,
        size: Vec2,
        texture: Option<Rc<wgpu::TextureView>>,
    ) -> Camera2D {
        let target = position + (size / 2.);

        Camera2D {
            target,
            zoom: Vec2::new(1. / size.x * 2., -1. / size.y * 2.),
            offset: Vec2::ZERO,
            rotation: 0.,
            texture,
        }
    }
}

impl Camera for Camera2D {
    fn matrix(&self, _screen: &wgpu::SurfaceConfiguration) -> Mat4 {
        let mat_origin = Mat4::from_translation(Vec3::new(-self.target.x, -self.target.y, 0.0));
        let mat_rotation = Mat4::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), self.rotation);

        let y_invert = if self.texture.is_some() { -1.0 } else { 1.0 };

        let mat_scale = Mat4::from_scale(Vec3::new(self.zoom.x, self.zoom.y * y_invert, 1.0));
        let mat_translation = Mat4::from_translation(Vec3::new(self.offset.x, self.offset.y, 0.0));

        mat_translation * ((mat_scale * mat_rotation) * mat_origin)
    }
    fn texture(&self) -> Option<Rc<wgpu::TextureView>> {
        self.texture.clone()
    }
}

// #[rustfmt::skip]
// const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.5,
//     0.0, 0.0, 0.0, 1.0,
// );

impl Camera for Camera3D {
    fn matrix(&self, screen: &wgpu::SurfaceConfiguration) -> Mat4 {
        let aspect = screen.width as f32 / screen.height as f32;

        let view = Mat4::look_at_rh(self.position, self.target, self.up);
        let proj = Mat4::perspective_rh_gl(self.fov_y, aspect, 0.01, 10000.0);

        return proj * view;
    }
    fn texture(&self) -> Option<Rc<wgpu::TextureView>> {
        self.texture.clone()
    }
}

impl Default for Camera3D {
    fn default() -> Camera3D {
        Camera3D {
            position: Vec3::new(0., 0., 35.),
            target: Vec3::new(0., 0., 0.),
            up: Vec3::Y,
            fov_y: 45.0_f32.to_radians(),
            texture: None,
        }
    }
}

pub struct State<'a> {
    frame_texture: Option<Rc<wgpu::TextureView>>,
    frame: Option<wgpu::SurfaceTexture>,
    texture_format: wgpu::TextureFormat,

    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    matrix_bind_group_layout: wgpu::BindGroupLayout,
    white_texture: Rc<wgpu::BindGroup>,

    active_render_pass: Option<(wgpu::CommandEncoder, wgpu::RenderPass<'static>)>,

    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    text_renderer: glyphon::TextRenderer,

    camera_matrix: Mat4,
    camera_texture: Option<Rc<wgpu::TextureView>>,
    active_bind_group: Rc<wgpu::BindGroup>,
    vertices: Vec<AVertex>,
    indices: Vec<u16>,
}

impl State<'_> {
    pub async fn new<'a, F: FnOnce(&wgpu::Instance) -> Result<wgpu::Surface<'a>, String>>(
        width: u32,
        height: u32,
        maker: F,
        error_handler: Box<dyn UncapturedErrorHandler>,
    ) -> Result<State<'a>, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY | wgpu::Backends::SECONDARY,
            dx12_shader_compiler: Default::default(),
            ..Default::default()
        });
        let surface = maker(&instance).map_err(|e| format!("failed to obtain surface: {}", e))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or("adapter not found")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    label: Some("device"),
                    required_features: wgpu::Features::empty(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())
            .map_err(|e| format!("failed to obtain adapter: {}", e))?;

        device.on_uncaptured_error(error_handler);
        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::default(),
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let matrix_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("matrix_bind_group_layout"),
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&texture_bind_group_layout, &matrix_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("ashader.wgsl"))),
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[AVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let texture_format = config.format;
        let white_texture = Rc::new(State::white_texture(
            &device,
            &queue,
            &texture_bind_group_layout,
            texture_format,
        ));

        // Set up text renderer
        let font_system = glyphon::FontSystem::new_with_fonts([
            fontdb::Source::Binary(Arc::new(include_bytes!("font/HankenGrotesk-Bold.ttf"))),
            fontdb::Source::Binary(Arc::new(include_bytes!("font/HankenGrotesk-Medium.ttf"))),
        ]);
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let viewport = glyphon::Viewport::new(&device, &cache);
        let mut atlas = glyphon::TextAtlas::new(&device, &queue, &cache, texture_format);
        let text_renderer = glyphon::TextRenderer::new(
            &mut atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );

        Ok(State {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            texture_bind_group_layout,
            matrix_bind_group_layout,
            white_texture: white_texture.clone(),

            frame: None,
            frame_texture: None,
            texture_format,

            active_render_pass: None,

            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,

            camera_matrix: Mat4::IDENTITY,
            camera_texture: None,
            active_bind_group: white_texture,
            vertices: vec![],
            indices: vec![],
        })
    }
    fn white_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> wgpu::BindGroup {
        let size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("blocks"),
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        texture_bind_group
    }
    pub fn create_texture(
        &self,
        width: u32,
        height: u32,
    ) -> (Rc<wgpu::BindGroup>, Rc<wgpu::TextureView>) {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.texture_format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("blocks"),
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        (Rc::new(texture_bind_group), Rc::new(texture_view))
    }
    pub fn upload_texture(
        &self,
        png_bytes: &[u8],
        filter: wgpu::FilterMode,
    ) -> Result<Rc<wgpu::BindGroup>, String> {
        let header = minipng::decode_png_header(png_bytes)
            .map_err(|e| e.to_string())
            .map_err(|e| format!("failed to decode PNG header: {}", e))?;
        let mut buffer = vec![0; header.required_bytes_rgba8bpc()];
        let mut png = minipng::decode_png(png_bytes, &mut buffer)
            .map_err(|e| e.to_string())
            .map_err(|e| format!("failed to decode PNG: {}", e))?;
        png.convert_to_rgba8bpc()
            .map_err(|e| e.to_string())
            .map_err(|e| format!("failed to convert PNG to rgba8bpc: {}", e))?;

        let size = wgpu::Extent3d {
            width: png.width(),
            height: png.height(),
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("blocks"),
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            png.pixels(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(png.bytes_per_row() as u32),
                rows_per_image: Some(png.height()),
            },
            size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: filter,
            ..Default::default()
        });

        let texture_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        Ok(Rc::new(texture_bind_group))
    }
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.config.width = width as u32;
        self.config.height = height as u32;

        self.frame_texture = None;
        self.frame = None;

        self.surface.configure(&self.device, &self.config);

        Ok(())
    }
    fn get_frame_view(&mut self) -> Result<Rc<wgpu::TextureView>, String> {
        if let Some(view) = &self.frame_texture {
            return Ok(view.clone());
        }

        let frame = self
            .surface
            .get_current_texture()
            .map_err(|e| format!("failed to get current texture of surface: {}", e))?;
        let view = Rc::new(
            frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        self.frame = Some(frame);
        self.frame_texture = Some(view.clone());

        Ok(view)
    }
    pub fn queue_draw<const V: usize, const I: usize>(&mut self, data: ([AVertex; V], [u16; I])) {
        let (v, i) = data;
        let count = self.vertices.len() as u16;
        self.indices.extend(i.iter().map(|x| *x + count));
        self.vertices.extend_from_slice(&v);
    }
    pub fn set_texture(&mut self, texture: Option<Rc<wgpu::BindGroup>>) {
        self.active_bind_group = texture.unwrap_or(self.white_texture.clone());
    }
    pub fn set_camera(&mut self, camera: &dyn Camera) {
        self.camera_matrix = camera.matrix(&self.config);
        self.camera_texture = camera.texture();
    }
    pub fn start_render_pass(&mut self, clear: Option<wgpu::Color>) -> Result<(), String> {
        let target = match &self.camera_texture {
            Some(texture) => texture.clone(),
            None => self.get_frame_view()?,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("main_command_encoder"),
            });

        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: clear.map(wgpu::LoadOp::Clear).unwrap_or(wgpu::LoadOp::Load),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                label: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();
        pass.set_pipeline(&self.render_pipeline);

        self.active_render_pass = Some((encoder, pass));

        Ok(())
    }
    pub fn complete_render_pass(&mut self) -> Result<(), String> {
        let (encoder, render_pass) = std::mem::replace(&mut self.active_render_pass, None)
            .ok_or("tried to complete a render pass without one being active")?;

        drop(render_pass);
        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
    pub fn do_draw(&mut self) -> Result<(), String> {
        if self.vertices.is_empty() {
            return Ok(());
        }
        let (_, ref mut render_pass) = self
            .active_render_pass
            .as_mut()
            .ok_or("tried to draw without a render pass being active")?;

        let matrix = MatrixUniform::from(&self.camera_matrix);

        let matrix_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Matrix Buffer"),
                contents: bytemuck::cast_slice(&[matrix]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let matrix_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.matrix_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: matrix_buffer.as_entire_binding(),
            }],
            label: Some("matrix_bind_group"),
        });

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Well Vertex Buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Well Index Buffer"),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        let num_indices = self.indices.len() as u32;

        render_pass.set_bind_group(0, self.active_bind_group.as_ref(), &[]);
        render_pass.set_bind_group(1, &matrix_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..num_indices, 0, 0..1);

        self.vertices.clear();
        self.indices.clear();

        Ok(())
    }
    pub fn create_buffer(&mut self) -> glyphon::Buffer {
        let mut text_buffer =
            glyphon::Buffer::new(&mut self.font_system, glyphon::Metrics::new(30.0, 42.0));

        text_buffer.set_size(&mut self.font_system, None, None);

        text_buffer
    }
    pub fn set_buffer_text<'r, 's, I>(
        &mut self,
        buffer: &mut glyphon::Buffer,
        spans: I,
        default_attrs: glyphon::Attrs,
    ) where
        I: IntoIterator<Item = (&'s str, glyphon::Attrs<'r>)>,
    {
        buffer.set_rich_text(
            &mut self.font_system,
            spans,
            default_attrs,
            glyphon::Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
    }
    pub fn world_to_view(&self, point: Vec3) -> Vec2 {
        let transformed = (self.camera_matrix.project_point3(point).xy() / Vec2::new(2., -2.))
            + Vec2::new(0.5, 0.5);
        let screen_size = Vec2::new(self.config.width as f32, self.config.height as f32);

        transformed * screen_size
    }
    pub fn draw_text(&mut self, buffer: &mut glyphon::Buffer, point: Vec2) -> Result<(), String> {
        self.viewport.update(
            &self.queue,
            glyphon::Resolution {
                width: self.config.width,
                height: self.config.height,
            },
        );
        self.text_renderer
            .prepare(
                &mut self.device,
                &mut self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &mut self.viewport,
                [glyphon::TextArea {
                    buffer,
                    left: point.x,
                    top: point.y,
                    scale: 1.0,
                    bounds: glyphon::TextBounds {
                        left: 0,
                        top: 0,
                        right: self.config.width as i32,
                        bottom: self.config.height as i32,
                    },
                    default_color: glyphon::Color::rgb(255, 255, 255),
                    custom_glyphs: &[],
                }],
                &mut self.swash_cache,
            )
            .map_err(|e| e.to_string())
            .map_err(|e| format!("failed to prepare a text render: {}", e))?;

        let (_, ref mut render_pass) = self
            .active_render_pass
            .as_mut()
            .ok_or("tried to draw without a render pass being active")?;

        self.text_renderer
            .render(&self.atlas, &self.viewport, render_pass)
            .map_err(|e| e.to_string())
            .map_err(|e| format!("failed to complete a text render: {}", e))?;

        Ok(())
    }
    pub fn present(&mut self) -> Result<(), String> {
        self.frame_texture = None;
        let frame = self
            .frame
            .take()
            .ok_or("tried to present without a frame being acquired")?;

        frame.present();

        Ok(())
    }
}
