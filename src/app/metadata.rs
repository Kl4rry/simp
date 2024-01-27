use egui::{RichText, ScrollArea};

use super::App;
impl App {
    pub fn metadata_ui(&mut self, ctx: &egui::Context) {
        if self.metadata_visible && self.image_view.is_some() {
            let mut open = true;
            egui::Window::new("Metadata")
                .id(egui::Id::new("metadata window"))
                .collapsible(false)
                .resizable(true)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(self.size / 2.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        let guard = self.image_view.as_ref().unwrap().image_data.read().unwrap();
                        if guard.metadata.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label(RichText::new("Could not find any metadata.").size(20.0));
                            });
                        } else {
                            egui::Grid::new("metadata grid")
                                .striped(true)
                                .min_col_width(180.0)
                                .show(ui, |ui| {
                                    for (label, data) in &guard.metadata {
                                        ui.label(label);
                                        ui.label(data);
                                        ui.end_row();
                                    }
                                });
                        }
                    })
                });
            self.metadata_visible = open;
        }
    }
}
