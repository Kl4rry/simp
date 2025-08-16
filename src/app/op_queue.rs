use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

use cgmath::Vector2;
use image::{
    imageops::{
        colorops::{contrast_in_place, huerotate_in_place},
        FilterType,
    },
    ColorType, DynamicImage,
};
use winit::event_loop::EventLoopProxy;

use self::imageops::{adjust_saturation_in_place, brighten_in_place};
use super::{
    cache::Cache,
    clipboard,
    dialog_manager::DialogProxy,
    image_list::ImageList,
    image_view::ImageView,
    load_image::{load_from_bytes, load_uncached},
    save_image,
};
use crate::{
    app::undo_stack::UndoStack,
    rect::Rect,
    util::{extensions::EXTENSIONS, Image, ImageData, UserEvent},
};

mod imageops;

#[derive(Debug)]
pub enum Op {
    LoadPath(PathBuf, bool),
    LoadBytes(Vec<u8>),
    Next,
    Prev,
    Save(PathBuf),
    Resize(Vector2<u32>, FilterType),
    Color {
        hue: f32,
        saturation: f32,
        contrast: f32,
        brightness: f32,
        grayscale: bool,
        invert: bool,
    },
    Crop(Rect),
    ColorSpace(ColorType),
    FlipHorizontal,
    FlipVertical,
    Rotate(i32),
    Undo,
    Redo,
    Close,
    Copy,
    Paste,
    Delete(PathBuf),
}

pub enum Output {
    ImageLoaded(Arc<ImageData>, Option<PathBuf>),
    Rotate(i32),
    FlipHorizontal,
    FlipVertical,
    Resize(Vec<Image>),
    Color(Vec<Image>),
    Crop(Vec<Image>, i32),
    ColorSpace(Vec<Image>),
    Undo,
    Redo,
    Close,
    Saved,
    // these are just used to indicate that it is done
    Done,
}

pub trait UserEventLoopProxyExt {
    fn send_output(&self, output: Output);
}

impl UserEventLoopProxyExt for EventLoopProxy<UserEvent> {
    fn send_output(&self, output: Output) {
        let _ = self.send_event(UserEvent::Output(Some(output)));
    }
}

#[derive(Default)]
pub struct LoadingInfo {
    target_file: Option<PathBuf>,
    loading: HashSet<PathBuf>,
}

pub struct OpQueue {
    working: bool,
    loading_info: Arc<Mutex<LoadingInfo>>,
    proxy: EventLoopProxy<UserEvent>,
    dialog_proxy: DialogProxy,
    stack: UndoStack,
    pub cache: Arc<Cache>,
    pub image_list: ImageList,
}

impl OpQueue {
    pub fn new(
        proxy: EventLoopProxy<UserEvent>,
        dialog_proxy: DialogProxy,
        no_cache: bool,
    ) -> Self {
        const CACHE_SIZE: usize = 1_000_000_000;
        let cache = Arc::new(Cache::new(if no_cache { 0 } else { CACHE_SIZE }));
        let loading_info = Arc::new(Mutex::new(LoadingInfo::default()));

        Self {
            working: false,
            image_list: ImageList::new(cache.clone(), proxy.clone(), loading_info.clone()),
            loading_info,
            dialog_proxy,
            stack: UndoStack::new(),
            proxy,
            cache,
        }
    }

