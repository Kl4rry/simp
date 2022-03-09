use std::{
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

use egui::{
    menu::{self},
    Button, RichText, Style, TopBottomPanel,
};
use glium::{
    backend::glutin::Display,
    glutin::{
        event::{ElementState, ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowEvent},
        event_loop::EventLoopProxy,
        window::{CursorIcon, Fullscreen},
    },
};
use image::imageops::FilterType;
use lazy_static::*;

use crate::{
    min,
    rect::Rect,
    util::{Image, UserEvent},
    vec2::Vec2,
};

pub mod image_view;
use image_view::ImageView;

pub mod image_list;
use image_list::ImageList;

mod clipboard;

pub mod crop;

pub mod load_image;

pub mod image_loader;
use image_loader::ImageLoader;

mod save_image;
use crop::Crop;

pub mod cursor;

mod undo_stack;
use undo_stack::{UndoFrame, UndoStack};

mod cache;
use cache::Cache;

mod resize;
use resize::Resize;

const TOP_BAR_SIZE: f32 = 26.0;
const BOTTOM_BAR_SIZE: f32 = 27.0;

lazy_static! {
    pub static ref RESIZING: AtomicBool = AtomicBool::new(false);
}

pub struct App {
    exit: bool,
    delay: Option<Duration>,
    pub image_view: Option<Box<ImageView>>,
    pub size: Vec2<f32>,
    pub position: Vec2<i32>,
    fullscreen: bool,
    pub top_bar_size: f32,
    pub bottom_bar_size: f32,
    proxy: EventLoopProxy<UserEvent>,
    modifiers: ModifiersState,
    mouse_position: Vec2<f32>,
    current_filename: String,
    image_list: ImageList,
    stack: UndoStack,
    pub crop: Box<Crop>,
    pub cache: Arc<Cache>,
    pub image_loader: Arc<RwLock<ImageLoader>>,
    resize: Resize,
}

impl App {
    pub fn handle_user_event(&mut self, display: &Display, event: &mut UserEvent) {
        match event {
            UserEvent::ImageLoaded(images, path) => {
                let mut replace = true;
                {
                    let mut guard = self.image_loader.write().unwrap();
                    match path {
                        Some(path) => match guard.target_file {
                            Some(ref mut target) => {
                                if path != target {
                                    replace = false;
                                } else {
                                    guard.target_file = None;
                                }
                            }
                            None => replace = false,
                        },
                        None => guard.target_file = None,
                    }
                }

                if let Some(path) = path {
                    self.image_loader.write().unwrap().loading.remove(path);
                }

                if replace {
                    cursor::set_cursor_icon(CursorIcon::default(), display);

                    let view = Box::new(ImageView::new(display, images.clone(), path.clone()));

                    self.resize
                        .set_size(Vec2::new(view.size.x() as u32, view.size.y() as u32));
                    self.image_view = Some(view);

                    self.current_filename = if let Some(path) = path {
                        self.image_list.change_dir(&path);
                        path.file_name().unwrap().to_str().unwrap().to_string()
                    } else {
                        String::new()
                    };

                    let window_context = display.gl_window();
                    let window = window_context.window();

                    if self.current_filename.is_empty() {
                        window.set_title("Simp");
                    } else {
                        window.set_title(&self.current_filename.to_string());
                    }

                    self.best_fit();
                    self.stack.reset();
                }
            }
            UserEvent::Resize(images) => {
                if let Some(ref mut view) = self.image_view {
                    view.swap_frames(images.as_mut().unwrap(), display);
                    self.stack.push(UndoFrame::Resize(images.take().unwrap()));
                }
                self.best_fit();
                RESIZING.store(false, Ordering::SeqCst);
            }
            UserEvent::Save(path) => {
                if let Some(ref view) = self.image_view {
                    save_image::save(
                        self.proxy.clone(),
                        path.clone(),
                        view.frames.clone(),
                        view.rotation,
                        view.horizontal_flip,
                        view.vertical_flip,
                    );
                }
            }
            UserEvent::LoadError(error, path) => {
                self.image_loader.write().unwrap().loading.remove(path);
                cursor::set_cursor_icon(CursorIcon::default(), display);
                let error = error.clone();
                thread::spawn(move || {
                    msgbox::create("Error", &error, msgbox::IconType::Error).unwrap()
                });
            }
            UserEvent::ErrorMessage(error) => {
                cursor::set_cursor_icon(CursorIcon::default(), display);
                let error = error.clone();
                thread::spawn(move || {
                    msgbox::create("Error", &error, msgbox::IconType::Error).unwrap()
                });
            }
            UserEvent::SetCursor(icon) => cursor::set_cursor_icon(*icon, display),
            UserEvent::Exit => self.exit = true,
        };
    }

    pub fn handle_window_event(&mut self, display: &Display, event: &WindowEvent<'_>) {
        match event {
            WindowEvent::Resized(size) => {
                *self.size.mut_x() = size.width as f32;
                *self.size.mut_y() = size.height as f32;
            }
            WindowEvent::Moved(position) => {
                *self.position.mut_x() = position.x;
                *self.position.mut_x() = position.y;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position.set_x(position.x as f32);
                self.mouse_position.set_y(position.y as f32);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };

                if self.crop.inner.is_none() {
                    self.zoom(scroll, self.mouse_position);
                }
            }
            WindowEvent::ModifiersChanged(state) => self.modifiers = *state,
            WindowEvent::DroppedFile(path) => {
                self.cache.clear();
                load_image::load(
                    self.proxy.clone(),
                    path,
                    self.cache.clone(),
                    self.image_loader.clone(),
                )
            }
            WindowEvent::KeyboardInput { input, .. } if !self.resize.visible => {
                if let Some(key) = input.virtual_keycode {
                    match input.state {
                        ElementState::Pressed => match key {
                            VirtualKeyCode::Delete => {
                                if let Some(ref view) = self.image_view {
                                    if let Some(ref path) = view.path {
                                        delete(path, self.proxy.clone());
                                    }
                                }
                            }

                            VirtualKeyCode::O if self.modifiers.ctrl() => load_image::open(
                                self.proxy.clone(),
                                display,
                                self.cache.clone(),
                                self.image_loader.clone(),
                            ),
                            VirtualKeyCode::S if self.modifiers.ctrl() => save_image::open(
                                self.current_filename.clone(),
                                self.proxy.clone(),
                                display,
                            ),
                            VirtualKeyCode::W if self.modifiers.ctrl() => self.exit = true,
                            VirtualKeyCode::N if self.modifiers.ctrl() => new_window(),

                            VirtualKeyCode::F => {
                                self.largest_fit();
                            }
                            VirtualKeyCode::B => {
                                self.best_fit();
                            }

                            VirtualKeyCode::Q => {
                                if let Some(ref mut image) = self.image_view {
                                    self.stack.push(UndoFrame::Rotate(-1));
                                    image.rotate(-1);
                                }
                            }
                            VirtualKeyCode::E => {
                                if let Some(ref mut image) = self.image_view {
                                    self.stack.push(UndoFrame::Rotate(1));
                                    image.rotate(1);
                                }
                            }

                            VirtualKeyCode::F5 => {
                                if let Some(image) = self.image_view.as_ref() {
                                    if let Some(path) = &image.path {
                                        self.cache.clear();
                                        load_image::load(
                                            self.proxy.clone(),
                                            path,
                                            self.cache.clone(),
                                            self.image_loader.clone(),
                                        );
                                    }
                                }
                            }

                            VirtualKeyCode::C if self.modifiers.ctrl() => {
                                if let Some(ref view) = self.image_view {
                                    clipboard::copy(view);
                                }
                            }
                            VirtualKeyCode::V if self.modifiers.ctrl() => {
                                clipboard::paste(&self.proxy);
                            }
                            VirtualKeyCode::X if self.modifiers.ctrl() => {
                                self.crop.cropping = true;
                            }

                            VirtualKeyCode::Z if self.modifiers.ctrl() => {
                                self.undo(display);
                            }
                            VirtualKeyCode::Y if self.modifiers.ctrl() => {
                                self.redo(display);
                            }

                            VirtualKeyCode::R if self.modifiers.ctrl() => {
                                self.resize.visible = true;
                            }

                            VirtualKeyCode::Left | VirtualKeyCode::D => {
                                if let Some(path) = self.image_list.previous() {
                                    if self.crop.inner.is_none() {
                                        load_image::load(
                                            self.proxy.clone(),
                                            path,
                                            self.cache.clone(),
                                            self.image_loader.clone(),
                                        );
                                    }
                                }
                            }

                            VirtualKeyCode::Right | VirtualKeyCode::A => {
                                if let Some(path) = self.image_list.next() {
                                    if self.crop.inner.is_none() {
                                        load_image::load(
                                            self.proxy.clone(),
                                            path,
                                            self.cache.clone(),
                                            self.image_loader.clone(),
                                        );
                                    }
                                }
                            }

                            VirtualKeyCode::F11 => {
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
                                let window_context = display.gl_window();
                                let window = window_context.window();
                                let fullscreen = window.fullscreen();
                                if fullscreen.is_some() {
                                    window.set_fullscreen(None);
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
                    if self.crop.inner.is_none() {
                        self.zoom(1.0, self.size / 2.0);
                    }
                }
                '-' => {
                    if self.crop.inner.is_none() {
                        self.zoom(-1.0, self.size / 2.0);
                    }
                }
                _ => (),
            },
            _ => (),
        };
    }

    pub fn handle_ui(&mut self, display: &Display, ctx: &egui::Context) {
        if !self.fullscreen {
            self.menu_bar(display, ctx);
            self.bottom_bar(ctx);
        }
        self.main_area(display, ctx);
        self.resize_ui(ctx);
    }

    pub fn main_area(&mut self, display: &Display, ctx: &egui::Context) {
        let frame = egui::Frame::dark_canvas(&Style::default()).multiply_with_opacity(0.0);
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if self.image_view.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("Open File: Ctrl + O").size(20.0))
                });
            }

            let res = ui.interact(egui::Rect::EVERYTHING, ui.id(), egui::Sense::drag());

            if let Some(ref mut image) = self.image_view {
                if res.dragged_by(egui::PointerButton::Primary) {
                    let vec = res.drag_delta();
                    let delta = Vec2::from((vec.x, vec.y));
                    if self.crop.cropping {
                        if let Some(ref mut inner) = self.crop.inner {
                            inner.current += delta;
                        } else {
                            let cursor_pos = self.mouse_position;
                            self.crop.inner = Some(crop::Inner {
                                start: cursor_pos - delta,
                                current: cursor_pos,
                            });
                        }
                    } else {
                        image.position += delta;
                    }
                } else if self.crop.cropping {
                    if let Some(ref inner) = self.crop.inner {
                        let mut size = inner.current - inner.start;
                        *size.mut_x() = size.x().abs();
                        *size.mut_y() = size.y().abs();

                        let start = Vec2::new(
                            min!(inner.start.x(), inner.current.x()),
                            min!(inner.start.y(), inner.current.y()),
                        );

                        let frames = image.crop(Rect::new(start, size), display);
                        if let Some((frames, rotation)) = frames {
                            self.stack.push(UndoFrame::Crop { frames, rotation })
                        }

                        self.crop.inner = None;
                        self.crop.cropping = false;
                    }
                }
            }
        });
    }

    fn menu_bar(&mut self, display: &Display, ctx: &egui::Context) {
        TopBottomPanel::top("top").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu_button(ui, "File", |ui| {
                    if ui.button("Open").clicked() {
                        load_image::open(
                            self.proxy.clone(),
                            display,
                            self.cache.clone(),
                            self.image_loader.clone(),
                        );
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
                            load_image::load(
                                self.proxy.clone(),
                                path,
                                self.cache.clone(),
                                self.image_loader.clone(),
                            );
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
                        .add_enabled(self.image_view.is_some(), Button::new("Undo"))
                        .clicked()
                    {
                        self.undo(display);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Redo"))
                        .clicked()
                    {
                        self.redo(display);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Copy"))
                        .clicked()
                    {
                        let image = self.image_view.as_ref().unwrap();
                        clipboard::copy(image);
                        ui.close_menu();
                    }

                    if ui.button("Paste").clicked() {
                        clipboard::paste(&self.proxy);
                    }
                });

                menu::menu_button(ui, "Image", |ui| {
                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Rotate Left"))
                        .clicked()
                    {
                        self.stack.push(UndoFrame::Rotate(-1));
                        self.image_view.as_mut().unwrap().rotate(-1);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Rotate Right"))
                        .clicked()
                    {
                        self.stack.push(UndoFrame::Rotate(1));
                        self.image_view.as_mut().unwrap().rotate(1);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Flip Horizontal"))
                        .clicked()
                    {
                        self.stack.push(UndoFrame::FlipHorizontal);
                        let image = self.image_view.as_mut().unwrap();
                        image.flip_horizontal(display);
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.image_view.is_some(), Button::new("Flip Vertical"))
                        .clicked()
                    {
                        self.stack.push(UndoFrame::FlipVertical);
                        let image = self.image_view.as_mut().unwrap();
                        image.flip_vertical(display);
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
                        .add_enabled(self.image_view.is_some(), Button::new("Crop"))
                        .clicked()
                    {
                        self.crop.cropping = true;
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

    fn bottom_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(), |ui| {
                if let Some(image) = self.image_view.as_mut() {
                    if ui.small_button("Prev").clicked() {
                        if let Some(path) = self.image_list.previous() {
                            load_image::load(
                                self.proxy.clone(),
                                path,
                                self.cache.clone(),
                                self.image_loader.clone(),
                            );
                        }
                    }
                    if ui.small_button("Next").clicked() {
                        if let Some(path) = self.image_list.next() {
                            load_image::load(
                                self.proxy.clone(),
                                path,
                                self.cache.clone(),
                                self.image_loader.clone(),
                            );
                        }
                    }

                    ui.label(format!("{} x {}", image.size.x(), image.size.y()));
                    ui.label(format!("Zoom: {}%", (image.scale * 100.0).round()));
                }
            });
        });
    }

    // i want this function to die
    pub fn update(&mut self, display: &Display) -> (bool, Option<Duration>) {
        self.exit = false;
        self.delay = None;

        if let Some(ref mut image) = self.image_view {
            update_delay(&mut self.delay, &image.animate(display));
        }

        if let Some(ref mut image) = self.image_view {
            let image_size = image.real_size();
            let mut window_size = self.size;
            window_size.set_y(window_size.y() - self.top_bar_size - self.bottom_bar_size);

            if image_size.x() < window_size.x() {
                image.position.set_x(self.size.x() / 2.0);
            } else {
                if image.position.x() - image_size.x() / 2.0 > 0.0 {
                    image.position.set_x(image_size.x() / 2.0);
                }

                if image.position.x() + image_size.x() / 2.0 < window_size.x() {
                    image.position.set_x(window_size.x() - image_size.x() / 2.0);
                }
            }

            if image_size.y() < window_size.y() {
                image.position.set_y(self.size.y() / 2.0);
            } else {
                if image.position.y() - image_size.y() / 2.0 > self.top_bar_size {
                    image
                        .position
                        .set_y(image_size.y() / 2.0 + self.top_bar_size);
                }

                if image.position.y() + image_size.y() / 2.0 < window_size.y() + self.top_bar_size {
                    image
                        .position
                        .set_y((window_size.y() - image_size.y() / 2.0) + self.top_bar_size);
                }
            }
        }

        (self.exit, self.delay)
    }

    pub fn resize_ui(&mut self, ctx: &egui::Context) {
        if self.resize.visible {
            let mut open = self.image_view.is_some();
            let mut resized = false;
            egui::Window::new("Resize")
                .id(egui::Id::new("resize window"))
                .collapsible(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Width");
                    let w_focus = ui.text_edit_singleline(&mut self.resize.width).has_focus();
                    ui.label("Height");
                    let h_focus = ui.text_edit_singleline(&mut self.resize.height).has_focus();

                    self.resize.width.retain(|c| c.is_numeric());
                    self.resize.width.retain(|c| c.is_numeric());

                    ui.checkbox(
                        &mut self.resize.maintain_aspect_ratio,
                        "Maintain aspect ratio",
                    );

                    ui.label("Resample");
                    egui::ComboBox::new("filter", "")
                        .selected_text("Nearest Neighbor")
                        .show_ui(ui, |ui| {
                            let selected = &mut self.resize.resample;
                            ui.selectable_value(selected, FilterType::Nearest, "Nearest Neighbor");
                            ui.selectable_value(selected, FilterType::Triangle, "Linear Filter");
                            ui.selectable_value(selected, FilterType::CatmullRom, "Cubic Filter");
                            ui.selectable_value(selected, FilterType::Gaussian, "Gaussian Filter");
                            ui.selectable_value(selected, FilterType::Lanczos3, "Lanczos");
                        });

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

                    if ui
                        .add_enabled(
                            width.is_ok() && height.is_ok() && !RESIZING.load(Ordering::SeqCst),
                            Button::new("Resize"),
                        )
                        .clicked()
                    {
                        let width = width.unwrap();
                        let height = height.unwrap();
                        self.resize(Vec2::new(width, height));
                        resized = true;
                    }
                });
            self.resize.visible = open && !resized;
        }
    }

    fn resize(&self, size: Vec2<u32>) {
        if RESIZING
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        let frames = self.image_view.as_ref().unwrap().frames.clone();
        let resample = self.resize.resample;
        let proxy = self.proxy.clone();

        thread::spawn(move || {
            let guard = frames.read().unwrap();
            let mut new = Vec::new();
            for image in guard.iter() {
                let buffer = image.buffer().resize_exact(size.x(), size.y(), resample);
                new.push(Image::with_delay(buffer, image.delay));
            }
            let _ = proxy.send_event(UserEvent::Resize(Some(new)));
        });
    }

    pub fn undo(&mut self, display: &Display) {
        let frame = self.stack.undo();
        if let Some(frame) = frame {
            match frame {
                UndoFrame::Rotate(rot) => {
                    self.image_view.as_mut().unwrap().rotate(-*rot);
                }
                UndoFrame::FlipHorizontal => {
                    self.image_view.as_mut().unwrap().flip_horizontal(display);
                }
                UndoFrame::FlipVertical => {
                    self.image_view.as_mut().unwrap().flip_vertical(display);
                }
                UndoFrame::Crop { frames, rotation } => {
                    let view = self.image_view.as_mut().unwrap();
                    view.swap_frames(frames, display);
                    std::mem::swap(&mut view.rotation, rotation);
                }
                UndoFrame::Resize(frames) => {
                    let view = self.image_view.as_mut().unwrap();
                    view.swap_frames(frames, display);
                }
            }
        }
    }

    pub fn redo(&mut self, display: &Display) {
        let frame = self.stack.redo();
        if let Some(frame) = frame {
            match frame {
                UndoFrame::Rotate(rot) => {
                    self.image_view.as_mut().unwrap().rotate(*rot);
                }
                UndoFrame::FlipHorizontal => {
                    self.image_view.as_mut().unwrap().flip_horizontal(display);
                }
                UndoFrame::FlipVertical => {
                    self.image_view.as_mut().unwrap().flip_vertical(display);
                }
                UndoFrame::Crop { frames, rotation } => {
                    let view = self.image_view.as_mut().unwrap();
                    view.swap_frames(frames, display);
                    std::mem::swap(&mut view.rotation, rotation);
                }
                UndoFrame::Resize(frames) => {
                    let view = self.image_view.as_mut().unwrap();
                    view.swap_frames(frames, display);
                }
            }
        }
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

    pub fn new(
        proxy: EventLoopProxy<UserEvent>,
        size: [f32; 2],
        position: [i32; 2],
        display: &Display,
    ) -> Self {
        const MAX_SIZE: usize = 1_000_000_000;
        let cache = Arc::new(Cache::new(MAX_SIZE));
        let image_loader = Arc::new(RwLock::new(ImageLoader::new()));
        App {
            exit: false,
            delay: None,
            image_view: None,
            size: Vec2::from(size),
            position: Vec2::from(position),
            fullscreen: false,
            top_bar_size: TOP_BAR_SIZE,
            bottom_bar_size: BOTTOM_BAR_SIZE,
            image_list: ImageList::new(cache.clone(), proxy.clone(), image_loader.clone()),
            proxy,
            modifiers: ModifiersState::empty(),
            mouse_position: Vec2::default(),
            current_filename: String::new(),
            stack: UndoStack::new(),
            crop: Box::new(Crop::new(display)),
            cache,
            image_loader,
            resize: Resize::default(),
        }
    }
}

pub fn delete<P: AsRef<Path>>(path: P, proxy: EventLoopProxy<UserEvent>) {
    let path = path.as_ref().to_path_buf();
    thread::spawn(move || {
        let dialog = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title("Move to trash")
            .set_description("Are you sure u want to move this to trash")
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();

        if dialog {
            if let Err(error) = trash::delete(path) {
                let _ = proxy.send_event(UserEvent::ErrorMessage(error.to_string()));
            }
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
