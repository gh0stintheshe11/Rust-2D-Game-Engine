// Add this at the top of the file
use wgpu::util::DeviceExt;

pub struct RenderEngine {
    pub texture_view: wgpu::TextureView,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

// Define vertex structure for 2D sprites
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl RenderEngine {
    pub fn new() -> Self {
        // Initialize wgpu using InstanceDescriptor
        let instance_desc = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            flags: wgpu::InstanceFlags::empty(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        };
        let instance = wgpu::Instance::new(instance_desc);

        let (device, queue) = futures::executor::block_on(async {
            let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            }).await.unwrap();

            adapter.request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            }, None).await.unwrap()
        });

        // Create a texture for rendering
        let texture_extent = wgpu::Extent3d {
            width: 1024,
            height: 1024,
            depth_or_array_layers: 1,
        };
        let texture_desc = wgpu::TextureDescriptor {
            label: Some("Render Texture"),
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create empty vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 2048, // Some initial size
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create index buffer with basic quad indices
        let indices: &[u16] = &[
            0, 1, 2,  // First triangle
            2, 1, 3,  // Second triangle
        ];
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create a temporary pipeline
        let pipeline = create_render_pipeline(&device);

        // Return an instance of Renderer
        Self {
            texture_view,
            device,
            queue,
            pipeline,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn render_frame(&mut self, sprites: &[Sprite]) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Update vertex buffer with sprite data
        let vertices = self.prepare_vertices(sprites);
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("2D Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // Draw all sprites
            render_pass.draw_indexed(0..6, 0, 0..sprites.len() as _);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    fn prepare_vertices(&self, sprites: &[Sprite]) -> Vec<Vertex> {
        // Convert sprites to vertices
        let mut vertices = Vec::with_capacity(sprites.len() * 4);
        for sprite in sprites {
            // Add four vertices for each sprite (rectangle)
            vertices.extend_from_slice(&sprite.to_vertices());
        }
        vertices
    }
}

// You'll need to define a Sprite struct in your game engine
#[derive(Debug, Clone)]
pub struct Sprite {
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub rotation: f32,
    pub texture_coords: (f32, f32, f32, f32), // (u1, v1, u2, v2)
}

impl Sprite {
    pub fn to_vertices(&self) -> [Vertex; 4] {
        // Convert sprite data to four vertices
        // This is a simplified version - you'll want to add rotation handling
        let (x, y) = self.position;
        let (w, h) = self.size;
        let (u1, v1, u2, v2) = self.texture_coords;

        [
            Vertex { position: [x, y, 0.0], tex_coords: [u1, v1] },
            Vertex { position: [x + w, y, 0.0], tex_coords: [u2, v1] },
            Vertex { position: [x, y + h, 0.0], tex_coords: [u1, v2] },
            Vertex { position: [x + w, y + h, 0.0], tex_coords: [u2, v2] },
        ]
    }
}

fn create_render_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }
            ],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
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
    })
}
