use std::fs;

use egui::{menu, TopBottomPanel};

use super::{load_image, new_window, op_queue::Op, save_image, App};
use crate::{util::UserEvent, WgpuState};

impl App {
    pub fn menu_bar(&mut self, wgpu: &WgpuState, ctx: &egui::Context) {
        TopBottomPanel::top("top").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu_button(ui, "File", |ui| {
                    if ui
                        .add(egui::Button::new("Open").shortcut_text("Ctrl + O"))
                        .clicked()
                    {
                        load_image::open(self.proxy.clone(), wgpu, false);
                        ui.close_menu();
                    }

                    if ui.add(egui::Button::new("Open folder")).clicked() {
                        load_image::open(self.proxy.clone(), wgpu, true);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Save as").shortcut_text("Ctrl + S"),
                        )
                        .clicked()
                    {
                        save_image::open(self.current_filename.clone(), self.proxy.clone(), wgpu);
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
                        if let Some(image) = self.image_view.as_ref() {
                            if let Some(path) = &image.path {
                                let buf = path.to_path_buf();
                                self.queue(Op::LoadPath(buf, false));
                            }
                        }
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
                            egui::Button::new("Best fit").shortcut_text("Ctrl + B"),
                        )
                        .clicked()
                    {
                        self.best_fit();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            egui::Button::new("Largest fit").shortcut_text("Ctrl + L"),
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
                                super::delete(
                                    path.clone(),
                                    self.dialog_manager.get_proxy(),
                                    self.proxy.clone(),
                                );
                            }
                        }
                        ui.close_menu();
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

                    ui.separator();

                    if ui.add(egui::Button::new("Third-party Software")).clicked() {
                        if let Err(err) = open_licenes() {
                            let _ = self
                                .proxy
                                .send_event(UserEvent::ErrorMessage(err.to_string()));
                        }
                    }

                    if ui.add(egui::Button::new("About")).clicked() {
                        let info = wgpu.adapter.get_info();

                        let about = format!(
                            "{}\n{}\n{}\n{}\n{}",
                            env!("CARGO_PKG_NAME"),
                            env!("CARGO_PKG_DESCRIPTION"),
                            &format!("Version: {}", env!("CARGO_PKG_VERSION")),
                            &format!("Commit: {}", env!("GIT_HASH")),
                            &format!("GPU backend: {:?}", info.backend),
                        );

                        self.dialog_manager.get_proxy().spawn_dialog(
                            "About",
                            move |ui, enter| -> Option<()> {
                                ui.label(&about);

                                if *enter {
                                    *enter = false;
                                    return Some(());
                                }

                                ui.button("Ok").clicked().then_some(())
                            },
                        );

                        ui.close_menu();
                    }
                });
            })
        });
    }
}

fn open_licenes() -> Result<(), std::io::Error> {
    let licenses = include_str!(concat!(env!("OUT_DIR"), "/license.html"));
    let temp = std::env::temp_dir();
    let name = env!("CARGO_BIN_NAME");
    fs::create_dir_all(temp.join(name))?;
    let license_file = temp.join(name).join("license.html");
    fs::write(&license_file, licenses.as_bytes())?;
    webbrowser::open(&license_file.to_string_lossy())
}
