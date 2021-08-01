use std::time::{Duration, Instant};

use imgui::*;

pub enum Action {
    Right,
    Left,
    None,
}

pub struct Arrows {
    left_hover_start: Instant,
    right_hover_start: Instant,
}

impl Arrows {
    pub fn new() -> Self {
        Self {
            left_hover_start: Instant::now(),
            right_hover_start: Instant::now(),
        }
    }

    pub fn build(&mut self, ui: &Ui<'_>) -> (Action, Option<Duration>) {
        let mut action = Action::None;
        let mut delay = None;
        let now = Instant::now();
        ui.same_line_with_spacing(0.0, 5.0);
        if ui.arrow_button(im_str!("Left"), Direction::Left) {
            action = Action::Left;
        }

        if ui.is_item_hovered() {
            if now.duration_since(self.left_hover_start) > Duration::from_millis(500) {
                ui.tooltip(|| {
                    ui.text(im_str!("Previous Image"));
                    ui.same_line_with_spacing(0.0, 10.0);
                    ui.text_colored([0.501, 0.501, 0.501, 1.0], im_str!("Left Arrow"));
                });
            } else {
                delay = Some(Duration::from_millis(16));
            }
        } else {
            self.left_hover_start = now;
        }

        ui.same_line_with_spacing(0.0, 10.0);
        if ui.arrow_button(im_str!("Right"), Direction::Right) {
            action = Action::Right;
        }

        if ui.is_item_hovered() {
            if now.duration_since(self.right_hover_start) > Duration::from_millis(500) {
                ui.tooltip(|| {
                    ui.text(im_str!("Next Image"));
                    ui.same_line_with_spacing(0.0, 10.0);
                    ui.text_colored([0.501, 0.501, 0.501, 1.0], im_str!("Right Arrow"));
                });
            } else {
                delay = Some(Duration::from_millis(16));
            }
        } else {
            self.right_hover_start = now;
        }
        (action, delay)
    }
}
