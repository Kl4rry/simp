use std::{path::Path, process::Command, time::Duration};

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
mod save_image;
use crop::Crop;
pub mod cursor;
mod undo_stack;
use std::thread;

use undo_stack::{UndoFrame, UndoStack};

const TOP_BAR_SIZE: f32 = 25.0;
const BOTTOM_BAR_SIZE: f32 = 22.0;

pub struct App {
    pub image_view: Option<ImageView>,
    size: Vec2<f32>,
    proxy: EventLoopProxy<UserEvent>,
    modifiers: ModifiersState,
    mouse_position: Vec2<f32>,
    current_filename: String,
    image_list: ImageList,
    arrows: Arrows,
    stack: UndoStack,
    pub crop: Crop,
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
        let mut exit = false;
        let mut delay: Option<Duration> = None;
        {
            let dimensions = display.get_framebuffer_dimensions();
            self.size = Vec2::new(dimensions.0 as f32, dimensions.1 as f32)
        }

        if let Some(ref mut image) = self.image_view {
            update_delay(&mut delay, &image.animate(display));
        }

        if let Some(event) = user_event {
            match event {
                UserEvent::ImageLoaded(images, path, instant) => {
                    cursor::set_cursor_icon(CursorIcon::default(), display);
                    let replace = if let Some(ref old) = self.image_view {
                        old.start.saturating_duration_since(*instant) == Duration::from_secs(0)
                    } else {
                        true
                    };

                    let images = images.take().unwrap();
                    if replace {
                        self.image_view =
                            Some(ImageView::new(display, images, path.clone(), *instant));

                        self.current_filename = if let Some(path) = path {
                            self.image_list.change_dir(&path);
                            path.file_name().unwrap().to_str().unwrap().to_string()
                        } else {
                            String::new()
                        };
                    }
                    self.best_fit();
                    self.stack.reset();
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
                UserEvent::Error(error) => {
                    cursor::set_cursor_icon(CursorIcon::default(), display);
                    let error = error.clone();
                    thread::spawn(move || {
                        msgbox::create("Error", &error, msgbox::IconType::Error).unwrap()
                    });
                }
                UserEvent::SetCursor(icon) => cursor::set_cursor_icon(*icon, display),
                UserEvent::Exit => exit = true,
            };
        }

        if let Some(event) = window_event {
            match event {
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
                WindowEvent::DroppedFile(path) => load_image::load(self.proxy.clone(), path),
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

                                VirtualKeyCode::O if self.modifiers.ctrl() => {
                                    load_image::open(self.proxy.clone(), display)
                                }
                                VirtualKeyCode::S if self.modifiers.ctrl() => save_image::open(
                                    self.current_filename.clone(),
                                    self.proxy.clone(),
                                    display,
                                ),
                                VirtualKeyCode::W if self.modifiers.ctrl() => exit = true,
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

                                VirtualKeyCode::R if self.modifiers.ctrl() => {
                                    if let Some(image) = self.image_view.as_ref() {
                                        if let Some(path) = &image.path {
                                            load_image::load(self.proxy.clone(), path);
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
                                            load_image::load(self.proxy.clone(), path);
                                        }
                                    }
                                }

                                VirtualKeyCode::Right | VirtualKeyCode::A => {
                                    if let Some(path) = self.image_list.next() {
                                        if self.crop.inner.is_none() {
                                            load_image::load(self.proxy.clone(), path);
                                        }
                                    }
                                }

                                VirtualKeyCode::F11 => {
                                    let window_context = display.gl_window();
                                    let window = window_context.window();
                                    let fullscreen = window.fullscreen();
                                    if fullscreen.is_some() {
                                        window.set_fullscreen(None);
                                    } else {
                                        window.set_fullscreen(Some(Fullscreen::Borderless(None)));
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
                        ui.reset_mouse_drag_delta(imgui::MouseButton::Left);
                    } else {
                        let cursor_pos = self.mouse_position;
                        let delta = Vec2::from(ui.mouse_drag_delta(imgui::MouseButton::Left));
                        self.crop.inner = Some(crop::Inner {
                            start: cursor_pos - delta,
                            current: cursor_pos,
                        });
                        ui.reset_mouse_drag_delta(imgui::MouseButton::Left);
                    }
                } else {
                    let delta = Vec2::from(ui.mouse_drag_delta(imgui::MouseButton::Left));
                    image.position += delta;
                    ui.reset_mouse_drag_delta(imgui::MouseButton::Left);
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

        if let Some(ref mut image) = self.image_view {
            let image_size = image.real_size();
            let mut window_size = self.size;
            window_size.set_y(window_size.y() - TOP_BAR_SIZE - BOTTOM_BAR_SIZE);

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
                if image.position.y() - image_size.y() / 2.0 > TOP_BAR_SIZE {
                    image.position.set_y(image_size.y() / 2.0 + TOP_BAR_SIZE);
                }

                if image.position.y() + image_size.y() / 2.0 < window_size.y() + TOP_BAR_SIZE {
                    image
                        .position
                        .set_y((window_size.y() - image_size.y() / 2.0) + TOP_BAR_SIZE);
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

        ui.main_menu_bar(|| {
            ui.menu(im_str!("File"), true, || {
                if MenuItem::new(im_str!("Open"))
                    .shortcut(im_str!("Ctrl + O"))
                    .build(ui)
                {
                    load_image::open(self.proxy.clone(), display);
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
                        load_image::load(self.proxy.clone(), path);
                    }
                }

                ui.separator();

                if MenuItem::new(im_str!("Exit"))
                    .shortcut(im_str!("Ctrl + W"))
                    .build(ui)
                {
                    exit = true;
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
                    update_delay(&mut delay, &new_delay);
                    match action {
                        Action::Left => {
                            if let Some(path) = self.image_list.previous() {
                                load_image::load(self.proxy.clone(), path);
                            }
                        }
                        Action::Right => {
                            if let Some(path) = self.image_list.next() {
                                load_image::load(self.proxy.clone(), path);
                            }
                        }
                        Action::None => (),
                    }

                    ui.same_line_with_spacing(0.0, 10.0);
                    if self.current_filename.len() > 20 {
                        let mut text = self.current_filename.chars().take(20).collect::<String>();
                        text.push_str("...");
                        ui.text(&text);
                    } else {
                        ui.text(&self.current_filename);
                    }

                    ui.same_line_with_spacing(0.0, 20.0);
                    ui.text(&format!("{} x {}", image.size.x(), image.size.y()));
                    ui.same_line_with_spacing(0.0, 20.0);
                    ui.text(&format!("Zoom: {}%", (image.scale * 100.0).round()));
                } else {
                    ui.same_line_with_spacing(0.0, 10.0);
                    ui.text("No File");
                }
            });

        c.pop(ui);
        s.pop(ui);

        styles.pop(ui);
        colors.pop(ui);
        (exit, delay)
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
                (self.size.y() - TOP_BAR_SIZE - BOTTOM_BAR_SIZE) / view.size.y()
            );
            view.scale = min!(scaling, 1.0);
            view.position = self.size / 2.0;
        }
    }

    pub fn largest_fit(&mut self) {
        if let Some(ref mut view) = self.image_view {
            let scaling = min!(
                self.size.x() / view.size.x(),
                (self.size.y() - TOP_BAR_SIZE - BOTTOM_BAR_SIZE) / view.size.y()
            );
            view.scale = scaling;
            view.position = self.size / 2.0;
        }
    }

    pub fn new(proxy: EventLoopProxy<UserEvent>, size: [f32; 2], display: &Display) -> Self {
        App {
            image_view: None,
            size: Vec2::new(size[0], size[1]),
            proxy,
            modifiers: ModifiersState::empty(),
            mouse_position: Vec2::default(),
            current_filename: String::new(),
            image_list: ImageList::new(),
            arrows: Arrows::new(),
            stack: UndoStack::new(),
            crop: Crop::new(display),
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
                let _ = proxy.send_event(UserEvent::Error(error.to_string()));
            }
        }
    });
}

fn new_window() {
    Command::new(std::env::current_exe().unwrap())
        .spawn()
        .unwrap();
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
