use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::App;

pub static PREFERENCES: Mutex<Preferences> = Mutex::new(Preferences::new());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub min_svg_size: u32,
    pub zoom_speed: f32,
}

impl Preferences {
    const fn new() -> Self {
        Self {
            min_svg_size: 1000,
            zoom_speed: 1.0,
        }
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn preferences_ui(&mut self, ctx: &egui::Context) {
        if self.preferences_visible {
            let mut preferences = PREFERENCES.lock().unwrap().clone();

            let mut open = true;
            egui::Window::new("Preferences")
                .id(egui::Id::new("preferences window"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("preferences grid").show(ui, |ui| {
                        ui.add(
                            egui::Slider::new(&mut preferences.zoom_speed, 0.001..=10.0)
                                .text("Zoom Speed"),
                        );
                        ui.end_row();
                        ui.add(
                            egui::Slider::new(&mut preferences.min_svg_size, 0..=10000)
                                .text("Minimum svg size"),
                        );
                    });
                });

            *PREFERENCES.lock().unwrap() = preferences;

            self.preferences_visible = open;
        }
    }
}