    pub fn queue(&mut self, op: Op, view: Option<&ImageView>) {
        if !self.working {
            self.working = true;
            match op {
                Op::LoadPath(path, use_cache) => {
                    self.load(path, use_cache, self.stack.is_edited());
                }
                Op::LoadBytes(bytes) => self.load_from_bytes(bytes),

                Op::Next => match self.image_list.next() {
                    Some(path) => {
                        self.load(path, true, self.stack.is_edited());
                    }
                    None => {
                        self.proxy.send_output(Output::Done);
                    }
                },
                Op::Prev => match self.image_list.prev() {
                    Some(path) => {
                        self.load(path, true, self.stack.is_edited());
                    }
                    None => {
                        self.proxy.send_output(Output::Done);
                    }
                },
                Op::Save(path) => {
                    if let Some(view) = view {
                        save_image::save(
                            self.proxy.clone(),
                            self.dialog_proxy.clone(),
                            path,
                            view.image_data.clone(),
                            view.rotation(),
                            view.horizontal_flip,
                            view.vertical_flip,
                        )
                    }
                }
                Op::Rotate(dir) => {
                    self.proxy.send_output(Output::Rotate(dir));
                }
                Op::FlipHorizontal => {
                    self.proxy.send_output(Output::FlipHorizontal);
                }
                Op::FlipVertical => {
                    self.proxy.send_output(Output::FlipVertical);
                }
                Op::Undo => {
                    self.proxy.send_output(Output::Undo);
                }
                Op::Redo => {
                    self.proxy.send_output(Output::Redo);
                }
                Op::Close => {
                    let proxy = self.proxy.clone();
                    let dialog_proxy = self.dialog_proxy.clone();
                    if self.stack.is_edited() {
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
                                proxy.send_output(Output::Close);
                            } else {
                                proxy.send_output(Output::Done);
                            }
                        });
                    } else {
                        proxy.send_output(Output::Close);
                    }
                }
                Op::ColorSpace(color_type) => {
                    let image_data = view.as_ref().unwrap().image_data.clone();
                    let proxy = self.proxy.clone();
                    thread::spawn(move || {
                        let guard = image_data.read().unwrap();
                        let mut new = Vec::new();
                        for image in guard.frames.iter() {
                            let buffer = image.buffer();
                            let buffer = match color_type {
                                ColorType::L8 => DynamicImage::ImageLuma8(buffer.to_luma8()),
                                ColorType::La8 => {
                                    DynamicImage::ImageLumaA8(buffer.to_luma_alpha8())
                                }
                                ColorType::Rgb8 => DynamicImage::ImageRgb8(buffer.to_rgb8()),
                                ColorType::Rgba8 => DynamicImage::ImageRgba8(buffer.to_rgba8()),
                                ColorType::L16 => DynamicImage::ImageLuma16(buffer.to_luma16()),
                                ColorType::La16 => {
                                    DynamicImage::ImageLumaA16(buffer.to_luma_alpha16())
                                }
                                ColorType::Rgb16 => DynamicImage::ImageRgb16(buffer.to_rgb16()),
                                ColorType::Rgba16 => DynamicImage::ImageRgba16(buffer.to_rgba16()),
                                ColorType::Rgb32F => DynamicImage::ImageRgb32F(buffer.to_rgb32f()),
                                ColorType::Rgba32F => {
                                    DynamicImage::ImageRgba32F(buffer.to_rgba32f())
                                }
                                _ => panic!("unknown color type this a bug"),
                            };
                            new.push(Image::with_delay(buffer, image.delay));
                        }
                        proxy.send_output(Output::ColorSpace(new));
                    });
                }
                Op::Resize(size, resample) => {
                    let image_data = view.as_ref().unwrap().image_data.clone();
                    let proxy = self.proxy.clone();
                    thread::spawn(move || {
                        let guard = image_data.read().unwrap();
                        let mut new = Vec::new();
                        for image in guard.frames.iter() {
                            let buffer = image.buffer().resize_exact(size.x, size.y, resample);
                            new.push(Image::with_delay(buffer, image.delay));
                        }
                        proxy.send_output(Output::Resize(new));
                    });
                }
                Op::Color {
                    hue,
                    saturation,
                    contrast,
                    brightness,
                    grayscale,
                    invert,
                } => {
                    let image_data = view.as_ref().unwrap().image_data.clone();
                    let proxy = self.proxy.clone();
                    thread::spawn(move || {
                        let guard = image_data.read().unwrap();
                        let mut new: Vec<_> = guard.frames.clone();
                        drop(guard);
                        for image in &mut new {
                            let buffer = image.buffer_mut();

                            if hue != 0.0 {
                                huerotate_in_place(buffer, hue as i32);
                            }

                            if contrast != 0.0 {
                                contrast_in_place(buffer, contrast);
                            }

                            if saturation != 0.0 {
                                adjust_saturation_in_place(buffer, saturation as f64);
                            }

                            if brightness != 0.0 {
                                brighten_in_place(buffer, brightness as f64);
                            }

                            if grayscale {
                                *buffer = DynamicImage::ImageLumaA8(imageops::grayscale(buffer));
                            }

                            if invert {
                                buffer.invert();
                            }
                        }
                        proxy.send_output(Output::Color(new));
                    });
                }
                Op::Crop(rect) => {
                    view.unwrap().crop(rect, self.proxy.clone());
                }
                Op::Copy => {
                    clipboard::copy(view.unwrap(), self.proxy.clone());
                }
                Op::Paste => {
                    clipboard::paste(self.proxy.clone());
                }
                Op::Delete(path) => match trash::delete(&path) {
                    Ok(_) => match self.image_list.trash(&path) {
                        Some(path) => {
                            self.load(path, true, false);
                        }
                        None => {
                            self.proxy.send_output(Output::Close);
                        }
                    },
                    Err(error) => {
                        let _ = self
                            .proxy
                            .send_event(UserEvent::ErrorMessage(error.to_string()));
                    }
                },
            }
        }
    }

    pub fn undo_stack_mut(&mut self) -> &mut UndoStack {
        &mut self.stack
    }

    pub fn undo_stack(&self) -> &UndoStack {
        &self.stack
    }

    fn load(&self, path_buf: PathBuf, use_cache: bool, edited_prompt: bool) {
        let path_buf = match path_buf.strip_prefix("file://") {
            Ok(path) => Path::new("/").join(path),
            Err(_) => path_buf,
        };

        {
            let mut guard = self.loading_info.lock().unwrap();
            guard.target_file = Some(path_buf.clone());
            if guard.loading.contains(&path_buf) {
                return;
            } else {
                guard.loading.insert(path_buf.clone());
            }
        }

        if !use_cache {
            self.cache.pop(&path_buf);
        }

        let cache = self.cache.clone();
        let proxy = self.proxy.clone();
        let loading_info = self.loading_info.clone();
        let dialog_proxy = self.dialog_proxy.clone();
        thread::spawn(move || {
            let mut path = path_buf.clone();
            if path.is_dir() {
                let mut paths = Vec::new();
                if let Ok(dir) = fs::read_dir(&path) {
                    for entry in dir.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() {
                                let path = entry.path();
                                match path.extension() {
                                    Some(ext)
                                        if EXTENSIONS.contains(
                                            &&*ext.to_string_lossy().to_ascii_lowercase(),
                                        ) =>
                                    {
                                        paths.push(path);
                                    }
                                    _ => (),
                                }
                            }
                        }
                    }
                }
                paths.sort();
                if let Some(p) = paths.into_iter().next() {
                    path = p;
                }
            }

            if edited_prompt {
                let close = dialog_proxy
                    .spawn_dialog("Unsaved changes", move |ui, enter| {
                        ui.label(
                            "You have unsaved changes are you sure you want to close this image?",
                        );

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT), |ui| {
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
                        })
                        .inner
                    })
                    .wait()
                    .unwrap_or(false);
                if !close {
                    proxy.send_output(Output::Done);
                    return;
                }
            }

            if let Some(images) = cache.get(&path_buf) {
                let mut guard = loading_info.lock().unwrap();
                guard.loading.remove(&path_buf);
                guard.target_file = None;
                proxy.send_output(Output::ImageLoaded(images, Some(path_buf)));
                return;
            }

            let res = load_uncached(&path);
            let mut guard = loading_info.lock().unwrap();
            guard.loading.remove(&path_buf);
            guard.target_file = None;

            match res {
                Ok(images) => {
                    let images = Arc::new(images);
                    cache.put(path_buf.clone(), images.clone());
                    proxy.send_output(Output::ImageLoaded(images, Some(path)));
                }
                Err(error) => {
                    proxy.send_output(Output::Done);
                    let _ = proxy.send_event(UserEvent::ErrorMessage(error.to_string()));
                }
            };
        });
    }

    // should only be used to load first image from stdin
    fn load_from_bytes(&self, bytes: Vec<u8>) {
        let proxy = self.proxy.clone();
        thread::spawn(move || {
            let res = load_from_bytes(&bytes, None);
            match res {
                Ok(images) => {
                    let images = Arc::new(images);
                    proxy.send_output(Output::ImageLoaded(images, None));
                }
                Err(error) => {
                    proxy.send_output(Output::Done);
                    let _ = proxy.send_event(UserEvent::ErrorMessage(error.to_string()));
                }
            };
        });
    }

    pub fn working(&self) -> bool {
        self.working
    }

    pub fn set_working(&mut self, working: bool) {
        self.working = working;
    }
}

