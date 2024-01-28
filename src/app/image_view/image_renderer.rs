use std::mem;

use cgmath::{Matrix4, SquareMatrix, Vector2};
use crevice::std140::AsStd140;
use num_traits::Zero;
use wgpu::util::DeviceExt;

use super::{texture, ImageView};
use crate::WgpuState;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub fn new(x: f32, y: f32, tex_x: f32, tex_y: f32) -> Self {
        Self {
            position: [x, y],
            tex_coords: [tex_x, tex_y],
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(AsStd140, Debug, Copy, Clone)]
pub struct Uniform {
    pub matrix: Matrix4<f32>,
    pub size: Vector2<f32>,
    pub hue: f32,
    pub contrast: f32,
    pub brightness: f32,
    pub saturation: f32,
    pub grayscale: u32,
    pub invert: u32,
}

impl Default for Uniform {
    fn default() -> Self {
        Self {
            matrix: Matrix4::identity(),
            size: Vector2::zero(),
            hue: Default::default(),
            contrast: Default::default(),
            brightness: Default::default(),
            saturation: Default::default(),
            grayscale: Default::default(),
            invert: Default::default(),
        }
    }
}

impl Uniform {
    fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Image fragment bind group"),
        })
    }
}

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub fn new(wgpu: &WgpuState) -> Self {
        let render_pipeline_layout =
            wgpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Image render pipeline layout"),
                    bind_group_layouts: &[
                        &Uniform::get_bind_group_layout(&wgpu.device),
                        &texture::Texture::get_bind_group_layout(&wgpu.device),
                    ],
                    push_constant_ranges: &[],
                });

        let vertex = wgpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Image vertex"),
                source: wgpu::ShaderSource::Glsl {
                    shader: include_str!("../../shader/image.vert").into(),
                    stage: wgpu::naga::ShaderStage::Vertex,
                    defines: Default::default(),
                },
            });

        let fragment = wgpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Image fragment"),
                source: wgpu::ShaderSource::Glsl {
                    shader: include_str!("../../shader/image.frag").into(),
                    stage: wgpu::naga::ShaderStage::Fragment,
                    defines: Default::default(),
                },
            });

        let pipeline = wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Image Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex,
                    entry_point: "main",
                    buffers: &[Vertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment,
                    entry_point: "main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu.config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
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
            });

        let value_std140 = Uniform::default().as_std140();
        let uniform_buffer = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex uniform buffer"),
                contents: value_std140.as_bytes(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let uniform_bind_group_layout = Uniform::get_bind_group_layout(&wgpu.device);
        let uniform_bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Vertex uniform bind group"),
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn prepare(&mut self, wgpu: &WgpuState, uniform: Uniform) {
        let value_std140 = uniform.as_std140();
        wgpu.queue
            .write_buffer(&self.uniform_buffer, 0, value_std140.as_bytes());
    }

    pub fn render<'rpass>(
        &'rpass mut self,
        rpass: &mut wgpu::RenderPass<'rpass>,
        image_view: &'rpass ImageView,
    ) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
        let mosaic = &image_view.mosaic[image_view.index];
        rpass.set_index_buffer(mosaic.indices.slice(..), wgpu::IndexFormat::Uint32);

        for tile in &mosaic.tiles {
            rpass.set_bind_group(1, &tile.texture.diffuse_bind_group, &[]);
            rpass.set_vertex_buffer(0, tile.vertices.slice(..));
            rpass.draw_indexed(0..6, 0, 0..1);
        }
    }
}
