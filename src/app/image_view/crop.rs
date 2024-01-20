use std::mem;

use egui::{CursorIcon, PointerButton};
use once_cell::sync::OnceCell;

use crate::{rect::Rect, vec2::Vec2, WgpuState};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: Vec2<f32>,
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CropInput {
    start: Vec2<f32>,
    end: Vec2<f32>,
    size: Vec2<f32>,
}

unsafe impl bytemuck::Pod for CropInput {}
unsafe impl bytemuck::Zeroable for CropInput {}

impl Vertex {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

struct RenderState {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
}

pub struct Crop {
    pub rect: Option<Rect>,
    pub x: String,
    pub y: String,
    pub width: String,
    pub height: String,
    drag_m: bool,
    drag_l: bool,
    drag_r: bool,
    drag_t: bool,
    drag_b: bool,
    drag_rem: Vec2<f32>,
    dragging: bool,
}

impl Crop {
    pub fn new(wgpu: &WgpuState) -> Self {
        Self {
            rect: None,
            x: String::new(),
            y: String::new(),
            width: String::new(),
            height: String::new(),
            drag_m: false,
            drag_l: false,
            drag_r: false,
            drag_t: false,
            drag_b: false,
            drag_rem: Vec2::splat(0.0),
            dragging: false,
        }
    }

    pub fn cropping(&self) -> bool {
        self.rect.is_some()
    }

    pub fn render(
        &self,
        wgpu: &WgpuState,
        rpass: &mut wgpu::RenderPass,
        window_size: Vec2<f32>,
        position: Vec2<f32>,
        image_size: Vec2<f32>,
        scale: f32,
    ) {
        if let Some(ref rect) = self.rect {
            let start = position - (image_size / 2.0) * scale + rect.position * scale;
            let end = position - (image_size / 2.0) * scale + (rect.position + rect.size) * scale;

            thread_local! {
                pub static PIPELINE: OnceCell<RenderState> = OnceCell::new();
            }

            PIPELINE.with(|f| {
                /*let render_state = f.get_or_init(|| {
                    let vertex = wgpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Crop vertex"),
                        source: wgpu::ShaderSource::Glsl { shader: include_str!("../../shader/crop.vert").into(), stage: wgpu::naga::ShaderStage::Vertex, defines: Default::default() },
                    });

                    let fragment = wgpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Crop fragment"),
                        source: wgpu::ShaderSource::Glsl { shader: include_str!("../../shader/crop.frag").into(), stage: wgpu::naga::ShaderStage::Fragment, defines: Default::default() },
                    });

                    let uniform_bind_group_layout = wgpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &[wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }],
                        label: Some("camera_bind_group_layout"),
                    });

                    let render_pipeline_layout = wgpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Render Pipeline Layout"),
                        bind_group_layouts: &[&uniform_bind_group_layout],
                        push_constant_ranges: &[],
                    });

                    let pipeline = wgpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("Render Pipeline"),
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
                            cull_mode: Some(wgpu::Face::Back),
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

                    let crop_unifrom = CropInput {
                        start: start, end: end, size: window_size,
                    };

                    let uniform_buffer = wgpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Unifrom Buffer"),
                        contents: bytemuck::cast_slice(&[crop_unifrom]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

                    let uniform_bind_group = wgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &uniform_bind_group_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
                        }],
                        label: Some("camera_bind_group"),
                    });

                    let shape = [
                        Vertex::new(-1.0, 1.0),
                        Vertex::new(-1.0, -1.0),
                        Vertex::new(1.0, 1.0),
                        Vertex::new(1.0, -1.0),
                    ];
                    let index_buffer: [u32; 6] = [0, 1, 2, 2, 1, 3];

                    let vertices = wgpu
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(&shape),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                    let indices = wgpu
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Triangle Index Buffer"),
                            contents: bytemuck::cast_slice(&index_buffer),
                            usage: wgpu::BufferUsages::INDEX,
                        });

                    RenderState { pipeline, uniform_buffer, uniform_bind_group, uniform_bind_group_layout, vertices, indices }
                });

                let crop_unifrom = CropInput {
                    start: start, end: end, size: window_size,
                };

                wgpu.queue.write_buffer(&render_state.uniform_buffer, 0, bytemuck::cast_slice(&[crop_unifrom]));

                rpass.set_pipeline(&render_state.pipeline);
                rpass.set_bind_group(0, &render_state.uniform_bind_group, &[]);
                rpass.set_vertex_buffer(0, render_state.vertices.slice(..));
                rpass.set_index_buffer(render_state.indices.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..6, 0, 0..1);*/
            });
        }
    }

