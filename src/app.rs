use std::{
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use egui::{Button, CursorIcon, RichText, Style, TopBottomPanel};
use glium::{
    backend::glutin::Display,
    glutin::{
        event::{ElementState, ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowEvent},
        event_loop::EventLoopProxy,
        window::Fullscreen,
    },
};
use image::{imageops::FilterType, DynamicImage};

use crate::{min, util::UserEvent, vec2::Vec2};

pub mod image_view;
use image_view::ImageView;

pub mod image_list;

mod clipboard;

mod color;
mod help;
mod menu_bar;
mod metadata;

pub mod op_queue;
use op_queue::{Op, OpQueue, Output};

pub mod load_image;

mod save_image;

mod undo_stack;

mod cache;

mod resize;
use resize::Resize;

use self::{op_queue::get_unsaved_changes_dialog, undo_stack::UndoFrame};

const TOP_BAR_SIZE: f32 = 26.0;
const BOTTOM_BAR_SIZE: f32 = 27.0;

pub struct App {
    exit: Arc<AtomicBool>,
    delay: Option<Duration>,
    pub image_view: Option<Box<ImageView>>,
    pub size: Vec2<f32>,
    fullscreen: bool,
    pub top_bar_size: f32,
    pub bottom_bar_size: f32,
    proxy: EventLoopProxy<UserEvent>,
    modifiers: ModifiersState,
    mouse_position: Vec2<f32>,
    current_filename: String,
    op_queue: OpQueue,
    resize: Resize,
    help_visible: bool,
    color_visible: bool,
    metadata_visible: bool,
}

impl App {
    pub fn view_available(&self) -> bool {
        !self.op_queue.working() && self.image_view.is_some()
    }

    pub fn handle_output(&mut self, display: &Display, output: Output) {
        let stack = self.op_queue.undo_stack_mut();
        match output {
            Output::ImageLoaded(image_data, path) => {
                stack.clear();
                self.current_filename = if let Some(path) = &path {
                    self.op_queue.image_list.change_dir(&path);
                    path.file_name().unwrap().to_str().unwrap().to_string()
                } else {
                    String::new()
                };

                let view = Box::new(ImageView::new(display, image_data, path));
                self.resize
                    .set_size(Vec2::new(view.size.x() as u32, view.size.y() as u32));
                self.image_view = Some(view);

                let window_context = display.gl_window();
                let window = window_context.window();

                if self.current_filename.is_empty() {
                    window.set_title("Simp");
                } else {
                    window.set_title(&self.current_filename.to_string());
                }

                self.largest_fit();
            }
            Output::FlipHorizontal => {
                self.image_view.as_mut().unwrap().flip_horizontal();
                stack.push(UndoFrame::FlipHorizontal);
            }
            Output::FlipVertical => {
                self.image_view.as_mut().unwrap().flip_vertical();
                stack.push(UndoFrame::FlipVertical);
            }
            Output::Rotate(dir) => {
                self.image_view.as_mut().unwrap().rotate(dir);
                stack.push(UndoFrame::Rotate(dir));
            }
            Output::Resize(mut frames) => {
                if let Some(ref mut view) = self.image_view {
                    view.swap_frames(&mut frames, display);
                    stack.push(UndoFrame::Resize(frames));
                }
            }
            Output::Color(mut frames) => {
                if let Some(ref mut view) = self.image_view {
                    view.swap_frames(&mut frames, display);
                    stack.push(UndoFrame::Color(frames));
                    view.hue = 0.0;
                    view.contrast = 0.0;
                    view.saturation = 0.0;
                    view.brightness = 0.0;
                    view.grayscale = false;
                    view.invert = false;
                    let window = display.gl_window();
                    window.window().request_redraw();
                }
            }
            Output::Crop(mut frames, rotation) => {
                if let Some(ref mut view) = self.image_view {
                    view.set_rotation(0);
                    view.swap_frames(&mut frames, display);
                    stack.push(UndoFrame::Crop { frames, rotation })
                }
            }
            Output::Undo => {
                let frame = stack.undo();
                if let Some(frame) = frame {
                    match frame {
                        UndoFrame::Rotate(rot) => {
                            self.image_view.as_mut().unwrap().rotate(-*rot);
                        }
                        UndoFrame::FlipHorizontal => {
                            self.image_view.as_mut().unwrap().flip_horizontal();
                        }
                        UndoFrame::FlipVertical => {
                            self.image_view.as_mut().unwrap().flip_vertical();
                        }
                        UndoFrame::Crop { frames, rotation } => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(frames, display);
                            view.swap_rotation(rotation);
                        }
                        UndoFrame::Resize(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(frames, display);
                        }
                        UndoFrame::Color(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(frames, display);
                        }
                    }
                }
            }
            Output::Redo => {
                let frame = stack.redo();
                if let Some(frame) = frame {
                    match frame {
                        UndoFrame::Rotate(rot) => {
                            self.image_view.as_mut().unwrap().rotate(*rot);
                        }
                        UndoFrame::FlipHorizontal => {
                            self.image_view.as_mut().unwrap().flip_horizontal();
                        }
                        UndoFrame::FlipVertical => {
                            self.image_view.as_mut().unwrap().flip_vertical();
                        }
                        UndoFrame::Crop { frames, rotation } => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(frames, display);
                            view.swap_rotation(rotation);
                        }
                        UndoFrame::Resize(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(frames, display);
                        }
                        UndoFrame::Color(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(frames, display);
                        }
                    }
                }
            }
            Output::Close => {
                self.image_view = None;
                stack.clear();
                self.op_queue.image_list.clear();
                self.op_queue.cache.clear();
            }
            Output::Saved => {
                stack.set_saved();
            }
            // indicates that the operation is done with no output
            Output::Done => (),
        }
    }

    pub fn handle_user_event(&mut self, display: &Display, event: &mut UserEvent) {
        match event {
            UserEvent::QueueLoad(path) => {
                self.queue(Op::LoadPath(path.to_path_buf(), false));
            }
            UserEvent::QueueSave(path) => {
                self.queue(Op::Save(path.to_path_buf()));
            }
            UserEvent::QueueDelete(path) => {
                self.queue(Op::Delete(path.to_path_buf()));
            }
            UserEvent::ErrorMessage(error) => {
                let dialog = rfd::MessageDialog::new()
                    .set_parent(display.gl_window().window())
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("About")
                    .set_description(error)
                    .set_buttons(rfd::MessageButtons::Ok);

                thread::spawn(move || dialog.show());
            }
            UserEvent::Exit => {
                let exit = self.exit.clone();
                let dialog = get_unsaved_changes_dialog();
                if self.op_queue.undo_stack().is_edited() {
                    thread::spawn(move || {
                        if dialog.show() {
                            exit.store(true, Ordering::Relaxed);
                        }
                    });
                } else {
                    exit.store(true, Ordering::Relaxed);
                }
            }
            UserEvent::Output(output) => {
                if let Some(output) = output.take() {
                    self.op_queue.set_working(false);
                    self.handle_output(display, output);
                }
            }
        };
    }

    pub fn handle_window_event(&mut self, display: &Display, event: &WindowEvent<'_>) {
        match event {
            WindowEvent::Resized(size) => {
                *self.size.mut_x() = size.width as f32;
                *self.size.mut_y() = size.height as f32;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position.set_x(position.x as f32);
                self.mouse_position.set_y(position.y as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if !self.metadata_visible {
                    let scroll = match delta {
                        MouseScrollDelta::LineDelta(_, y) => *y,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };

                    self.zoom(scroll, self.mouse_position);
                }
            }
            WindowEvent::ModifiersChanged(state) => self.modifiers = *state,
            WindowEvent::DroppedFile(path) => {
                self.op_queue.cache.clear();
                self.queue(Op::LoadPath(path.to_path_buf(), true));
            }
            WindowEvent::KeyboardInput { input, .. } if !self.resize.visible => {
                if let Some(key) = input.virtual_keycode {
                    match input.state {
                        ElementState::Pressed => match key {
                            VirtualKeyCode::Delete => {
                                if let Some(ref view) = self.image_view {
                                    if let Some(ref path) = view.path {
                                        delete(path.clone(), self.proxy.clone(), display);
                                    }
                                }
                            }

                            VirtualKeyCode::H if self.modifiers.ctrl() => self.help_visible = true,
                            VirtualKeyCode::O if self.modifiers.ctrl() => {
                                load_image::open(self.proxy.clone(), display, false)
                            }
                            VirtualKeyCode::S if self.modifiers.ctrl() => save_image::open(
                                self.current_filename.clone(),
                                self.proxy.clone(),
                                display,
                            ),
                            VirtualKeyCode::W if self.modifiers.ctrl() => {
                                let _ = self.proxy.send_event(UserEvent::Exit);
                            }
                            VirtualKeyCode::N if self.modifiers.ctrl() => new_window(),

                            VirtualKeyCode::L => {
                                self.largest_fit();
                            }
                            VirtualKeyCode::B => {
                                self.best_fit();
                            }

                            VirtualKeyCode::Q => {
                                if self.image_view.is_some() {
                                    self.queue(Op::Rotate(-1))
                                }
                            }
                            VirtualKeyCode::E => {
                                if self.image_view.is_some() {
                                    self.queue(Op::Rotate(1))
                                }
                            }

                            VirtualKeyCode::F5 => {
                                if let Some(image) = self.image_view.as_ref() {
                                    if let Some(path) = &image.path {
                                        let buf = path.to_path_buf();
                                        self.queue(Op::LoadPath(buf, false));
                                    }
                                }
                            }

                            VirtualKeyCode::C if self.modifiers.ctrl() => {
                                if self.view_available() {
                                    self.queue(Op::Copy);
                                }
                            }
                            VirtualKeyCode::V if self.modifiers.ctrl() => {
                                if !self.op_queue.working() {
                                    self.queue(Op::Paste);
                                }
                            }
                            VirtualKeyCode::X if self.modifiers.ctrl() => {
                                self.image_view.as_mut().unwrap().start_crop()
                            }

                            VirtualKeyCode::Z if self.modifiers.ctrl() => {
                                self.queue(Op::Undo);
                            }
                            VirtualKeyCode::Y if self.modifiers.ctrl() => {
                                self.queue(Op::Redo);
                            }

                            VirtualKeyCode::R if self.modifiers.ctrl() => {
                                self.resize.visible = true;
                            }

                            VirtualKeyCode::Left | VirtualKeyCode::D => {
                                if self.view_available()
                                    && !self.image_view.as_ref().unwrap().cropping()
                                {
                                    self.queue(Op::Prev);
                                }
                            }

                            VirtualKeyCode::Right | VirtualKeyCode::A => {
                                if self.view_available()
                                    && !self.image_view.as_ref().unwrap().cropping()
                                {
                                    self.queue(Op::Next);
                                }
                            }
                            VirtualKeyCode::F4 if self.modifiers.ctrl() => {
                                self.queue(Op::Close);
                            }

                            VirtualKeyCode::F11 | VirtualKeyCode::F => {
                                let window_context = display.gl_window();
                                let window = window_context.window();
                                let fullscreen = window.fullscreen();
                                if fullscreen.is_some() {
                                    window.set_fullscreen(None);
                                    self.fullscreen = false;
                                    self.top_bar_size = TOP_BAR_SIZE;
                                    self.bottom_bar_size = BOTTOM_BAR_SIZE;
                                } else {
                                    window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                                    self.fullscreen = true;
                                    self.top_bar_size = 0.0;
                                    self.bottom_bar_size = 0.0;
                                }
                            }
                            VirtualKeyCode::Escape => {
                                if self.view_available()
                                    && self.image_view.as_ref().unwrap().cropping()
                                {
                                    self.image_view.as_mut().unwrap().cancel_crop();
                                } else {
                                    let window_context = display.gl_window();
                                    let window = window_context.window();
                                    let fullscreen = window.fullscreen();
                                    if fullscreen.is_some() {
                                        window.set_fullscreen(None);
                                        self.fullscreen = false;
                                    }
                                }
                            }
                            _ => (),
                        },
                        ElementState::Released => (),
                    }
                }
            }
            WindowEvent::ReceivedCharacter(c) if !self.resize.visible => match c {
                '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                    if let Some(ref mut view) = self.image_view {
                        let zoom = c.to_digit(10).unwrap() as f32;
                        view.scale = zoom;
                    }
                }
                '+' => {
                    self.zoom(1.0, self.size / 2.0);
                }
                '-' => {
                    self.zoom(-1.0, self.size / 2.0);
                }
                _ => (),
            },
            _ => (),
        };
    }

    pub fn handle_ui(&mut self, display: &Display, ctx: &egui::Context) {
        if self.op_queue.working() {
            ctx.output().cursor_icon = CursorIcon::Progress;
        }

        if !self.fullscreen {
            self.menu_bar(display, ctx);
            self.bottom_bar(ctx);
        }

        self.main_area(display, ctx);
        self.resize_ui(ctx);
        self.help_ui(ctx);
        self.color_ui(ctx);
        self.metadata_ui(ctx);
        self.crop_ui(ctx);
    }

    pub fn main_area(&mut self, _display: &Display, ctx: &egui::Context) {
        let frame = egui::Frame::dark_canvas(&Style::default()).multiply_with_opacity(0.0);
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if self.image_view.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("Open File: Ctrl + O\n\nPaste: Ctrl + V\n\nHelp: Ctrl + H")
                            .size(20.0),
                    );
                });
            }

            if let Some(ref mut view) = self.image_view {
                view.handle_drag(ui);
            }
        });
    }

    fn bottom_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if self.image_view.is_some() {
                    ui.add_enabled_ui(
                        self.view_available() && !self.image_view.as_ref().unwrap().cropping(),
                        |ui| {
                            if ui.small_button("⬅").clicked() {
                                self.queue(Op::Prev);
                            }
                            if ui.small_button("➡").clicked() {
                                self.queue(Op::Next);
                            }
                        },
                    );
                }

                if let Some(image) = self.image_view.as_mut() {
                    ui.label(format!("{} x {}", image.size.x(), image.size.y()));
                    ui.label(format!("Zoom: {}%", (image.scale * 100.0).round()));

                    let g = image.image_data.read();
                    let buf = g.as_ref().unwrap().frames[0].buffer();
                    let color_space = match buf {
                        DynamicImage::ImageLuma8(_) => "Luma8",
                        DynamicImage::ImageLumaA8(_) => "LumaA8",
                        DynamicImage::ImageRgb8(_) => "Rgb8",
                        DynamicImage::ImageRgba8(_) => "Rgba8",
                        DynamicImage::ImageLuma16(_) => "Luma16",
                        DynamicImage::ImageLumaA16(_) => "LumaA16",
                        DynamicImage::ImageRgb16(_) => "Rgb16",
                        DynamicImage::ImageRgba16(_) => "Rgba16",
                        DynamicImage::ImageRgb32F(_) => "Rgb32F",
                        DynamicImage::ImageRgba32F(_) => "Rgba32F",
                        _ => panic!("Unknown color space name. This is a bug."),
                    };

                    {
                        let pos = (((self.mouse_position
                            - image.position
                            - (image.rotated_size() / 2.0) * image.scale)
                            + image.rotated_size() * image.scale)
                            / image.scale)
                            .floor()
                            .map(|v| v as i64);

                        if pos.x() >= 0
                            && pos.y() >= 0
                            && pos.x() < image.rotated_size().x() as i64
                            && pos.y() < image.rotated_size().y() as i64
                        {
                            let guard = image.image_data.read().unwrap();
                            let frame = &guard.frames[image.index];
                            let buffer = frame.buffer();

                            let mut pos = pos.map(|v| v as u32);
                            match image.rotation() {
                                0 => (),
                                1 => {
                                    pos.swap();
                                    *pos.mut_y() = image.size.y() as u32 - pos.y() - 1;
                                },
                                2 => {
                                    *pos.mut_x() = image.size.x() as u32 - pos.x() - 1;
                                    *pos.mut_y() = image.size.y() as u32 - pos.y() - 1;
                                },
                                3 => {
                                    pos.swap();
                                    *pos.mut_x() = image.size.x() as u32 - pos.x() - 1;
                                },
                                _ => panic!("rotated more then 360 degrees"),
                            }

                            if image.horizontal_flip {
                                *pos.mut_x() = image.size.x() as u32 - pos.x() - 1;
                            }

                            if image.vertical_flip {
                                *pos.mut_y() = image.size.y() as u32 - pos.y() - 1;
                            }

                            fn p2s<P: >(p: P) -> String
                            where
                                P: image::Pixel,
                                <P as image::Pixel>::Subpixel: ToString,
                            {
                                let channels = p.channels();
                                let mut out = String::new();

                                for i in 0..channels.len() {
                                    out.push_str(&channels[i].to_string());
                                    if i < channels.len() - 1 {
                                        out.push_str(", ");
                                    }
                                }

                                out
                            }

                            #[rustfmt::skip]
                            let color_str = match buffer {
                                DynamicImage::ImageLuma8(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                DynamicImage::ImageLumaA8(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                DynamicImage::ImageRgb8(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                DynamicImage::ImageRgba8(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                DynamicImage::ImageLuma16(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                DynamicImage::ImageLumaA16(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                DynamicImage::ImageRgb16(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                DynamicImage::ImageRgba16(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                DynamicImage::ImageRgb32F(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                DynamicImage::ImageRgba32F(b) => p2s(*b.get_pixel(pos.x(), pos.y())),
                                _ => panic!("Unknown color space name. This is a bug."),
                            };
                            ui.label(format!("{}: {}", color_space, color_str));
                            return;
                        }
                        ui.label(color_space);
                    }
                }
            });
        });
    }

    pub fn update(&mut self, display: &Display) -> (bool, Option<Duration>) {
        self.delay = None;

        if let Some(ref mut image) = self.image_view {
            update_delay(&mut self.delay, &image.animate(display));
        }

        if let Some(ref mut image) = self.image_view {
            let image_size = image.real_size();
            let mut window_size = self.size;
            window_size.set_y(window_size.y() - self.top_bar_size - self.bottom_bar_size);

            const MARGIN: f32 = 50.0;

            if image_size.x() < window_size.x() {
                image.position.set_x(self.size.x() / 2.0);
            } else {
                if image.position.x() - image_size.x() / 2.0 > 0.0 + MARGIN {
                    image.position.set_x(image_size.x() / 2.0 + MARGIN);
                }

                if image.position.x() + image_size.x() / 2.0 < window_size.x() - MARGIN {
                    image
                        .position
                        .set_x(window_size.x() - image_size.x() / 2.0 - MARGIN);
                }
            }

            if image_size.y() < window_size.y() {
                image.position.set_y(self.size.y() / 2.0);
            } else {
                if image.position.y() - image_size.y() / 2.0 > self.top_bar_size + MARGIN {
                    image
                        .position
                        .set_y(image_size.y() / 2.0 + self.top_bar_size + MARGIN);
                }

                if image.position.y() + image_size.y() / 2.0
                    < window_size.y() + self.top_bar_size - MARGIN
                {
                    image.position.set_y(
                        (window_size.y() - image_size.y() / 2.0) + self.top_bar_size - MARGIN,
                    );
                }
            }
        }

        (self.exit.load(Ordering::Relaxed), self.delay)
    }

    pub fn resize_ui(&mut self, ctx: &egui::Context) {
        if self.resize.visible {
            let mut open = self.image_view.is_some();
            let mut resized = false;
            egui::Window::new("Resize")
                .id(egui::Id::new("resize window"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("resize grid").show(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("Width: ");
                        });
                        let w_focus = ui.text_edit_singleline(&mut self.resize.width).has_focus();
                        ui.end_row();
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("Height: ");
                        });
                        let h_focus = ui.text_edit_singleline(&mut self.resize.height).has_focus();
                        ui.end_row();

                        self.resize.width.retain(|c| c.is_ascii_digit());
                        self.resize.height.retain(|c| c.is_ascii_digit());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("Maintain aspect ratio: ");
                        });
                        ui.checkbox(&mut self.resize.maintain_aspect_ratio, "");
                        ui.end_row();

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            ui.label("Resample: ");
                        });
                        let selected = &mut self.resize.resample;
                        egui::ComboBox::new("filter", "")
                            .selected_text(filter_name(selected))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    selected,
                                    FilterType::Nearest,
                                    filter_name(&FilterType::Nearest),
                                );
                                ui.selectable_value(
                                    selected,
                                    FilterType::Triangle,
                                    filter_name(&FilterType::Triangle),
                                );
                                ui.selectable_value(
                                    selected,
                                    FilterType::CatmullRom,
                                    filter_name(&FilterType::CatmullRom),
                                );
                                ui.selectable_value(
                                    selected,
                                    FilterType::Gaussian,
                                    filter_name(&FilterType::Gaussian),
                                );
                                ui.selectable_value(
                                    selected,
                                    FilterType::Lanczos3,
                                    filter_name(&FilterType::Lanczos3),
                                );
                            });
                        ui.end_row();
                        ui.end_row();

                        let width = self.resize.width.parse::<u32>();
                        let height = self.resize.height.parse::<u32>();

                        if self.resize.maintain_aspect_ratio && w_focus && width.is_ok() {
                            let width = *width.as_ref().unwrap();
                            let size = self.image_view.as_ref().unwrap().size;
                            let ratio = width as f32 / size.x();
                            self.resize.height = ((ratio * size.y()) as u32).to_string();
                        }

                        if self.resize.maintain_aspect_ratio && h_focus && height.is_ok() {
                            let height = *height.as_ref().unwrap();
                            let size = self.image_view.as_ref().unwrap().size;
                            let ratio = height as f32 / size.y();
                            self.resize.width = ((ratio * size.x()) as u32).to_string();
                        }

                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::Center),
                            |ui| {
                                if ui.add(Button::new("Cancel")).clicked() {
                                    resized = true;
                                }
                            },
                        );

                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::Center),
                            |ui| {
                                if ui
                                    .add_enabled(
                                        width.is_ok() && height.is_ok() && self.view_available(),
                                        Button::new("Resize"),
                                    )
                                    .clicked()
                                {
                                    let width = width.unwrap();
                                    let height = height.unwrap();
                                    self.queue(Op::Resize(
                                        Vec2::new(width, height),
                                        self.resize.resample,
                                    ));
                                    resized = true;
                                }
                            },
                        );
                    });
                });
            self.resize.visible = open && !resized;
        }
    }

    fn crop_ui(&mut self, ctx: &egui::Context) {
        let mut crop = None;
        if let Some(ref mut view) = self.image_view {
            let mut cancel = false;
            let size = view.rotated_size();
            if let Some(rect) = view.crop.rect.as_mut() {
                egui::Window::new("Crop")
                    .id(egui::Id::new("crop window"))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        egui::Grid::new("crop grid").show(ui, |ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                                ui.label("X: ");
                            });
                            ui.text_edit_singleline(&mut view.crop.x);
                            ui.end_row();
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                                ui.label("Y: ");
                            });
                            ui.text_edit_singleline(&mut view.crop.y);
                            ui.end_row();

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                                ui.label("Width: ");
                            });
                            ui.text_edit_singleline(&mut view.crop.width);
                            ui.end_row();
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                                ui.label("Height: ");
                            });
                            ui.text_edit_singleline(&mut view.crop.height);
                            ui.end_row();
                            ui.end_row();

                            *rect.position.mut_x() = min!(
                                view.crop.x.parse::<u32>().unwrap_or(0) as f32,
                                size.x() - 1.0
                            );
                            *rect.position.mut_y() = min!(
                                view.crop.y.parse::<u32>().unwrap_or(0) as f32,
                                size.y() - 1.0
                            );

                            *rect.size.mut_x() =
                                (view.crop.width.parse::<u32>().unwrap_or(size.x() as u32) as f32)
                                    .clamp(1.0, size.x() - rect.x());
                            *rect.size.mut_y() =
                                (view.crop.height.parse::<u32>().unwrap_or(size.y() as u32) as f32)
                                    .clamp(1.0, size.y() - rect.y());

                            if view.crop.x.parse::<u32>().is_ok() {
                                view.crop.x = rect.x().to_string();
                            }

                            if view.crop.y.parse::<u32>().is_ok() {
                                view.crop.y = rect.y().to_string();
                            }

                            if view.crop.width.parse::<u32>().is_ok() {
                                view.crop.width = rect.width().to_string();
                            }

                            if view.crop.height.parse::<u32>().is_ok() {
                                view.crop.height = rect.height().to_string();
                            }

                            view.crop.x.retain(|c| c.is_ascii_digit());
                            view.crop.y.retain(|c| c.is_ascii_digit());
                            view.crop.width.retain(|c| c.is_ascii_digit());
                            view.crop.height.retain(|c| c.is_ascii_digit());

                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::Center),
                                |ui| {
                                    if ui.add(Button::new("Cancel").wrap(false)).clicked() {
                                        cancel = true;
                                    }
                                },
                            );

                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::Center),
                                |ui| {
                                    if ui.add(Button::new("Crop").wrap(false)).clicked() {
                                        crop = Some(*rect);
                                        cancel = true;
                                    }
                                },
                            );
                        });
                    });
            }
            if cancel {
                view.cancel_crop();
            }
        }
        if let Some(rect) = crop {
            self.queue(Op::Crop(rect));
        }
    }

    pub fn queue(&mut self, op: Op) {
        self.op_queue
            .queue(op, self.image_view.as_ref().map(|v| v.as_ref()))
    }

    fn zoom(&mut self, zoom: f32, mouse_position: Vec2<f32>) {
        if let Some(ref mut image) = self.image_view {
            let old_scale = image.scale;
            image.scale += image.scale * zoom as f32 / 10.0;

            let new_size = image.scaled();
            if (new_size.x() < 100.0 || new_size.y() < 100.0)
                && old_scale >= image.scale
                && image.scale < 1.0
            {
                image.scale = min!(old_scale, 1.0);
            } else {
                let mouse_to_center = image.position - mouse_position;
                image.position -= mouse_to_center * (old_scale - image.scale) / old_scale;
            }
        }
    }

    pub fn best_fit(&mut self) {
        if let Some(ref mut view) = self.image_view {
            let scaling = min!(
                self.size.x() / view.size.x(),
                (self.size.y() - self.top_bar_size - self.bottom_bar_size) / view.size.y()
            );
            view.scale = min!(scaling, 1.0);
            view.position = self.size / 2.0;
        }
    }

    pub fn largest_fit(&mut self) {
        if let Some(ref mut view) = self.image_view {
            let scaling = min!(
                self.size.x() / view.size.x(),
                (self.size.y() - self.top_bar_size - self.bottom_bar_size) / view.size.y()
            );
            view.scale = scaling;
            view.position = self.size / 2.0;
        }
    }

    pub fn new(proxy: EventLoopProxy<UserEvent>, size: [f32; 2]) -> Self {
        App {
            exit: Arc::new(AtomicBool::new(false)),
            delay: None,
            image_view: None,
            size: Vec2::from(size),
            fullscreen: false,
            top_bar_size: TOP_BAR_SIZE,
            bottom_bar_size: BOTTOM_BAR_SIZE,
            op_queue: OpQueue::new(proxy.clone()),
            proxy,
            modifiers: ModifiersState::empty(),
            mouse_position: Vec2::default(),
            current_filename: String::new(),
            resize: Resize::default(),
            help_visible: false,
            color_visible: false,
            metadata_visible: false,
        }
    }
}

pub fn delete(path: PathBuf, proxy: EventLoopProxy<UserEvent>, display: &Display) {
    let dialog = rfd::MessageDialog::new()
        .set_parent(display.gl_window().window())
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Move to trash")
        .set_description("Are you sure you want to move this to trash?")
        .set_buttons(rfd::MessageButtons::YesNo);
    thread::spawn(move || {
        if dialog.show() {
            let _ = proxy.send_event(UserEvent::QueueDelete(path));
        }
    });
}

fn new_window() {
    let _ = Command::new(std::env::current_exe().unwrap()).spawn();
}

fn update_delay(old: &mut Option<Duration>, new: &Option<Duration>) {
    if let Some(ref mut old_time) = old {
        if let Some(ref new_time) = new {
            if *old_time > *new_time {
                *old_time = *new_time;
            }
        }
    } else {
        *old = *new;
    }
}

fn filter_name(filter: &FilterType) -> &'static str {
    match filter {
        FilterType::Nearest => "Nearest Neighbor",
        FilterType::Triangle => "Linear Filter",
        FilterType::CatmullRom => "Cubic Filter",
        FilterType::Gaussian => "Gaussian Filter",
        FilterType::Lanczos3 => "Lanczos",
    }
}
