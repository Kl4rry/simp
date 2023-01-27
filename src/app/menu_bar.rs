use std::thread;

use egui::{menu, TopBottomPanel};
use glium::Display;

use super::{load_image, new_window, op_queue::Op, save_image, App};
use crate::util::UserEvent;

impl App {
    pub fn menu_bar(&mut self, display: &Display, ctx: &egui::Context) {
        TopBottomPanel::top("top").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu_button(ui, "File", |ui| {
                    if ui
                        .add(egui::Button::new("Open").shortcut_text("Ctrl + O"))
                        .clicked()
                    {
                        load_image::open(self.proxy.clone(), display, false);
                        ui.close_menu();
                    }

                    if ui.add(egui::Button::new("Open folder")).clicked() {
                        load_image::open(self.proxy.clone(), display, true);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Save as").shortcut_text("Ctrl + S"),
                        )
                        .clicked()
                    {
                        save_image::open(
                            self.current_filename.clone(),
                            self.proxy.clone(),
                            display,
                        );
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add(egui::Button::new("New Window").shortcut_text("Ctrl + N"))
                        .clicked()
                    {
                        new_window();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Reload").shortcut_text("F5"),
                        )
                        .clicked()
                    {
                        save_image::open(
                            self.current_filename.clone(),
                            self.proxy.clone(),
                            display,
                        );
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add(egui::Button::new("Exit").shortcut_text("Ctrl + W"))
                        .clicked()
                    {
                        let _ = self.proxy.send_event(UserEvent::Exit);
                        ui.close_menu();
                    }
                });

                menu::menu_button(ui, "Edit", |ui| {
                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Undo").shortcut_text("Ctrl + Z"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Undo);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Redo").shortcut_text("Ctrl + Y"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Redo);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Copy").shortcut_text("Ctrl + C"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Copy);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Paste").shortcut_text("Ctrl + V"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Paste);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.add(egui::Button::new("Preferences")).clicked() {
                        self.preferences_visible = true;
                        ui.close_menu();
                    }
                });

                menu::menu_button(ui, "Image", |ui| {
                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Rotate Left").shortcut_text("Q"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Rotate(-1));
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Rotate Right").shortcut_text("E"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Rotate(1));
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Flip Horizontal"),
                        )
                        .clicked()
                    {
                        self.queue(Op::FlipHorizontal);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.view_available(), egui::Button::new("Flip Vertical"))
                        .clicked()
                    {
                        self.queue(Op::FlipVertical);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Zoom in").shortcut_text("+"),
                        )
                        .clicked()
                    {
                        self.zoom(1.0, self.size / 2.0);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Zoom out").shortcut_text("-"),
                        )
                        .clicked()
                    {
                        self.zoom(-1.0, self.size / 2.0);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Best fit").shortcut_text("B"),
                        )
                        .clicked()
                    {
                        self.best_fit();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Largest fit").shortcut_text("L"),
                        )
                        .clicked()
                    {
                        self.largest_fit();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.image_view.is_some(), egui::Button::new("Color"))
                        .clicked()
                    {
                        self.color_visible = true;
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.view_available(),
                            egui::Button::new("Crop").shortcut_text("Ctrl + X"),
                        )
                        .clicked()
                    {
                        self.image_view.as_mut().unwrap().start_crop();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Resize").shortcut_text("Ctrl + R"),
                        )
                        .clicked()
                    {
                        self.resize.visible = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(
                            self.image_view.is_some()
                                && !self
                                    .image_view
                                    .as_ref()
                                    .unwrap()
                                    .image_data
                                    .read()
                                    .unwrap()
                                    .metadata
                                    .is_empty(),
                            egui::Button::new("Metadata"),
                        )
                        .clicked()
                    {
                        self.metadata_visible = true;
                        ui.close_menu();
                    }

                    #[cfg(feature = "trash")]
                    {
                        ui.separator();
                        if ui
                            .add_enabled(
                                self.image_view.is_some(),
                                egui::Button::new("Delete").shortcut_text("Delete"),
                            )
                            .clicked()
                        {
                            if let Some(ref view) = self.image_view {
                                if let Some(ref path) = view.path {
                                    super::delete(path.clone(), self.proxy.clone(), display);
                                }
                            }
                            ui.close_menu();
                        }
                    }
                });

                menu::menu_button(ui, "Help", |ui| {
                    if ui.add(egui::Button::new("Repository")).clicked() {
                        webbrowser::open("https://github.com/Kl4rry/simp").unwrap();
                        ui.close_menu();
                    }

                    if ui.add(egui::Button::new("Report Bug")).clicked() {
                        webbrowser::open("https://github.com/Kl4rry/simp/issues").unwrap();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add(egui::Button::new("Help").shortcut_text("Ctrl + H"))
                        .clicked()
                    {
                        self.help_visible = true;
                        ui.close_menu();
                    }

                    if ui.add(egui::Button::new("About")).clicked() {
                        let about = format!(
                            "{}\n{}\n{}\n{}",
                            env!("CARGO_PKG_NAME"),
                            env!("CARGO_PKG_DESCRIPTION"),
                            &format!("Version: {}", env!("CARGO_PKG_VERSION")),
                            &format!("Commit: {}", env!("GIT_HASH")),
                        );

                        let dialog = rfd::MessageDialog::new()
                            .set_parent(display.gl_window().window())
                            .set_level(rfd::MessageLevel::Info)
                            .set_title("About")
                            .set_description(&about)
                            .set_buttons(rfd::MessageButtons::Ok);

                        thread::spawn(move || dialog.show());
                        ui.close_menu();
                    }
                });
            })
        });
    }
}
