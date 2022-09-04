use egui::RichText;

use super::App;
impl App {
    pub fn help_ui(&mut self, ctx: &egui::Context) {
        if self.help_visible {
            let mut open = true;
            egui::Window::new("Help")
                .id(egui::Id::new("help window"))
                .collapsible(false)
                .resizable(false)
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
                                ("Close image", "Ctrl + F4"),
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
                                ("Best fit", "B"),
                                ("Largest fit", "L"),
                                ("Crop", "Ctrl + X"),
                                ("F11 or F", "Fullscreen"),
                                ("Delete image", "Delete"),
                                ("1 - 9", "100% - 900% Zoom"),
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
