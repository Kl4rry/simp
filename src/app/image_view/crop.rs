use bytemuck::Zeroable;
use cgmath::{EuclideanSpace, Point2, Vector2};
use egui::{CursorIcon, PointerButton};
use num_traits::Zero;

use super::crop_renderer;
use crate::{
    rect::Rect,
    util::{p2, v2},
};

pub struct Crop {
    pub rect: Option<Rect>,
    pub x: String,
    pub y: String,
    pub width: String,
    pub height: String,
    drag_rem: Vector2<f32>,
    dragging: bool,
}

impl Default for Crop {
    fn default() -> Self {
        Self::new()
    }
}

impl Crop {
    pub fn new() -> Self {
        Self {
            rect: None,
            x: String::new(),
            y: String::new(),
            width: String::new(),
            height: String::new(),
            drag_rem: Vector2::zero(),
            dragging: false,
        }
    }

    pub fn cropping(&self) -> bool {
        self.rect.is_some()
    }

    pub fn get_uniform(
        &self,
        window_size: Vector2<f32>,
        position: Vector2<f32>,
        image_size: Vector2<f32>,
        scale: f32,
    ) -> Option<crop_renderer::Uniform> {
        let rect = self.rect.as_ref()?;
        let start = position - (image_size / 2.0) * scale + rect.position * scale;
        let end = position - (image_size / 2.0) * scale + (rect.position + rect.size) * scale;
        Some(crop_renderer::Uniform {
            start,
            end,
            size: window_size,
        })
    }

