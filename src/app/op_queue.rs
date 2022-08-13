use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, RwLock,
    },
    thread,
};

use glium::glutin::event_loop::EventLoopProxy;
use image::{imageops::FilterType, DynamicImage};

use self::imageops::{adjust_saturation, lighten};
use super::{
    cache::Cache, clipboard, image_list::ImageList, image_view::ImageView,
    load_image::load_uncached, save_image,
};
use crate::{
    app::undo_stack::UndoStack,
    rect::Rect,
    util::{Image, ImageData, UserEvent},
    vec2::Vec2,
};

mod imageops;

#[derive(Debug)]
pub enum Op {
    LoadPath(PathBuf, bool),
    Next,
    Prev,
    Save(PathBuf),
    Resize(Vec2<u32>, FilterType),
    Color {
        hue: f32,
        saturation: f32,
        contrast: f32,
        lightness: f32,
        grayscale: bool,
        invert: bool,
    },
    Crop(Rect),
    FlipHorizontal,
    FlipVertical,
    Rotate(i32),
    Undo,
    Redo,
    Close,
    Copy,
    Paste,
}

pub enum Output {
    ImageLoaded(Arc<RwLock<ImageData>>, Option<PathBuf>),
    Rotate(i32),
    FlipHorizontal,
    FlipVertical,
    Resize(Vec<Image>),
    Color(Vec<Image>),
    Crop(Vec<Image>, i32),
    Undo,
    Redo,
    Close,

    // these are just used to indicate that it is done
    Done,
}

#[derive(Default)]
pub struct LoadingInfo {
    target_file: Option<PathBuf>,
    loading: HashSet<PathBuf>,
}

pub struct OpQueue {
    working: bool,
    loading_info: Arc<Mutex<LoadingInfo>>,
    sender: Sender<Output>,
    receiver: Receiver<Output>,
    proxy: EventLoopProxy<UserEvent>,
    stack: UndoStack,
    pub cache: Arc<Cache>,
    pub image_list: ImageList,
}

impl OpQueue {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let (sender, receiver) = mpsc::channel();

        const CACHE_SIZE: usize = 1_000_000_000;
        let cache = Arc::new(Cache::new(CACHE_SIZE));
        let loading_info = Arc::new(Mutex::new(LoadingInfo::default()));

