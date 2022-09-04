use std::thread;

use egui::{menu, TopBottomPanel};
use glium::Display;

use super::{delete, load_image, new_window, op_queue::Op, save_image, App};
use crate::util::UserEvent;

mod menu_button;
use menu_button::MenuButton;

impl App {
    pub fn menu_bar(&mut self, display: &Display, ctx: &egui::Context) {
        TopBottomPanel::top("top").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu_button(ui, "File", |ui| {
                    if ui.add(MenuButton::new("Open").tip("Ctrl + O")).clicked() {
                        load_image::open(self.proxy.clone(), display, false);
                        ui.close_menu();
                    }

                    if ui.add(MenuButton::new("Open folder")).clicked() {
                        load_image::open(self.proxy.clone(), display, true);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Save as").tip("Ctrl + S"),
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
                        .add(MenuButton::new("New Window").tip("Ctrl + N"))
                        .clicked()
                    {
                        new_window();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Reload").tip("F5"),
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

                    if ui.add(MenuButton::new("Exit").tip("Ctrl + W")).clicked() {
                        let _ = self.proxy.send_event(UserEvent::Exit);
                        ui.close_menu();
                    }
                });

                menu::menu_button(ui, "Edit", |ui| {
                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Undo").tip("Ctrl + Z"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Undo);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Redo").tip("Ctrl + Y"),
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
                            MenuButton::new("Copy").tip("Ctrl + C"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Copy);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Paste").tip("Ctrl + V"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Paste);
                        ui.close_menu();
                    }
                });

                menu::menu_button(ui, "Image", |ui| {
                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Rotate Left").tip("Q"),
                        )
                        .clicked()
                    {
                        self.queue(Op::Rotate(-1));
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Rotate Right").tip("E"),
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
                            MenuButton::new("Flip Horizontal"),
                        )
                        .clicked()
                    {
                        self.queue(Op::FlipHorizontal);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.view_available(), MenuButton::new("Flip Vertical"))
                        .clicked()
                    {
                        self.queue(Op::FlipVertical);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Zoom in").tip("+"),
                        )
                        .clicked()
                    {
                        self.zoom(1.0, self.size / 2.0);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Zoom out").tip("-"),
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
                            MenuButton::new("Best fit").tip("B"),
                        )
                        .clicked()
                    {
                        self.best_fit();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Largest fit").tip("L"),
                        )
                        .clicked()
                    {
                        self.largest_fit();
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.image_view.is_some(), MenuButton::new("Color"))
                        .clicked()
                    {
                        self.color_visible = true;
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.view_available(),
                            MenuButton::new("Crop").tip("Ctrl + X"),
                        )
                        .clicked()
                    {
                        self.image_view.as_mut().unwrap().start_crop();
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.image_view.is_some(),
                            MenuButton::new("Resize").tip("Ctrl + R"),
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
                            MenuButton::new("Metadata"),
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
                            MenuButton::new("Delete").tip("Delete"),
                        )
                        .clicked()
                    {
                        if let Some(ref view) = self.image_view {
                            if let Some(ref path) = view.path {
                                delete(path.clone(), self.proxy.clone(), display);
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