    pub fn handle_drag(
        &mut self,
        ui: &egui::Ui,
        position: Vector2<f32>,
        image_size: Vector2<f32>,
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

            let mut drag_m = false;
            let mut drag_l = false;
            let mut drag_r = false;
            let mut drag_t = false;
            let mut drag_b = false;

            const HITBOX_SIZE: f32 = 10.0;
            const HITBOX_SIZE2: f32 = HITBOX_SIZE * 2.0;

            // TODO fix this hideous hitmax math code. like why does it mix adding vectors with just add floats!?!?

            let top = {
                let start = pos - Vector2::new(HITBOX_SIZE - HITBOX_SIZE2, HITBOX_SIZE);
                let size = Vector2::new(size.x - HITBOX_SIZE2, HITBOX_SIZE2);
                egui::Rect::from_min_size(p2(Point2::from_vec(start)).into(), v2(size).into())
            };

            let bottom = {
                let start = Vector2::new(
                    pos.x - HITBOX_SIZE + HITBOX_SIZE2,
                    pos.y + size.y - HITBOX_SIZE,
                );
                let size = Vector2::new(size.x - HITBOX_SIZE2, HITBOX_SIZE2);
                egui::Rect::from_min_size(p2(Point2::from_vec(start)).into(), v2(size).into())
            };

            let left = {
                let start = pos - Vector2::new(HITBOX_SIZE, HITBOX_SIZE - HITBOX_SIZE2);
                let size = Vector2::new(HITBOX_SIZE2, size.y - HITBOX_SIZE2);
                egui::Rect::from_min_size(p2(Point2::from_vec(start)).into(), v2(size).into())
            };

            let right = {
                let start = Vector2::new(pos.x + size.x - HITBOX_SIZE, pos.y + HITBOX_SIZE);
                let size = Vector2::new(HITBOX_SIZE2, size.y - HITBOX_SIZE2);
                egui::Rect::from_min_size(p2(Point2::from_vec(start)).into(), v2(size).into())
            };

            let top_left = {
                let start = pos - Vector2::new(HITBOX_SIZE, HITBOX_SIZE);
                let size = Vector2::new(HITBOX_SIZE2, HITBOX_SIZE2);
                egui::Rect::from_min_size(p2(Point2::from_vec(start)).into(), v2(size).into())
            };

            let top_right = {
                let start = pos - Vector2::new(HITBOX_SIZE - size.x, HITBOX_SIZE);
                let size = Vector2::new(HITBOX_SIZE2, HITBOX_SIZE2);
                egui::Rect::from_min_size(p2(Point2::from_vec(start)).into(), v2(size).into())
            };

            let bottom_left = {
                let start = pos - Vector2::new(HITBOX_SIZE, HITBOX_SIZE - size.y);
                let size = Vector2::new(HITBOX_SIZE2, HITBOX_SIZE2);
                egui::Rect::from_min_size(p2(Point2::from_vec(start)).into(), v2(size).into())
            };

            let bottom_right = {
                let start = pos - Vector2::new(HITBOX_SIZE - size.x, HITBOX_SIZE - size.y);
                let size = Vector2::new(HITBOX_SIZE2, HITBOX_SIZE2);
                egui::Rect::from_min_size(p2(Point2::from_vec(start)).into(), v2(size).into())
            };

            let sense = egui::Sense::hover().union(egui::Sense::drag());

            let mut drag_delta = egui::Vec2::zeroed();

            {
                let res = ui.interact(top, "crop top".into(), sense);
                if res.hovered() {
                    t = true;
                }
                if res.dragged_by(PointerButton::Primary) {
                    drag_t = true;
                    drag_delta = res.drag_delta();
                }

                let res = ui.interact(bottom, "crop bottom".into(), sense);
                if res.hovered() {
                    b = true;
                }
                if res.dragged_by(PointerButton::Primary) {
                    drag_b = true;
                    drag_delta = res.drag_delta();
                }

                let res = ui.interact(left, "crop left".into(), sense);
                if res.hovered() {
                    l = true;
                }
                if res.dragged_by(PointerButton::Primary) {
                    drag_l = true;
                    drag_delta = res.drag_delta();
                }

                let res = ui.interact(right, "crop right".into(), sense);
                if res.hovered() {
                    r = true;
                }
                if res.dragged_by(PointerButton::Primary) {
                    drag_r = true;
                    drag_delta = res.drag_delta();
                }

                let res = ui.interact(top_left, "crop top_left".into(), sense);
                if res.hovered() {
                    l = true;
                    t = true;
                }
                if res.dragged_by(PointerButton::Primary) {
                    drag_l = true;
                    drag_t = true;
                    drag_delta = res.drag_delta();
                }

                let res = ui.interact(top_right, "crop top_right".into(), sense);
                if res.hovered() {
                    r = true;
                    t = true;
                }
                if res.dragged_by(PointerButton::Primary) {
                    drag_r = true;
                    drag_t = true;
                    drag_delta = res.drag_delta();
                }

                let res = ui.interact(bottom_left, "crop bottom_left".into(), sense);
                if res.hovered() {
                    l = true;
                    b = true;
                }
                if res.dragged_by(PointerButton::Primary) {
                    drag_l = true;
                    drag_b = true;
                    drag_delta = res.drag_delta();
                }

                let res = ui.interact(bottom_right, "crop bottom_right".into(), sense);
                if res.hovered() {
                    r = true;
                    b = true;
                }
                if res.dragged_by(PointerButton::Primary) {
                    drag_r = true;
                    drag_b = true;
                    drag_delta = res.drag_delta();
                }

                let middle =
                    egui::Rect::from_min_size(p2(Point2::from_vec(pos)).into(), v2(size).into());
                let res = ui.interact(middle.shrink(HITBOX_SIZE), "crop middle".into(), sense);
                if res.hovered() {
                    m = true;
                }
                if res.dragged_by(PointerButton::Primary) {
                    drag_m = true;
                    drag_delta = res.drag_delta();
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
            } else {
                ui.ctx().set_cursor_icon(CursorIcon::Default);
            }

            if !self.dragging && (drag_m || drag_l || drag_r || drag_t || drag_b) {
                self.dragging = true;
            } else {
                let mut delta = Vector2::from(v2(drag_delta)) / scale + self.drag_rem;

                if delta.x.abs() > 1.0 {
                    self.drag_rem.x = delta.x % 1.0;
                    delta.x = delta.x - delta.x % 1.0;
                } else {
                    self.drag_rem.x = delta.x;
                    delta.x = 0.0;
                }

                if delta.y.abs() > 1.0 {
                    self.drag_rem.y = delta.y % 1.0;
                    delta.y = delta.y - delta.y % 1.0;
                } else {
                    self.drag_rem.y = delta.y;
                    delta.y = 0.0;
                }

                let (new_pos, new_size) = if drag_t && drag_l {
                    let new_pos = crop.position + delta;
                    let new_size = crop.size - delta;
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeNorthWest);
                    (new_pos, new_size)
                } else if drag_t && drag_r {
                    let new_pos = Vector2::new(crop.position.x, crop.position.y + delta.y);
                    let new_size = Vector2::new(crop.size.x + delta.x, crop.size.y - delta.y);
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeNorthEast);
                    (new_pos, new_size)
                } else if drag_b && drag_l {
                    let new_pos = Vector2::new(crop.position.x + delta.x, crop.position.y);
                    let new_size = Vector2::new(crop.size.x - delta.x, crop.size.y + delta.y);
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeSouthWest);
                    (new_pos, new_size)
                } else if drag_b && drag_r {
                    let new_pos = crop.position;
                    let new_size = crop.size + delta;
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeSouthEast);
                    (new_pos, new_size)
                } else if drag_t {
                    let new_pos = Vector2::new(crop.position.x, crop.position.y + delta.y);
                    let new_size = Vector2::new(crop.size.x, crop.size.y - delta.y);
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                    (new_pos, new_size)
                } else if drag_b {
                    let new_pos = Vector2::new(crop.position.x, crop.position.y);
                    let new_size = Vector2::new(crop.size.x, crop.size.y + delta.y);
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                    (new_pos, new_size)
                } else if drag_l {
                    let new_pos = Vector2::new(crop.position.x + delta.x, crop.position.y);
                    let new_size = Vector2::new(crop.size.x - delta.x, crop.size.y);
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                    (new_pos, new_size)
                } else if drag_r {
                    let new_pos = Vector2::new(crop.position.x, crop.position.y);
                    let new_size = Vector2::new(crop.size.x + delta.x, crop.size.y);
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                    (new_pos, new_size)
                } else if drag_m {
                    crop.position += delta;
                    let max = (image_size - crop.size).map(|v| v.max(0.0));
                    crop.position.x = crop.position.x.clamp(0.0, max.x);
                    crop.position.y = crop.position.y.clamp(0.0, max.y);
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    (crop.position, crop.size)
                } else {
                    (crop.position, crop.size)
                };

                if new_size.x >= 1.0 && new_pos.x >= 0.0 {
                    crop.position.x = new_pos.x;
                    crop.size.x = new_size.x;
                }
                if new_size.y >= 1.0 && new_pos.y >= 0.0 {
                    crop.position.y = new_pos.y;
                    crop.size.y = new_size.y;
                }

                self.x = crop.x().to_string();
                self.y = crop.y().to_string();
                self.width = crop.width().to_string();
                self.height = crop.height().to_string();
            }
            drag_m || drag_l || drag_r || drag_t || drag_b
        } else {
            self.dragging = false;
            false
        }
    }
}