    pub fn handle_drag(
        &mut self,
        ui: &egui::Ui,
        position: Vec2<f32>,
        image_size: Vec2<f32>,
        scale: f32,
    ) -> bool {
        if let Some(ref mut crop) = self.rect {
            let pos = crop.position * scale + position - (image_size * scale / 2.0);
            let size = crop.size * scale;

            let mut t = false;
            let mut b = false;
            let mut r = false;
            let mut l = false;
            let mut m = false;

            let top = {
                let start = pos - Vec2::splat(5.0);
                let size = Vec2::new(size.x() + 10.0, 10.0);
                egui::Rect::from_min_size(start.into(), size.into())
            };

            let bottom = {
                let start = Vec2::new(pos.x() - 5.0, pos.y() + size.y() - 5.0);
                let size = Vec2::new(size.x() + 10.0, 10.0);
                egui::Rect::from_min_size(start.into(), size.into())
            };

            let left = {
                let start = pos - Vec2::splat(5.0);
                let size = Vec2::new(10.0, size.y() + 10.0);
                egui::Rect::from_min_size(start.into(), size.into())
            };

            let right = {
                let start = Vec2::new(pos.x() + size.x() - 5.0, pos.y() - 5.0);
                let size = Vec2::new(10.0, size.y() + 10.0);
                egui::Rect::from_min_size(start.into(), size.into())
            };

            {
                let res = ui.interact(top, ui.id(), egui::Sense::hover());
                if res.hovered() {
                    t = true;
                }

                let res = ui.interact(bottom, ui.id(), egui::Sense::hover());
                if res.hovered() {
                    b = true;
                }

                let res = ui.interact(left, ui.id(), egui::Sense::hover());
                if res.hovered() {
                    l = true;
                }

                let res = ui.interact(right, ui.id(), egui::Sense::hover());
                if res.hovered() {
                    r = true;
                }
            }

            if let Some(mouse) = ui.ctx().pointer_latest_pos() {
                if l && r {
                    if (left.center().x - mouse.x).abs() < (right.center().x - mouse.x).abs() {
                        r = false;
                    } else {
                        l = false;
                    }
                }

                if t && b {
                    if (top.center().y - mouse.y).abs() < (bottom.center().y - mouse.y).abs() {
                        b = false;
                    } else {
                        t = false;
                    }
                }
            }

            let middle = egui::Rect::from_min_size(pos.into(), size.into());
            let res = ui.interact(middle, ui.id(), egui::Sense::hover());
            if res.hovered() {
                m = true;
            }

            if t && l {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeNorthWest);
            } else if t && r {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeNorthEast);
            } else if b && l {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeSouthWest);
            } else if b && r {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeSouthEast);
            } else if t || b {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
            } else if l || r {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            } else if m {
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            }

            let res = ui.interact(egui::Rect::EVERYTHING, ui.id(), egui::Sense::drag());
            if res.dragged_by(PointerButton::Primary) {
                if !self.dragging {
                    self.dragging = true;
                    self.drag_m = m;
                    self.drag_l = l;
                    self.drag_r = r;
                    self.drag_t = t;
                    self.drag_b = b;
                } else {
                    let mut delta = Vec2::from(res.drag_delta() / scale) + self.drag_rem;

                    if delta.x().abs() > 1.0 {
                        *self.drag_rem.mut_x() = delta.x() % 1.0;
                        *delta.mut_x() = delta.x() - delta.x() % 1.0;
                    } else {
                        *self.drag_rem.mut_x() = delta.x();
                        *delta.mut_x() = 0.0;
                    }

                    if delta.y().abs() > 1.0 {
                        *self.drag_rem.mut_y() = delta.y() % 1.0;
                        *delta.mut_y() = delta.y() - delta.y() % 1.0;
                    } else {
                        *self.drag_rem.mut_y() = delta.y();
                        *delta.mut_y() = 0.0;
                    }

                    let (new_pos, new_size) = if self.drag_t && self.drag_l {
                        let new_pos = crop.position + delta;
                        let new_size = crop.size - delta;
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeNorthWest);
                        (new_pos, new_size)
                    } else if self.drag_t && self.drag_r {
                        let new_pos = Vec2::new(crop.position.x(), crop.position.y() + delta.y());
                        let new_size =
                            Vec2::new(crop.size.x() + delta.x(), crop.size.y() - delta.y());
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeNorthEast);
                        (new_pos, new_size)
                    } else if self.drag_b && self.drag_l {
                        let new_pos = Vec2::new(crop.position.x() + delta.x(), crop.position.y());
                        let new_size =
                            Vec2::new(crop.size.x() - delta.x(), crop.size.y() + delta.y());
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeSouthWest);
                        (new_pos, new_size)
                    } else if self.drag_b && self.drag_r {
                        let new_pos = crop.position;
                        let new_size = crop.size + delta;
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeSouthEast);
                        (new_pos, new_size)
                    } else if self.drag_t {
                        let new_pos = Vec2::new(crop.position.x(), crop.position.y() + delta.y());
                        let new_size = Vec2::new(crop.size.x(), crop.size.y() - delta.y());
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                        (new_pos, new_size)
                    } else if self.drag_b {
                        let new_pos = Vec2::new(crop.position.x(), crop.position.y());
                        let new_size = Vec2::new(crop.size.x(), crop.size.y() + delta.y());
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                        (new_pos, new_size)
                    } else if self.drag_l {
                        let new_pos = Vec2::new(crop.position.x() + delta.x(), crop.position.y());
                        let new_size = Vec2::new(crop.size.x() - delta.x(), crop.size.y());
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                        (new_pos, new_size)
                    } else if self.drag_r {
                        let new_pos = Vec2::new(crop.position.x(), crop.position.y());
                        let new_size = Vec2::new(crop.size.x() + delta.x(), crop.size.y());
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                        (new_pos, new_size)
                    } else if self.drag_m {
                        crop.position += delta;
                        let max = (image_size - crop.size).max(0.0, 0.0);
                        *crop.position.mut_x() = crop.position.x().clamp(0.0, max.x());
                        *crop.position.mut_y() = crop.position.y().clamp(0.0, max.y());
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                        (crop.position, crop.size)
                    } else {
                        (crop.position, crop.size)
                    };

                    if new_size.x() >= 1.0 && new_pos.x() >= 0.0 {
                        *crop.position.mut_x() = new_pos.x();
                        *crop.size.mut_x() = new_size.x();
                    }
                    if new_size.y() >= 1.0 && new_pos.y() >= 0.0 {
                        *crop.position.mut_y() = new_pos.y();
                        *crop.size.mut_y() = new_size.y();
                    }

                    self.x = crop.x().to_string();
                    self.y = crop.y().to_string();
                    self.width = crop.width().to_string();
                    self.height = crop.height().to_string();
                }
            } else {
                self.drag_m = false;
                self.drag_l = false;
                self.drag_r = false;
                self.drag_t = false;
                self.drag_b = false;
                self.dragging = false;
            }
        }
        self.drag_m || self.drag_l || self.drag_r || self.drag_t || self.drag_b
    }
}