        Self {
            working: false,
            image_list: ImageList::new(
                cache.clone(),
                proxy.clone(),
                sender.clone(),
                loading_info.clone(),
            ),
            loading_info,
            sender,
            receiver,
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
                    self.load(path, use_cache);
                }
                Op::Next => match self.image_list.next() {
                    Some(path) => {
                        self.load(path, true);
                    }
                    None => {
                        let _ = self.sender.send(Output::Done);
                        let _ = self.proxy.send_event(UserEvent::Wake);
                    }
                },
                Op::Prev => match self.image_list.prev() {
                    Some(path) => {
                        self.load(path, true);
                    }
                    None => {
                        let _ = self.sender.send(Output::Done);
                        let _ = self.proxy.send_event(UserEvent::Wake);
                    }
                },
                Op::Save(path) => {
                    if let Some(view) = view {
                        save_image::save(
                            self.proxy.clone(),
                            self.sender.clone(),
                            path,
                            view.image_data.clone(),
                            view.rotation(),
                            view.horizontal_flip,
                            view.vertical_flip,
                        )
                    }
                }
                Op::Rotate(dir) => {
                    let _ = self.sender.send(Output::Rotate(dir));
                    let _ = self.proxy.send_event(UserEvent::Wake);
                }
                Op::FlipHorizontal => {
                    let _ = self.sender.send(Output::FlipHorizontal);
                    let _ = self.proxy.send_event(UserEvent::Wake);
                }
                Op::FlipVertical => {
                    let _ = self.sender.send(Output::FlipVertical);
                    let _ = self.proxy.send_event(UserEvent::Wake);
                }
                Op::Undo => {
                    let _ = self.sender.send(Output::Undo);
                    let _ = self.proxy.send_event(UserEvent::Wake);
                }
                Op::Redo => {
                    let _ = self.sender.send(Output::Redo);
                    let _ = self.proxy.send_event(UserEvent::Wake);
                }
                Op::Close => {
                    let _ = self.sender.send(Output::Close);
                    let _ = self.proxy.send_event(UserEvent::Wake);
                }
                Op::Resize(size, resample) => {
                    let image_data = view.as_ref().unwrap().image_data.clone();
                    let proxy = self.proxy.clone();
                    let sender = self.sender.clone();
                    thread::spawn(move || {
                        let guard = image_data.read().unwrap();
                        let mut new = Vec::new();
                        for image in guard.frames.iter() {
                            let buffer = image.buffer().resize_exact(size.x(), size.y(), resample);
                            new.push(Image::with_delay(buffer, image.delay));
                        }
                        let _ = sender.send(Output::Resize(new));
                        let _ = proxy.send_event(UserEvent::Wake);
                    });
                }
                Op::Color {
                    hue,
                    saturation,
                    contrast,
                    lightness,
                    grayscale,
                    invert,
                } => {
                    let image_data = view.as_ref().unwrap().image_data.clone();
                    let proxy = self.proxy.clone();
                    let sender = self.sender.clone();
                    thread::spawn(move || {
                        let guard = image_data.read().unwrap();
                        let mut new = Vec::new();
                        for image in guard.frames.iter() {
                            let mut buffer = image
                                .buffer()
                                .huerotate(hue as i32)
                                .adjust_contrast(contrast);

                            if saturation != 0.0 {
                                buffer = DynamicImage::from(lighten(buffer, lightness as f64));
                            }

                            if lightness != 0.0 {
                                buffer = DynamicImage::from(adjust_saturation(
                                    buffer,
                                    saturation as f64,
                                ));
                            }

                            if grayscale {
                                buffer = DynamicImage::ImageLumaA8(imageops::grayscale(&buffer));
                            }

                            if invert {
                                buffer.invert();
                            }

                            new.push(Image::with_delay(buffer, image.delay));
                        }
                        let _ = sender.send(Output::Color(new));
                        let _ = proxy.send_event(UserEvent::Wake);
                    });
                }
                Op::Crop(rect) => {
                    view.unwrap()
                        .crop(rect, self.proxy.clone(), self.sender.clone());
                }
                Op::Copy => {
                    clipboard::copy(view.unwrap(), self.proxy.clone(), self.sender.clone());
                }
                Op::Paste => {
                    clipboard::paste(self.proxy.clone(), self.sender.clone());
                }
            }
        }
    }

    pub fn poll(&mut self) -> Option<(Output, &mut UndoStack)> {
        match self.receiver.try_recv() {
            Ok(output) => {
                self.working = false;
                Some((output, &mut self.stack))
            }
            Err(_) => None,
        }
    }

    fn load(&self, path_buf: PathBuf, use_cache: bool) {
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

        if let Some(images) = self.cache.get(&path_buf) {
            let mut guard = self.loading_info.lock().unwrap();
            guard.loading.remove(&path_buf);
            guard.target_file = None;
            self.sender
                .send(Output::ImageLoaded(images, Some(path_buf)))
                .unwrap();
            let _ = self.proxy.send_event(UserEvent::Wake);
            return;
        }

        let cache = self.cache.clone();
        let sender = self.sender.clone();
        let proxy = self.proxy.clone();
        let loading_info = self.loading_info.clone();
        thread::spawn(move || {
            let res = load_uncached(&path_buf);
            let mut guard = loading_info.lock().unwrap();
            guard.loading.remove(&path_buf);
            guard.target_file = None;

            match res {
                Ok(images) => {
                    let images = Arc::new(RwLock::new(images));
                    cache.put(path_buf.clone(), images.clone());
                    sender
                        .send(Output::ImageLoaded(images, Some(path_buf)))
                        .unwrap();
                    let _ = proxy.send_event(UserEvent::Wake);
                }
                Err(error) => {
                    let _ = sender.send(Output::Done);
                    let _ = proxy.send_event(UserEvent::ErrorMessage(error.to_string()));
                }
            };
        });
    }

    pub fn working(&self) -> bool {
        self.working
    }
}

pub fn prefetch(
    path: impl AsRef<Path>,
    cache: Arc<Cache>,
    proxy: EventLoopProxy<UserEvent>,
    sender: Sender<Output>,
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
                let images = Arc::new(RwLock::new(images));
                cache.put(path_buf.clone(), images.clone());
                if guard.target_file.as_ref() == Some(&path_buf) {
                    sender
                        .send(Output::ImageLoaded(images, Some(path_buf.clone())))
                        .unwrap();
                    let _ = proxy.send_event(UserEvent::Wake);
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
