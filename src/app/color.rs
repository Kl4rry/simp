use egui::{Button, Slider};

use super::{op_queue::Op, App};

impl App {
    pub fn color_ui(&mut self, ctx: &egui::Context) {
        if self.color_visible && self.image_view.is_some() {
            let mut open = true;
            let mut closed = false;
            egui::Window::new("Color")
                .id(egui::Id::new("color window"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("color grid").show(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            ui.label("Hue: ");
                        });
                        ui.add(Slider::new(
                            &mut self.image_view.as_mut().unwrap().hue,
                            0.0..=180.0,
                        ));
                        ui.end_row();
                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            ui.label("Contrast: ");
                        });
                        ui.add(Slider::new(
                            &mut self.image_view.as_mut().unwrap().contrast,
                            -100.0..=100.0,
                        ));
                        ui.end_row();
                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            ui.label("Lightness: ");
                        });
                        ui.add(Slider::new(
                            &mut self.image_view.as_mut().unwrap().lightness,
                            -100.0..=100.0,
                        ));
                        ui.end_row();
                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            ui.label("Saturation: ");
                        });
                        ui.add(Slider::new(
                            &mut self.image_view.as_mut().unwrap().saturation,
                            -100.0..=100.0,
                        ));
                        ui.end_row();
                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            ui.label("Grayscale: ");
                        });
                        ui.checkbox(&mut self.image_view.as_mut().unwrap().grayscale, "");
                        ui.end_row();
                        ui.with_layout(egui::Layout::right_to_left(), |ui| {
                            ui.label("Invert: ");
                        });
                        ui.checkbox(&mut self.image_view.as_mut().unwrap().invert, "");
                        ui.end_row();
                        ui.end_row();

                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::Center),
                            |ui| {
                                if ui
                                    .add_enabled(self.view_available(), Button::new("Cancel"))
                                    .clicked()
                                {
                                    closed = true;
                                }
                            },
                        );

                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::Center),
                            |ui| {
                                if ui
                                    .add_enabled(self.view_available(), Button::new("Apply"))
                                    .clicked()
                                {
                                    let view = self.image_view.as_ref().unwrap();
                                    let hue = view.hue;
                                    let saturation = view.saturation;
                                    let contrast = view.contrast;
                                    let lightness = view.lightness;
                                    let grayscale = view.grayscale;
                                    let invert = view.invert;
                                    self.queue(Op::Color {
                                        hue,
                                        saturation,
                                        contrast,
                                        lightness,
                                        grayscale,
                                        invert,
                                    });
                                    closed = true;
                                }
                            },
                        );
                    });
                });
            self.color_visible = open && !closed;
            if !self.color_visible {
                if let Some(view) = self.image_view.as_mut() {
                    view.hue = 0.0;
                    view.contrast = 0.0;
                    view.saturation = 0.0;
                    view.lightness = 0.0;
                    view.grayscale = false;
                    view.invert = false;
                }
            }
        }
    }
}
