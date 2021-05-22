use super::vec2::*;
use imgui::*;

static SIZE: f32 = 20.0;

pub fn render_background(ui: &Ui, size: &Vec2<f32>) {
    let columns = (size.x() / SIZE).ceil() as i32;
    let rows = (size.y() / SIZE).ceil() as i32;

    for x in 0..columns {
        for y in 0..rows {
            let color = if (x % 2 + y % 2) % 2 == 0 {
                color::ImColor32::from_rgb(51, 56, 66)
            } else {
                color::ImColor32::from_rgb(67, 73, 86)
            };
            draw_rect(ui, Vec2::new((x * SIZE as i32) as f32, (y * SIZE as i32) as f32 + 50.0), color);
        }
    }
}

fn draw_rect(ui: &Ui, pos: Vec2<f32>, color: color::ImColor32) {
    let draw_list = ui.get_background_draw_list();
    draw_list.add_rect_filled_multicolor(*pos, *(pos + Vec2::new(SIZE, SIZE)), color, color, color, color);
}