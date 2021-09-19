use std::{
    path::Path,
    process::Command,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use glium::{
    backend::glutin::Display,
    glutin::{
        event::{ElementState, ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowEvent},
        event_loop::EventLoopProxy,
        window::{CursorIcon, Fullscreen},
    },
};
use imgui::*;
use imgui_glium_renderer::Renderer;
use rect::Rect;
use util::{min, UserEvent};
use vec2::Vec2;

pub mod image_view;
use image_view::ImageView;

pub mod image_list;
use image_list::ImageList;

pub mod arrows;
use arrows::{Action, Arrows};

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

const TOP_BAR_SIZE: f32 = 25.0;
const BOTTOM_BAR_SIZE: f32 = 22.0;

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
    arrows: Arrows,
    stack: UndoStack,
    pub crop: Box<Crop>,
    pub cache: Arc<Cache>,
    pub image_loader: Arc<RwLock<ImageLoader>>,
}

impl App {
    pub fn update(
        &mut self,
        ui: &mut Ui<'_>,
        display: &glium::Display,
        _renderer: &mut Renderer,
        window_event: Option<&WindowEvent<'_>>,
        user_event: Option<&mut UserEvent>,
    ) -> (bool, Option<Duration>) {
        self.exit = false;
        self.delay = None;
        {
            let dimensions = display.get_framebuffer_dimensions();
            self.size = Vec2::new(dimensions.0 as f32, dimensions.1 as f32)
        }

        if let Some(ref mut image) = self.image_view {
            update_delay(&mut self.delay, &image.animate(display));
        }

        if let Some(event) = user_event {
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

                        self.image_view = Some(Box::new(ImageView::new(
                            display,
                            images.clone(),
                            path.clone(),
                        )));

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

        if let Some(event) = window_event {
            match event {
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
                WindowEvent::DroppedFile(path) => load_image::load(
                    self.proxy.clone(),
                    path,
                    self.cache.clone(),
                    self.image_loader.clone(),
                ),
                WindowEvent::KeyboardInput { input, .. } => {
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
                                        self.stack.push(UndoFrame::Rotate(1));
                                        image.rotate(1);
                                    }
                                }
                                VirtualKeyCode::E => {
                                    if let Some(ref mut image) = self.image_view {
                                        self.stack.push(UndoFrame::Rotate(-1));
                                        image.rotate(-1);
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
                WindowEvent::ReceivedCharacter(c) => match c {
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

        if let Some(ref mut image) = self.image_view {
            if ui.is_mouse_dragging(imgui::MouseButton::Left) {
                if self.crop.cropping {
                    if let Some(ref mut inner) = self.crop.inner {
                        let delta = Vec2::from(ui.mouse_drag_delta(imgui::MouseButton::Left));
                        inner.current += delta;
                    } else {
                        let cursor_pos = self.mouse_position;
                        let delta = Vec2::from(ui.mouse_drag_delta(imgui::MouseButton::Left));
                        self.crop.inner = Some(crop::Inner {
                            start: cursor_pos - delta,
                            current: cursor_pos,
                        });
                    }
                } else {
                    let delta = Vec2::from(ui.mouse_drag_delta(imgui::MouseButton::Left));
                    image.position += delta;
                }
                ui.reset_mouse_drag_delta(imgui::MouseButton::Left);
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

        let styles = ui.push_style_vars(&[
            StyleVar::WindowPadding([10.0, 10.0]),
            StyleVar::FramePadding([0.0, 6.0]),
            StyleVar::ItemSpacing([5.0, 10.0]),
            StyleVar::WindowBorderSize(0.0),
        ]);

        let colors = ui.push_style_colors(&[
            (StyleColor::MenuBarBg, [0.117, 0.117, 0.117, 1.0]),
            (StyleColor::ButtonHovered, [0.078, 0.078, 0.078, 1.0]),
            (StyleColor::ButtonActive, [0.078, 0.078, 0.078, 1.0]),
        ]);

        if !self.fullscreen {
            self.menu_bar(display, ui);
        }

        let s = ui.push_style_vars(&[
            StyleVar::WindowPadding([10.0, 4.0]),
            StyleVar::FramePadding([0.0, 0.0]),
            StyleVar::ItemSpacing([0.0, 0.0]),
            StyleVar::ButtonTextAlign([0.0, 0.5]),
        ]);

        let c = ui.push_style_colors(&[
            (StyleColor::WindowBg, [0.117, 0.117, 0.117, 1.0]),
            (StyleColor::Button, [0.117, 0.117, 0.117, 1.0]),
        ]);

        if !self.fullscreen {
            self.bottom_bar(ui);
        }

        c.pop(ui);
        s.pop(ui);

        styles.pop(ui);
        colors.pop(ui);
        (self.exit, self.delay)
    }

    pub fn menu_bar(&mut self, display: &Display, ui: &mut Ui<'_>) {
        ui.main_menu_bar(|| {
            ui.menu(im_str!("File"), true, || {
                if MenuItem::new(im_str!("Open"))
                    .shortcut(im_str!("Ctrl + O"))
                    .build(ui)
                {
                    load_image::open(
                        self.proxy.clone(),
                        display,
                        self.cache.clone(),
                        self.image_loader.clone(),
                    );
                }

                if MenuItem::new(im_str!("Save as"))
                    .shortcut(im_str!("Ctrl + S"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    save_image::open(self.current_filename.clone(), self.proxy.clone(), display);
                }

                ui.separator();

                if MenuItem::new(im_str!("New Window"))
                    .shortcut(im_str!("Ctrl + N"))
                    .build(ui)
                {
                    new_window();
                }

                if MenuItem::new(im_str!("Refresh"))
                    .shortcut(im_str!("R"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    if let Some(ref path) = self.image_view.as_ref().unwrap().path {
                        load_image::load(
                            self.proxy.clone(),
                            path,
                            self.cache.clone(),
                            self.image_loader.clone(),
                        );
                    }
                }

                ui.separator();

                if MenuItem::new(im_str!("Exit"))
                    .shortcut(im_str!("Ctrl + W"))
                    .build(ui)
                {
                    self.exit = true;
                }
            });

            ui.menu(im_str!("Edit"), true, || {
                if MenuItem::new(im_str!("Undo"))
                    .shortcut(im_str!("Ctrl + Z"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.undo(display);
                }

                if MenuItem::new(im_str!("Redo"))
                    .shortcut(im_str!("Ctrl + Y"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.redo(display);
                }

                ui.separator();

                if MenuItem::new(im_str!("Copy"))
                    .shortcut(im_str!("Ctrl + C"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    let image = self.image_view.as_ref().unwrap();
                    clipboard::copy(image);
                }

                if MenuItem::new(im_str!("Paste"))
                    .shortcut(im_str!("Ctrl + V"))
                    .build(ui)
                {
                    clipboard::paste(&self.proxy);
                }
            });

            ui.menu(im_str!("Image"), true, || {
                if MenuItem::new(im_str!("Rotate Left"))
                    .shortcut(im_str!("Q"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.stack.push(UndoFrame::Rotate(1));
                    self.image_view.as_mut().unwrap().rotate(1);
                }

                if MenuItem::new(im_str!("Rotate Right"))
                    .shortcut(im_str!("E"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.stack.push(UndoFrame::Rotate(-1));
                    self.image_view.as_mut().unwrap().rotate(-1);
                }

                ui.separator();

                if MenuItem::new(im_str!("Flip Horizontal"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.stack.push(UndoFrame::FlipHorizontal);
                    let image = self.image_view.as_mut().unwrap();
                    image.flip_horizontal(display);
                }

                if MenuItem::new(im_str!("Flip Vertical"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.stack.push(UndoFrame::FlipVertical);
                    let image = self.image_view.as_mut().unwrap();
                    image.flip_vertical(display);
                }

                ui.separator();

                if MenuItem::new(im_str!("Zoom in"))
                    .shortcut(im_str!("+"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.zoom(1.0, self.size / 2.0);
                }

                if MenuItem::new(im_str!("Zoom out"))
                    .shortcut(im_str!("-"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.zoom(-1.0, self.size / 2.0);
                }

                ui.separator();

                if MenuItem::new(im_str!("Best fit"))
                    .shortcut(im_str!("E"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.best_fit();
                }

                if MenuItem::new(im_str!("Largest fit"))
                    .shortcut(im_str!("F"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.best_fit();
                }

                ui.separator();

                if MenuItem::new(im_str!("Crop"))
                    .shortcut(im_str!("Ctrl + X"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    self.crop.cropping = true;
                }

                if MenuItem::new(im_str!("Resize"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    todo!();
                }

                ui.separator();

                if MenuItem::new(im_str!("Delete"))
                    .shortcut(im_str!("Delete"))
                    .enabled(self.image_view.is_some())
                    .build(ui)
                {
                    if let Some(ref view) = self.image_view {
                        if let Some(ref path) = view.path {
                            delete(path, self.proxy.clone());
                        }
                    }
                }
            });

            ui.menu(im_str!("Help"), true, || {
                if MenuItem::new(im_str!("Repository")).build(ui) {
                    webbrowser::open("https://github.com/Kl4rry/simp").unwrap();
                }

                if MenuItem::new(im_str!("Report Bug")).build(ui) {
                    webbrowser::open("https://github.com/Kl4rry/simp/issues").unwrap();
                }

                ui.separator();

                if MenuItem::new(im_str!("About")).build(ui) {
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
                }
            });
        });
    }

    fn bottom_bar(&mut self, ui: &mut Ui<'_>) {
        Window::new(im_str!("Bottom"))
            .position([0.0, self.size.y() - BOTTOM_BAR_SIZE], Condition::Always)
            .size([self.size.x(), BOTTOM_BAR_SIZE], Condition::Always)
            .resizable(false)
            .bg_alpha(1.0)
            .movable(false)
            .no_decoration()
            .focus_on_appearing(false)
            .always_use_window_padding(true)
            .build(ui, || {
                if let Some(image) = self.image_view.as_mut() {
                    let (action, new_delay) = self.arrows.build(ui);
                    update_delay(&mut self.delay, &new_delay);
                    match action {
                        Action::Left => {
                            if let Some(path) = self.image_list.previous() {
                                load_image::load(
                                    self.proxy.clone(),
                                    path,
                                    self.cache.clone(),
                                    self.image_loader.clone(),
                                );
                            }
                        }
                        Action::Right => {
                            if let Some(path) = self.image_list.next() {
                                load_image::load(
                                    self.proxy.clone(),
                                    path,
                                    self.cache.clone(),
                                    self.image_loader.clone(),
                                );
                            }
                        }
                        Action::None => (),
                    }

                    ui.same_line_with_spacing(0.0, 20.0);
                    ui.text(&format!("{} x {}", image.size.x(), image.size.y()));
                    ui.same_line_with_spacing(0.0, 20.0);
                    ui.text(&format!("Zoom: {}%", (image.scale * 100.0).round()));
                }
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
            arrows: Arrows::new(),
            stack: UndoStack::new(),
            crop: Box::new(Crop::new(display)),
            cache,
            image_loader,
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
