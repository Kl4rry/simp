use std::sync::Mutex;

use cgmath::{EuclideanSpace, Point2};
use serde::{Deserialize, Serialize};

use super::App;
use crate::util::p2;

pub static PREFERENCES: Mutex<Preferences> = Mutex::new(Preferences::new());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub open_in_fullscreen: bool,
    pub auto_center: bool,
    pub min_svg_size: u32,
    pub zoom_speed: f32,
    pub jpeg_quality: u8,
    pub webp_lossy: bool,
    pub webp_quality: f32,
    pub jxl_lossy: bool,
    pub jxl_quality: f32,
}

impl Preferences {
    const fn new() -> Self {
        Self {
            open_in_fullscreen: false,
            auto_center: true,
            min_svg_size: 1000,
            zoom_speed: 1.0,
            jpeg_quality: 80,
            webp_lossy: false,
            webp_quality: 80.0,
            jxl_lossy: true,
            jxl_quality: 1.0,
        }
    }

    pub fn clamp(&mut self) {
        self.jpeg_quality = self.jpeg_quality.clamp(1, 100);
        self.webp_quality = self.webp_quality.clamp(0.0, 100.0);
        self.jxl_quality = self.jxl_quality.clamp(0.0, 15.0);
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
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(p2(Point2::from_vec(self.size / 2.0)))
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("preferences grid").show(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("Open in fullscreen: ");
                        });
                        ui.add(egui::Checkbox::new(&mut preferences.open_in_fullscreen, ""));
                        ui.end_row();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("Automatically center image: ");
                        });
                        ui.add(egui::Checkbox::new(&mut preferences.auto_center, ""));
                        ui.end_row();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("Zoom Speed: ");
                        });
                        ui.add(egui::Slider::new(&mut preferences.zoom_speed, 0.001..=10.0));
                        ui.end_row();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("Minimum svg size: ");
                        });
                        ui.add(egui::Slider::new(&mut preferences.min_svg_size, 0..=10000));
                        ui.end_row();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("JPEG quality: ");
                        });
                        ui.add(egui::Slider::new(&mut preferences.jpeg_quality, 1..=100));
                        ui.end_row();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("WebP lossy compression: ");
                        });
                        ui.add(egui::Checkbox::new(&mut preferences.webp_lossy, ""));
                        ui.end_row();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("WebP quality: ");
                        });
                        ui.add(egui::Slider::new(
                            &mut preferences.webp_quality,
                            0.0..=100.0,
                        ));
                        ui.end_row();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("JPEG XL lossy compression: ");
                        });
                        ui.add(egui::Checkbox::new(&mut preferences.jxl_lossy, ""));
                        ui.end_row();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("JPEG XL quality: ");
                        });
                        ui.add(egui::Slider::new(&mut preferences.jxl_quality, 0.0..=15.0));

                        ui.end_row();
                        ui.end_row();
                        if ui.button("Reset to default").clicked() {
                            preferences = Default::default();
                        }
                    });
                });

            *PREFERENCES.lock().unwrap() = preferences;

            self.preferences_visible = open;
        }
    }
}
