use egui::Slider;

use super::App;

impl App {
    pub fn color_ui(&mut self, ctx: &egui::Context) {
        if self.color_visible && self.image_view.is_some() {
            let mut open = true;
            egui::Window::new("Color")
                .id(egui::Id::new("color window"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("color grid").striped(true).show(ui, |ui| {
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
                    });
                });
            self.color_visible = open;
        }
    }
}
