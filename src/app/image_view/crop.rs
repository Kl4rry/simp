use egui::{CursorIcon, PointerButton};
use glium::{
    implement_vertex, index::PrimitiveType, uniform, Blend, Display, DrawParameters, IndexBuffer,
    Program, Surface, VertexBuffer,
};
use once_cell::sync::OnceCell;

use crate::{rect::Rect, vec2::Vec2};

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
}

impl Vertex {
    pub fn new(x: f32, y: f32) -> Self {
        Self { position: [x, y] }
    }
}

implement_vertex!(Vertex, position);

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
    vertices: VertexBuffer<Vertex>,
    indices: IndexBuffer<u8>,
}

impl Crop {
    pub fn new(display: &Display) -> Self {
        let shape = [
            Vertex::new(-1.0, 1.0),
            Vertex::new(-1.0, -1.0),
            Vertex::new(1.0, 1.0),
            Vertex::new(1.0, -1.0),
        ];
        let index_buffer = &[0, 1, 2, 2, 1, 3];

        let vertices = VertexBuffer::new(display, &shape).unwrap();
        let indices =
            IndexBuffer::new(display, PrimitiveType::TrianglesList, index_buffer).unwrap();

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
            vertices,
            indices,
        }
    }

    pub fn cropping(&self) -> bool {
        self.rect.is_some()
    }

    pub fn render(
        &self,
        target: &mut glium::Frame,
        display: &Display,
        window_size: Vec2<f32>,
        position: Vec2<f32>,
        image_size: Vec2<f32>,
        scale: f32,
    ) {
        if let Some(ref rect) = self.rect {
            let start = position - (image_size / 2.0) * scale + rect.position * scale;
            let end = position - (image_size / 2.0) * scale + (rect.position + rect.size) * scale;

            thread_local! {
                pub static PROGRAM: OnceCell<Program> = OnceCell::new();
            }

            PROGRAM.with(|f| {
                let shader = f.get_or_init(|| {
                    Program::from_source(
                        display,
                        include_str!("../../shader/crop.vert"),
                        include_str!("../../shader/crop.frag"),
                        None,
                    )
                    .unwrap()
                });

                target
                    .draw(
                        &self.vertices,
                        &self.indices,
                        shader,
                        &uniform! { start: start, end: end, size: *window_size },
                        &DrawParameters {
                            blend: Blend::alpha_blending(),
                            ..DrawParameters::default()
                        },
                    )
                    .unwrap();
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
                ui.ctx().output().cursor_icon = CursorIcon::ResizeNorthWest;
            } else if t && r {
                ui.ctx().output().cursor_icon = CursorIcon::ResizeNorthEast;
            } else if b && l {
                ui.ctx().output().cursor_icon = CursorIcon::ResizeSouthWest;
            } else if b && r {
                ui.ctx().output().cursor_icon = CursorIcon::ResizeSouthEast;
            } else if t || b {
                ui.ctx().output().cursor_icon = CursorIcon::ResizeVertical;
            } else if l || r {
                ui.ctx().output().cursor_icon = CursorIcon::ResizeHorizontal;
            } else if m {
                ui.ctx().output().cursor_icon = CursorIcon::Grabbing;
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
                        ui.ctx().output().cursor_icon = CursorIcon::ResizeNorthWest;
                        (new_pos, new_size)
                    } else if self.drag_t && self.drag_r {
                        let new_pos = Vec2::new(crop.position.x(), crop.position.y() + delta.y());
                        let new_size =
                            Vec2::new(crop.size.x() + delta.x(), crop.size.y() - delta.y());
                        ui.ctx().output().cursor_icon = CursorIcon::ResizeNorthEast;
                        (new_pos, new_size)
                    } else if self.drag_b && self.drag_l {
                        let new_pos = Vec2::new(crop.position.x() + delta.x(), crop.position.y());
                        let new_size =
                            Vec2::new(crop.size.x() - delta.x(), crop.size.y() + delta.y());
                        ui.ctx().output().cursor_icon = CursorIcon::ResizeSouthWest;
                        (new_pos, new_size)
                    } else if self.drag_b && self.drag_r {
                        let new_pos = crop.position;
                        let new_size = crop.size + delta;
                        ui.ctx().output().cursor_icon = CursorIcon::ResizeSouthEast;
                        (new_pos, new_size)
                    } else if self.drag_t {
                        let new_pos = Vec2::new(crop.position.x(), crop.position.y() + delta.y());
                        let new_size = Vec2::new(crop.size.x(), crop.size.y() - delta.y());
                        ui.ctx().output().cursor_icon = CursorIcon::ResizeVertical;
                        (new_pos, new_size)
                    } else if self.drag_b {
                        let new_pos = Vec2::new(crop.position.x(), crop.position.y());
                        let new_size = Vec2::new(crop.size.x(), crop.size.y() + delta.y());
                        ui.ctx().output().cursor_icon = CursorIcon::ResizeVertical;
                        (new_pos, new_size)
                    } else if self.drag_l {
                        let new_pos = Vec2::new(crop.position.x() + delta.x(), crop.position.y());
                        let new_size = Vec2::new(crop.size.x() - delta.x(), crop.size.y());
                        ui.ctx().output().cursor_icon = CursorIcon::ResizeHorizontal;
                        (new_pos, new_size)
                    } else if self.drag_r {
                        let new_pos = Vec2::new(crop.position.x(), crop.position.y());
                        let new_size = Vec2::new(crop.size.x() + delta.x(), crop.size.y());
                        ui.ctx().output().cursor_icon = CursorIcon::ResizeHorizontal;
                        (new_pos, new_size)
                    } else if self.drag_m {
                        crop.position += delta;
                        let max = (image_size - crop.size).max(0.0, 0.0);
                        *crop.position.mut_x() = crop.position.x().clamp(0.0, max.x());
                        *crop.position.mut_y() = crop.position.y().clamp(0.0, max.y());
                        ui.ctx().output().cursor_icon = CursorIcon::Grabbing;
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
