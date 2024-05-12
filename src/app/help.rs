use cgmath::{EuclideanSpace, Point2};
use egui::RichText;

use super::App;
use crate::util::p2;
impl App {
    pub fn help_ui(&mut self, ctx: &egui::Context) {
        if self.help_visible {
            let mut open = true;
            egui::Window::new("Help")
                .id(egui::Id::new("help window"))
                .collapsible(false)
                .resizable(false)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(p2(Point2::from_vec(self.size / 2.0)))
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("help grid")
                        .striped(true)
                        .min_col_width(180.0)
                        .show(ui, |ui| {
                            const HELP: &[(&str, &str)] = &[
                                ("Open image", "Ctrl + O"),
                                ("Save as", "Ctrl + S"),
                                ("Reload image", "F5"),
                                ("Close image", "Ctrl + W"),
                                ("Quit", "Ctrl + Q"),
                                ("New window", "Ctrl + N"),
                                ("Undo", "Ctrl + Z"),
                                ("Redo", "Ctrl + Y"),
                                ("Copy", "Ctrl + C"),
                                ("Paste", "Ctrl + V"),
                                ("Resize", "Ctrl + R"),
                                ("Rotate left", "Q"),
                                ("Rotate right", "E"),
                                ("Zoom in", "- or Mousewheel up"),
                                ("Zoom out", "+ or Mousewheel down"),
                                ("Best fit", "Ctrl + B"),
                                ("Largest fit", "Ctrl + L"),
                                ("Crop", "Ctrl + X"),
                                ("F11 or F", "Fullscreen"),
                                ("Delete image", "Delete"),
                                ("100% - 900% Zoom", "Ctrl + 1 - 9"),
                                ("Previous image", "A or L or Left Arrow"),
                                ("Next image", "D or H or Right Arrow"),
                            ];

                            ui.label(RichText::new("Action").strong());
                            ui.label(RichText::new("Hotkey").strong());
                            ui.end_row();
                            for (action, hotkey) in HELP {
                                ui.label(*action);
                                ui.label(*hotkey);
                                ui.end_row();
                            }
                        });
                });
            self.help_visible = open;
        }
    }
}