pub fn prefetch(
    path: impl AsRef<Path>,
    cache: Arc<Cache>,
    proxy: EventLoopProxy<UserEvent>,
    loading_info: Arc<Mutex<LoadingInfo>>,
) {
    let path_buf = path.as_ref().to_path_buf();

    if cache.contains(&path_buf) {
        return;
    }

    {
        let mut guard = loading_info.lock().unwrap();
        if guard.loading.contains(&path_buf) {
            return;
        } else {
            guard.loading.insert(path_buf.clone());
        }
    }

    thread::spawn(move || {
        let res = load_uncached(&path_buf);
        let mut guard = loading_info.lock().unwrap();
        guard.loading.remove(&path_buf);

        match res {
            Ok(images) => {
                let images = Arc::new(images);
                cache.put(path_buf.clone(), images.clone());
                if guard.target_file.as_ref() == Some(&path_buf) {
                    proxy.send_output(Output::ImageLoaded(images, Some(path_buf.clone())));
                }
            }
            Err(error) => {
                if guard.target_file.as_ref() == Some(&path_buf) {
                    let _ = proxy.send_event(UserEvent::ErrorMessage(error.to_string()));
                }
            }
        };

        if guard.target_file.as_ref() == Some(&path_buf) {
            guard.target_file = None;
        }
    });
}
