use std::{borrow::Cow, cell::RefCell, rc::Rc};
use thiserror::Error;
use wgpu::UncapturedErrorHandler;

use crate::gpu::{buffer::GpuBuffers, frame::Frame, geometry::AVertex};

struct Samplers {
    nearest: wgpu::Sampler,
    linear: wgpu::Sampler,
}

impl Samplers {
    fn new(device: &wgpu::Device) -> Samplers {
        let make = |filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: filter,
                ..Default::default()
            })
        };
        Samplers {
            nearest: make(wgpu::FilterMode::Nearest),
            linear: make(wgpu::FilterMode::Linear),
        }
    }
    fn get(&self, filter: wgpu::FilterMode) -> &wgpu::Sampler {
        match filter {
            wgpu::FilterMode::Nearest => &self.nearest,
            wgpu::FilterMode::Linear => &self.linear,
        }
    }
}

pub struct Layouts {
    pub texture: wgpu::BindGroupLayout,
    pub camera: wgpu::BindGroupLayout,
    pub standard: wgpu::PipelineLayout,
}

pub struct Texture {
    pub view: Rc<wgpu::TextureView>,
    pub bind_group: Rc<wgpu::BindGroup>,
}

struct TextureDesc<'a> {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    filter: wgpu::FilterMode,
    pixels: Option<&'a [u8]>,
}

pub struct Context<'surface> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pub config: wgpu::SurfaceConfiguration,
    pub white: Texture,
    pub layouts: Layouts,
    pub render_pipeline: wgpu::RenderPipeline,
    pub buffers: RefCell<GpuBuffers>,

    surface: wgpu::Surface<'surface>,
    samplers: Samplers,
}

#[derive(Error, Debug)]
pub enum ContextError {
    #[error("failed to obtain surface")]
    SurfaceError(Box<dyn std::error::Error>),
    #[error("adapter not found")]
    AdapterNotFound,
    #[error("failed to obtain adapter")]
    RequestError(#[from] wgpu::RequestDeviceError),
    #[error("failed to load PNG {0}")]
    PngError(minipng::Error),
    #[error("failed to get current texture of surface")]
    FrameError(#[from] wgpu::SurfaceError),
    #[error("failed to prepare a text render")]
    TextPrepareError(#[from] glyphon::PrepareError),
    #[error("failed to complete a text render")]
    TextRenderError(#[from] glyphon::RenderError),
    #[error("failed to perform layout")]
    LayoutError(#[from] taffy::TaffyError),
}

impl<'surface> Context<'surface> {
    pub async fn new<
        ESurface: std::error::Error + 'static,
        F: FnOnce(&wgpu::Instance) -> Result<wgpu::Surface<'surface>, ESurface>,
    >(
        width: u32,
        height: u32,
        maker: F,
        error_handler: Box<dyn UncapturedErrorHandler>,
    ) -> Result<Context<'surface>, ContextError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY | wgpu::Backends::SECONDARY,
            dx12_shader_compiler: Default::default(),
            ..Default::default()
        });
        let surface = maker(&instance).map_err(|e| ContextError::SurfaceError(Box::new(e)))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(ContextError::AdapterNotFound)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    label: Some("device"),
                    required_features: wgpu::Features::empty(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        device.on_uncaptured_error(error_handler);
        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let texture_format = surface_format.add_srgb_suffix();
        let view_formats = if texture_format != surface_format {
            vec![texture_format]
        } else {
            Vec::default()
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats,
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
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(64),
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

        let samplers = Samplers::new(&device);
        let white_texture = Context::make_texture(
            &device,
            &queue,
            &texture_bind_group_layout,
            &samplers,
            TextureDesc {
                width: 1,
                height: 1,
                format: texture_format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                filter: wgpu::FilterMode::Nearest,
                pixels: Some(&[255, 255, 255, 255]),
            },
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("ashader.wgsl"))),
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
                    format: texture_format,
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
        let layouts = Layouts {
            texture: texture_bind_group_layout,
            camera: matrix_bind_group_layout,
            standard: pipeline_layout,
        };

        Ok(Context {
            buffers: RefCell::new(GpuBuffers::new(&device, &layouts)),
            device,
            queue,
            format: texture_format,
            surface,
            config,
            layouts,
            samplers,
            white: white_texture,
            render_pipeline,
        })
    }

    fn make_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        samplers: &Samplers,
        desc: TextureDesc,
    ) -> Texture {
        let size = wgpu::Extent3d {
            width: desc.width,
            height: desc.height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: desc.format,
            usage: desc.usage,
            label: Some("texture"),
            view_formats: &[],
        });

        if let Some(pixels) = desc.pixels {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(desc.width * 4),
                    rows_per_image: Some(desc.height),
                },
                size,
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(samplers.get(desc.filter)),
                },
            ],
            label: Some("texture_bind_group"),
        });

        Texture {
            view: Rc::new(view),
            bind_group: Rc::new(bind_group),
        }
    }
    pub fn render_target(&self, width: u32, height: u32) -> Texture {
        Context::make_texture(
            &self.device,
            &self.queue,
            &self.layouts.texture,
            &self.samplers,
            TextureDesc {
                width,
                height,
                format: self.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                filter: wgpu::FilterMode::Nearest,
                pixels: None,
            },
        )
    }
    pub fn texture_from_png(
        &self,
        png_bytes: &[u8],
        filter: wgpu::FilterMode,
    ) -> Result<Texture, ContextError> {
        let header = minipng::decode_png_header(png_bytes).map_err(ContextError::PngError)?;
        let mut buffer = vec![0; header.required_bytes_rgba8bpc()];
        let mut png =
            minipng::decode_png(png_bytes, &mut buffer).map_err(ContextError::PngError)?;
        png.convert_to_rgba8bpc().map_err(ContextError::PngError)?;

        Ok(Context::make_texture(
            &self.device,
            &self.queue,
            &self.layouts.texture,
            &self.samplers,
            TextureDesc {
                width: png.width(),
                height: png.height(),
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                filter,
                pixels: Some(png.pixels()),
            },
        ))
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width as u32;
        self.config.height = height as u32;

        self.surface.configure(&self.device, &self.config);
    }
    pub fn frame<'context>(&'context self) -> Result<Frame<'context, 'surface>, ContextError> {
        let mut b = self.buffers.borrow_mut();
        b.vertices.reset();
        b.indices.reset();
        b.uniforms.reset();

        let frame = self
            .surface
            .get_current_texture()
            .map_err(ContextError::FrameError)?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.format),
            ..Default::default()
        });

        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("main_command_encoder"),
            });

        Ok(Frame::new(self, frame, view, encoder))
    }
}
