use std::thread;

use egui::{menu, Button, TopBottomPanel};
use glium::Display;

use super::{delete, load_image, new_window, op_queue::Op, save_image, App};

impl App {
    pub fn menu_bar(&mut self, display: &Display, ctx: &egui::Context) {
        TopBottomPanel::top("top").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu_button(ui, "File", |ui| {
                    if ui.button("Open").clicked() {
                        load_image::open(self.proxy.clone(), display);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Save as"))
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

                    if ui.button("New Window").clicked() {
                        new_window();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Refresh"))
                        .clicked()
                    {
                        if let Some(ref path) = self.image_view.as_ref().unwrap().path {
                            let buf = path.to_path_buf();
                            self.queue(Op::LoadPath(buf, false));
                        }
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Exit").clicked() {
                        self.exit = true;
                    }
                });

                menu::menu_button(ui, "Edit", |ui| {
                    if ui
                        .add_enabled(self.view_available(), Button::new("Undo"))
                        .clicked()
                    {
                        self.queue(Op::Undo);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.view_available(), Button::new("Redo"))
                        .clicked()
                    {
                        self.queue(Op::Redo);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.view_available(), Button::new("Copy"))
                        .clicked()
                    {
                        self.queue(Op::Copy);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(!self.op_queue.working(), Button::new("Paste"))
                        .clicked()
                    {
                        self.queue(Op::Paste);
                        ui.close_menu();
                    }
                });

                menu::menu_button(ui, "Image", |ui| {
                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Color"))
                        .clicked()
                    {
                        self.color_visible = true;
                        ui.close_menu();
                    }

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
                            Button::new("Metadata"),
                        )
                        .clicked()
                    {
                        self.metadata_visible = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.view_available(), Button::new("Rotate Left"))
                        .clicked()
                    {
                        self.queue(Op::Rotate(-1));
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.view_available(), Button::new("Rotate Right"))
                        .clicked()
                    {
                        self.queue(Op::Rotate(1));
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.view_available(), Button::new("Flip Horizontal"))
                        .clicked()
                    {
                        self.queue(Op::FlipHorizontal);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.view_available(), Button::new("Flip Vertical"))
                        .clicked()
                    {
                        self.queue(Op::FlipVertical);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Zoom in"))
                        .clicked()
                    {
                        self.zoom(1.0, self.size / 2.0);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Zoom out"))
                        .clicked()
                    {
                        self.zoom(-1.0, self.size / 2.0);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Best fit"))
                        .clicked()
                    {
                        self.best_fit();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Largest fit"))
                        .clicked()
                    {
                        self.largest_fit();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.view_available(), Button::new("Crop"))
                        .clicked()
                    {
                        self.image_view.as_mut().unwrap().start_crop();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Resize"))
                        .clicked()
                    {
                        self.resize.visible = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Delete"))
                        .clicked()
                    {
                        if let Some(ref view) = self.image_view {
                            if let Some(ref path) = view.path {
                                delete(path, self.proxy.clone());
                            }
                        }
                        ui.close_menu();
                    }
                });

                menu::menu_button(ui, "Help", |ui| {
                    if ui.button("Repository").clicked() {
                        webbrowser::open("https://github.com/Kl4rry/simp").unwrap();
                        ui.close_menu();
                    }

                    if ui.button("Report Bug").clicked() {
                        webbrowser::open("https://github.com/Kl4rry/simp/issues").unwrap();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Help").clicked() {
                        self.help_visible = true;
                    }

                    if ui.button("About").clicked() {
                        let about = format!(
                            "{}\n{}\n{}\n{}",
                            env!("CARGO_PKG_NAME"),
                            env!("CARGO_PKG_DESCRIPTION"),
                            &format!("Version: {}", env!("CARGO_PKG_VERSION")),
                            &format!("Commit: {}", env!("GIT_HASH")),
                        );
                        thread::spawn(move || {
                            msgbox::create("About", &about, msgbox::IconType::Info).unwrap()
                        });
                        ui.close_menu();
                    }
                });
            })
        });
    }
}
