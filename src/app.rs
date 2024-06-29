use std::{
    mem,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use cgmath::{EuclideanSpace, Point2, Vector2};
use egui::{Button, CursorIcon, Event, Modifiers, RichText, Style, TopBottomPanel};
use image::{imageops::FilterType, ColorType, DynamicImage};
use num_traits::Zero;
use winit::{
    event::WindowEvent, event_loop::EventLoopProxy, keyboard::ModifiersState, window::Fullscreen,
};

use crate::{
    min,
    util::{p2, UserEvent},
    WgpuState,
};

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

pub mod preferences;

pub mod dialog_manager;

use self::{
    dialog_manager::{DialogManager, DialogProxy},
    image_view::{crop_renderer, image_renderer},
    preferences::PREFERENCES,
    undo_stack::UndoFrame,
};

enum ResizeMode {
    Original,
    BestFit,
    LargestFit,
}

const TOP_BAR_SIZE: f32 = 26.0;
const BOTTOM_BAR_SIZE: f32 = 27.0;

pub struct App {
    exit: Arc<AtomicBool>,
    pub delay: Duration,
    pub image_renderer: image_renderer::Renderer,
    pub crop_renderer: crop_renderer::Renderer,
    pub image_view: Option<Box<ImageView>>,
    pub modifiers: ModifiersState,
    dialog_manager: DialogManager,
    color_type: ColorType,
    size: Vector2<f32>,
    top_bar_size: f32,
    bottom_bar_size: f32,
    proxy: EventLoopProxy<UserEvent>,
    mouse_position: Vector2<f32>,
    current_filename: String,
    op_queue: OpQueue,
    resize: Resize,
    resize_mode: ResizeMode,
    help_visible: bool,
    color_visible: bool,
    color_space_visible: bool,
    metadata_visible: bool,
    preferences_visible: bool,
    enter: bool,
}

impl App {
    pub fn view_available(&self) -> bool {
        !self.op_queue.working() && self.image_view.is_some()
    }

    pub fn handle_output(&mut self, wgpu: &WgpuState, output: Output) {
        let stack = self.op_queue.undo_stack_mut();
        match output {
            Output::ImageLoaded(image_data, path) => {
                stack.clear();
                self.current_filename = if let Some(path) = &path {
                    self.op_queue.image_list.change_dir(path);
                    path.file_name().unwrap().to_str().unwrap().to_string()
                } else {
                    String::new()
                };

                self.color_type = image_data.frames[0].buffer().color();

                let view = Box::new(ImageView::new(wgpu, image_data, path));
                self.resize
                    .set_size(Vector2::new(view.size.x as u32, view.size.y as u32));
                self.image_view = Some(view);

                if self.current_filename.is_empty() {
                    wgpu.window.set_title("Simp");
                } else {
                    wgpu.window.set_title(&self.current_filename.to_string());
                }

                self.largest_fit();
            }
            Output::FlipHorizontal => {
                let view = self.image_view.as_mut().unwrap();
                if view.rotation() % 2 != 0 {
                    view.flip_vertical();
                    stack.push(UndoFrame::FlipVertical);
                } else {
                    view.flip_horizontal();
                    stack.push(UndoFrame::FlipHorizontal);
                }
            }
            Output::FlipVertical => {
                let view = self.image_view.as_mut().unwrap();
                if view.rotation() % 2 != 0 {
                    view.flip_horizontal();
                    stack.push(UndoFrame::FlipHorizontal);
                } else {
                    view.flip_vertical();
                    stack.push(UndoFrame::FlipVertical);
                }
            }
            Output::Rotate(dir) => {
                self.image_view.as_mut().unwrap().rotate(dir);
                stack.push(UndoFrame::Rotate(dir));
            }
            Output::Resize(mut frames) => {
                if let Some(ref mut view) = self.image_view {
                    view.swap_frames(wgpu, &mut frames);
                    stack.push(UndoFrame::Resize(frames));
                }
            }
            Output::ColorSpace(mut frames) => {
                if let Some(ref mut view) = self.image_view {
                    let new = frames[0].buffer().color();
                    let old = view.image_data.read().unwrap().frames[0].buffer().color();
                    if new != old {
                        view.swap_frames(wgpu, &mut frames);
                        stack.push(UndoFrame::ColorSpace(frames));
                    }
                }
            }
            Output::Color(mut frames) => {
                if let Some(ref mut view) = self.image_view {
                    view.swap_frames(wgpu, &mut frames);
                    stack.push(UndoFrame::Color(frames));
                    view.hue = 0.0;
                    view.contrast = 0.0;
                    view.saturation = 0.0;
                    view.brightness = 0.0;
                    view.grayscale = false;
                    view.invert = false;
                    wgpu.window.request_redraw();
                }
            }
            Output::Crop(mut frames, rotation) => {
                if let Some(ref mut view) = self.image_view {
                    view.set_rotation(0);
                    view.swap_frames(wgpu, &mut frames);
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
                            view.swap_frames(wgpu, frames);
                            view.swap_rotation(rotation);
                        }
                        UndoFrame::Resize(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(wgpu, frames);
                        }
                        UndoFrame::Color(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(wgpu, frames);
                        }
                        UndoFrame::ColorSpace(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(wgpu, frames);
                            self.color_type =
                                view.image_data.read().unwrap().frames[0].buffer().color();
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
                            view.swap_frames(wgpu, frames);
                            view.swap_rotation(rotation);
                        }
                        UndoFrame::Resize(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(wgpu, frames);
                        }
                        UndoFrame::Color(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(wgpu, frames);
                        }
                        UndoFrame::ColorSpace(frames) => {
                            let view = self.image_view.as_mut().unwrap();
                            view.swap_frames(wgpu, frames);
                            self.color_type =
                                view.image_data.read().unwrap().frames[0].buffer().color();
                        }
                    }
                }
            }
            Output::Close => {
                self.image_view = None;
                stack.clear();
                self.op_queue.image_list.clear();
                self.op_queue.cache.clear();
                wgpu.window.set_title("Simp");
            }
            Output::Saved => {
                stack.set_saved();
            }
            // indicates that the operation is done with no output
            Output::Done => (),
        }
    }

    pub fn handle_user_event(&mut self, wgpu: &WgpuState, event: &mut UserEvent) {
        match event {
            UserEvent::LoadBytes(bytes) => {
                self.queue(Op::LoadBytes(mem::take(bytes)));
            }
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
                let error = error.clone();
                self.dialog_manager
                    .get_proxy()
                    .spawn_dialog("Error", move |ui, enter| {
                        ui.label(&error);
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT), |ui| {
                            if ui.button("Ok").clicked() {
                                Some(())
                            } else if *enter {
                                *enter = false;
                                Some(())
                            } else {
                                None
                            }
                        })
                        .inner
                    });
            }
            UserEvent::Exit => {
                let exit = self.exit.clone();
                let dialog_proxy = self.dialog_manager.get_proxy();
                if self.op_queue.undo_stack().is_edited() {
                    thread::spawn(move || {
                        let close = dialog_proxy
                            .spawn_dialog("Unsaved changes", move |ui, enter| {
                                ui.label(
                            "You have unsaved changes are you sure you want to close this image?",
                        );
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::LEFT),
                                    |ui| {
                                        if ui.button("Ok").clicked() {
                                            return Some(true);
                                        }

                                        if ui.button("Cancel").clicked() {
                                            return Some(false);
                                        }

                                        if *enter {
                                            *enter = false;
                                            return Some(true);
                                        }

                                        None
                                    },
                                )
                                .inner
                            })
                            .wait()
                            .unwrap_or(false);
                        if close {
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
                    self.handle_output(wgpu, output);
                }
            }
            UserEvent::RepaintRequest(request_repaint_info) => {
                self.delay = self.delay.min(request_repaint_info.delay);
            }
            UserEvent::Wake => (),
        };
    }

    pub fn handle_window_event(&mut self, _wgpu: &WgpuState, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position.x = position.x as f32;
                self.mouse_position.y = position.y as f32;
            }
            WindowEvent::DroppedFile(path) => {
                self.op_queue.cache.clear();
                self.queue(Op::LoadPath(path.to_path_buf(), true));
            }
            _ => (),
        };
    }

    pub fn handle_ui(&mut self, wgpu: &WgpuState, ctx: &egui::Context) {
        if self.op_queue.working() {
            ctx.set_cursor_icon(CursorIcon::Progress);
        }

        self.main_area(wgpu, ctx);

        if wgpu.window.fullscreen().is_none() {
            self.menu_bar(wgpu, ctx);
            self.bottom_bar(ctx);
        }

        self.resize_ui(ctx);
        self.preferences_ui(ctx);
        self.help_ui(ctx);
        self.color_ui(ctx);
        self.color_space_ui(ctx);
        self.metadata_ui(ctx);
        self.crop_ui(ctx);

        self.dialog_manager.update(ctx, self.size, &mut self.enter);
    }

    pub fn main_area(&mut self, wgpu: &WgpuState, ctx: &egui::Context) {
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

            if self.dialog_manager.dialog_count() == 0 {
                ui.input_mut(|input| {
                    use egui::{Key::*, KeyboardShortcut};

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: Delete,
                    }) {
                        if let Some(ref view) = self.image_view {
                            if let Some(ref path) = view.path {
                                delete(
                                    path.clone(),
                                    self.dialog_manager.get_proxy(),
                                    self.proxy.clone(),
                                );
                            }
                        }
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: H,
                    }) {
                        self.help_visible = true
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: O,
                    }) {
                        load_image::open(self.proxy.clone(), wgpu, false)
                    }
                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: S,
                    }) {
                        save_image::open(self.current_filename.clone(), self.proxy.clone(), wgpu)
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: Q,
                    }) {
                        let _ = self.proxy.send_event(UserEvent::Exit);
                    }
                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: N,
                    }) {
                        new_window()
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: L,
                    }) {
                        self.largest_fit();
                    }
                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: B,
                    }) {
                        self.best_fit();
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: Q,
                    }) && self.image_view.is_some()
                    {
                        self.queue(Op::Rotate(-1))
                    }
                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: E,
                    }) && self.image_view.is_some()
                    {
                        self.queue(Op::Rotate(1))
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: F5,
                    }) {
                        if let Some(image) = self.image_view.as_ref() {
                            if let Some(path) = &image.path {
                                let buf = path.to_path_buf();
                                self.queue(Op::LoadPath(buf, false));
                            }
                        }
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: Z,
                    }) {
                        self.queue(Op::Undo);
                    }
                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: Y,
                    }) {
                        self.queue(Op::Redo);
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: R,
                    }) {
                        self.resize.visible = true;
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: ArrowRight,
                    }) || input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: D,
                    }) || input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: H,
                    }) && self.view_available()
                        && !self.image_view.as_ref().unwrap().cropping()
                    {
                        self.queue(Op::Prev);
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: ArrowLeft,
                    }) || input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: A,
                    }) || input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: L,
                    }) && self.view_available()
                        && !self.image_view.as_ref().unwrap().cropping()
                    {
                        self.queue(Op::Next);
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: W,
                    }) {
                        self.queue(Op::Close);
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: F11,
                    }) || input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::CTRL,
                        logical_key: F,
                    }) {
                        let fullscreen = wgpu.window.fullscreen();
                        if fullscreen.is_some() {
                            wgpu.window.set_fullscreen(None);
                            self.top_bar_size = TOP_BAR_SIZE;
                            self.bottom_bar_size = BOTTOM_BAR_SIZE;
                        } else {
                            wgpu.window
                                .set_fullscreen(Some(Fullscreen::Borderless(None)));
                            self.top_bar_size = 0.0;
                            self.bottom_bar_size = 0.0;
                        }
                        self.largest_fit();
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: Escape,
                    }) {
                        if self.view_available() && self.image_view.as_ref().unwrap().cropping() {
                            self.image_view.as_mut().unwrap().cancel_crop();
                        }
                        self.help_visible = false;
                        self.color_visible = false;
                        self.metadata_visible = false;
                        self.resize.visible = false;
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: Enter,
                    }) {
                        self.enter = true;
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: Plus,
                    }) {
                        self.zoom(1.0, self.size / 2.0);
                    }

                    if input.consume_shortcut(&KeyboardShortcut {
                        modifiers: Modifiers::NONE,
                        logical_key: Minus,
                    }) {
                        self.zoom(-1.0, self.size / 2.0);
                    }

                    self.zoom(
                        input.raw_scroll_delta.y / 40.0 * PREFERENCES.lock().unwrap().zoom_speed,
                        self.mouse_position,
                    );

                    for event in &input.events {
                        match event {
                            Event::Copy => {
                                if self.view_available() {
                                    self.queue(Op::Copy);
                                }
                            }
                            Event::Cut => {
                                if self.view_available() {
                                    self.image_view.as_mut().unwrap().start_crop()
                                }
                            }
                            _ => (),
                        }
                    }
                    input
                        .events
                        .retain(|e| !matches!(e, Event::Copy | Event::Cut));

                    let nums = [Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9];
                    for (i, num) in nums.into_iter().enumerate() {
                        if input.consume_shortcut(&KeyboardShortcut {
                            modifiers: Modifiers::CTRL,
                            logical_key: num,
                        }) {
                            if let Some(ref mut view) = self.image_view {
                                view.scale = (i + 1) as f32;
                            }
                        }
                    }
                });
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
                    ui.label(self.current_filename.to_string());
                    ui.label(format!("{} x {}", image.size.x, image.size.y));
                    ui.label(format!("Zoom: {}%", (image.scale * 100.0).round()));

                    let g = image.image_data.read();
                    let buf = g.as_ref().unwrap().frames[0].buffer();
                    let mut color_space = color_type_to_str(buf.color()).to_string();

                    {
                        let pos = (((self.mouse_position
                            - image.position
                            - (image.rotated_size() / 2.0) * image.scale)
                            + image.rotated_size() * image.scale)
                            / image.scale)
                            .map(|v| v.floor() as i64);

                        if pos.x >= 0
                            && pos.y >= 0
                            && pos.x < image.rotated_size().x as i64
                            && pos.y < image.rotated_size().y as i64
                        {
                            let guard = image.image_data.read().unwrap();
                            let frame = &guard.frames[image.index];
                            let buffer = frame.buffer();

                            let mut pos = pos.map(|v| v as u32);
                            match image.rotation() {
                                0 => (),
                                1 => {
                                    mem::swap(&mut pos.x, &mut pos.y);
                                    pos.y = image.size.y as u32 - pos.y - 1;
                                }
                                2 => {
                                    pos.x = image.size.x as u32 - pos.x - 1;
                                    pos.y = image.size.y as u32 - pos.y - 1;
                                }
                                3 => {
                                    mem::swap(&mut pos.x, &mut pos.y);
                                    pos.x = image.size.x as u32 - pos.x - 1;
                                }
                                _ => panic!("rotated more then 360 degrees"),
                            }

                            if image.horizontal_flip {
                                pos.x = image.size.x as u32 - pos.x - 1;
                            }

                            if image.vertical_flip {
                                pos.y = image.size.y as u32 - pos.y - 1;
                            }

                            fn p2s<P>(p: P) -> String
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
                                DynamicImage::ImageLuma8(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                DynamicImage::ImageLumaA8(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                DynamicImage::ImageRgb8(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                DynamicImage::ImageRgba8(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                DynamicImage::ImageLuma16(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                DynamicImage::ImageLumaA16(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                DynamicImage::ImageRgb16(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                DynamicImage::ImageRgba16(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                DynamicImage::ImageRgb32F(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                DynamicImage::ImageRgba32F(b) => p2s(*b.get_pixel(pos.x, pos.y)),
                                _ => panic!("Unknown color space name. This is a bug."),
                            };
                            color_space = format!("{color_space}: {color_str}");
                        }
                        if ui.label(color_space).clicked() {
                            self.color_space_visible = true;
                        }
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.op_queue.working() {
                        ui.add(egui::widgets::Spinner::new().size(14.0));
                    }
                });
            });
        });
    }

    pub fn update(&mut self, _wgpu: &WgpuState) -> (bool, Duration) {
        self.delay = Duration::MAX;
        if let Some(ref mut image) = self.image_view {
            self.delay = self.delay.min(image.animate());
        }

        if let Some(ref mut image) = self.image_view {
            let image_size = image.real_size();
            let mut window_size = self.size;
            window_size.y = window_size.y - self.top_bar_size - self.bottom_bar_size;

            const MARGIN: f32 = 50.0;

            if image_size.x < window_size.x {
                image.position.x = self.size.x / 2.0;
            } else {
                if image.position.x - image_size.x / 2.0 > 0.0 + MARGIN {
                    image.position.x = image_size.x / 2.0 + MARGIN;
                }

                if image.position.x + image_size.x / 2.0 < window_size.x - MARGIN {
                    image.position.x = window_size.x - image_size.x / 2.0 - MARGIN;
                }
            }

            if image_size.y < window_size.y {
                image.position.y = self.size.y / 2.0;
            } else {
                if image.position.y - image_size.y / 2.0 > self.top_bar_size + MARGIN {
                    image.position.y = image_size.y / 2.0 + self.top_bar_size + MARGIN;
                }

                if image.position.y + image_size.y / 2.0
                    < window_size.y + self.top_bar_size - MARGIN
                {
                    image.position.y =
                        (window_size.y - image_size.y / 2.0) + self.top_bar_size - MARGIN;
                }
            }
        }

        self.enter = false;
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
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(p2(Point2::from_vec(self.size / 2.0)))
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
                            ui.label("Maintain aspect ratio:");
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
                            let ratio = width as f32 / size.x;
                            self.resize.height = ((ratio * size.y) as u32).to_string();
                        }

                        if self.resize.maintain_aspect_ratio && h_focus && height.is_ok() {
                            let height = *height.as_ref().unwrap();
                            let size = self.image_view.as_ref().unwrap().size;
                            let ratio = height as f32 / size.y;
                            self.resize.width = ((ratio * size.x) as u32).to_string();
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
                                    || self.enter
                                {
                                    let width = width.unwrap();
                                    let height = height.unwrap();
                                    self.queue(Op::Resize(
                                        Vector2::new(width, height),
                                        self.resize.resample,
                                    ));
                                    resized = true;
                                    self.enter = false;
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
                    .pivot(egui::Align2::CENTER_CENTER)
                    .default_pos(p2(Point2::from_vec(self.size / 2.0)))
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

                            rect.position.x =
                                min!(view.crop.x.parse::<u32>().unwrap_or(0) as f32, size.x - 1.0);
                            rect.position.y =
                                min!(view.crop.y.parse::<u32>().unwrap_or(0) as f32, size.y - 1.0);

                            rect.size.x = (view.crop.width.parse::<u32>().unwrap_or(size.x as u32)
                                as f32)
                                .clamp(1.0, size.x - rect.x());
                            rect.size.y = (view.crop.height.parse::<u32>().unwrap_or(size.y as u32)
                                as f32)
                                .clamp(1.0, size.y - rect.y());

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
                                    if ui.add(Button::new("Crop").wrap(false)).clicked()
                                        || self.enter
                                    {
                                        crop = Some(*rect);
                                        cancel = true;
                                        self.enter = false;
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

    fn zoom(&mut self, zoom: f32, mouse_position: Vector2<f32>) {
        if let Some(ref mut image) = self.image_view {
            let old_scale = image.scale;
            image.scale += image.scale * zoom / 10.0;

            let new_size = image.scaled();
            if (new_size.x < 1.0 || new_size.y < 1.0)
                && old_scale >= image.scale
                && image.scale < 1.0
            {
                image.scale = min!(old_scale, 1.0);
            } else {
                let mouse_to_center = image.position - mouse_position;
                image.position -= mouse_to_center * (old_scale - image.scale) / old_scale;
            }
            self.resize_mode = ResizeMode::Original;
        }
    }

    pub fn best_fit(&mut self) {
        if let Some(ref mut view) = self.image_view {
            let size = view.rotated_size();
            let scaling = min!(
                self.size.x / size.x,
                (self.size.y - self.top_bar_size - self.bottom_bar_size) / size.y
            );
            view.scale = min!(scaling, 1.0);
            view.position = self.size / 2.0;
            self.resize_mode = ResizeMode::BestFit;
        }
    }

    pub fn largest_fit(&mut self) {
        if let Some(ref mut view) = self.image_view {
            let size = view.rotated_size();
            let scaling = min!(
                self.size.x / size.x,
                (self.size.y - self.top_bar_size - self.bottom_bar_size) / size.y
            );
            view.scale = scaling;
            view.position = self.size / 2.0;
            self.resize_mode = ResizeMode::LargestFit;
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.size.x = size.width as f32;
        self.size.y = size.height as f32;

        match self.resize_mode {
            ResizeMode::Original => {}
            ResizeMode::LargestFit => {
                self.largest_fit();
            }
            ResizeMode::BestFit => {
                self.best_fit();
            }
        }
    }

    pub fn handle_paste(&mut self) {
        if !self.op_queue.working() {
            self.queue(Op::Paste);
        }
    }

    pub fn new(wgpu: &WgpuState, proxy: EventLoopProxy<UserEvent>, size: [f32; 2]) -> Self {
        let dialog_manager = DialogManager::new(proxy.clone());
        App {
            exit: Arc::new(AtomicBool::new(false)),
            delay: Duration::MAX,
            modifiers: ModifiersState::empty(),
            image_renderer: image_renderer::Renderer::new(wgpu),
            crop_renderer: crop_renderer::Renderer::new(wgpu),
            image_view: None,
            op_queue: OpQueue::new(proxy.clone(), dialog_manager.get_proxy()),
            dialog_manager,
            color_type: ColorType::Rgba8,
            size: Vector2::from(size),
            top_bar_size: TOP_BAR_SIZE,
            bottom_bar_size: BOTTOM_BAR_SIZE,
            proxy,
            mouse_position: Vector2::zero(),
            current_filename: String::new(),
            resize: Resize::default(),
            help_visible: false,
            color_visible: false,
            color_space_visible: false,
            metadata_visible: false,
            preferences_visible: false,
            resize_mode: ResizeMode::Original,
            enter: false,
        }
    }
}

pub fn delete(
    path: std::path::PathBuf,
    dialog_proxy: DialogProxy,
    proxy: EventLoopProxy<UserEvent>,
) {
    dialog_proxy.spawn_dialog("Move to trash", move |ui, enter| {
        ui.label("Are you sure you want to move this to trash?");

        ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT), |ui| {
            if ui.button("Yes").clicked() {
                let _ = proxy.send_event(UserEvent::QueueDelete(path.clone()));
                return Some(());
            }

            if ui.button("No").clicked() {
                return Some(());
            }

            if *enter {
                *enter = false;
                let _ = proxy.send_event(UserEvent::QueueDelete(path.clone()));
                return Some(());
            }

            None
        })
        .inner
    });
}

fn new_window() {
    let _ = Command::new(std::env::current_exe().unwrap()).spawn();
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

pub fn color_type_to_str(color_type: ColorType) -> &'static str {
    match color_type {
        ColorType::L8 => "Luma8",
        ColorType::La8 => "LumaA8",
        ColorType::Rgb8 => "Rgb8",
        ColorType::Rgba8 => "Rgba8",
        ColorType::L16 => "Luma16",
        ColorType::La16 => "LumaA16",
        ColorType::Rgb16 => "Rgb16",
        ColorType::Rgba16 => "Rgba16",
        ColorType::Rgb32F => "Rgb32F",
        ColorType::Rgba32F => "Rgba32F",
        _ => panic!("Unknown color space name. This is a bug."),
    }
}
