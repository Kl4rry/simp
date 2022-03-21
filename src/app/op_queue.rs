use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, RwLock,
    },
    thread,
};

use glium::{
    glutin::{event_loop::EventLoopProxy, window::CursorIcon},
    Display,
};
use image::imageops::FilterType;

use super::{
    cache::Cache, clipboard, cursor, image_view::ImageView, load_image::load_uncached, save_image,
};
use crate::{
    app::undo_stack::UndoStack,
    rect::Rect,
    util::{Image, ImageData, UserEvent},
    vec2::Vec2,
};

#[derive(Debug)]
pub enum Op {
    LoadPath(PathBuf, bool),
    Save(PathBuf),
    Resize(Vec2<u32>, FilterType),
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
    Crop(Vec<Image>, i32),
    Undo,
    Redo,
    Close,

    // these are just used to indicate that it is done
    Done,
}

#[derive(Default)]
struct LoadingInfo {
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
}

impl OpQueue {
    pub fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let (sender, receiver) = mpsc::channel();

        const CACHE_SIZE: usize = 1_000_000_000;
        Self {
            working: false,
            loading_info: Arc::new(Mutex::new(LoadingInfo::default())),
            sender,
            receiver,
            proxy,
            stack: UndoStack::new(),
            cache: Arc::new(Cache::new(CACHE_SIZE)),
        }
    }

    pub fn queue(&mut self, op: Op, view: Option<&Box<ImageView>>) {
        if !self.working {
            self.working = true;
            match op {
                Op::LoadPath(path, use_cache) => {
                    let _ = self
                        .proxy
                        .send_event(UserEvent::SetCursor(CursorIcon::Progress));
                    self.load(path, use_cache);
                }
                Op::Save(path) => {
                    let _ = self
                        .proxy
                        .send_event(UserEvent::SetCursor(CursorIcon::Progress));
                    if let Some(view) = view {
                        save_image::save(
                            self.proxy.clone(),
                            self.sender.clone(),
                            path,
                            view.image_data.clone(),
                            view.rotation,
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
                    let _ = self
                        .proxy
                        .send_event(UserEvent::SetCursor(CursorIcon::Progress));
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
                Op::Crop(rect) => {
                    let _ = self
                        .proxy
                        .send_event(UserEvent::SetCursor(CursorIcon::Progress));
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

    pub fn poll(&mut self, display: &Display) -> Option<(Output, &mut UndoStack)> {
        match self.receiver.try_recv() {
            Ok(output) => {
                self.working = false;
                cursor::set_cursor_icon(CursorIcon::default(), display);
                return Some((output, &mut self.stack));
            }
            Err(_) => None,
        }
    }

    pub fn _prefetch(&self, path: impl AsRef<Path>) {
        let path_buf = path.as_ref().to_path_buf();

        if self.cache.contains(&path_buf) {
            return;
        }

        {
            let mut guard = self.loading_info.lock().unwrap();
            if guard.loading.contains(&path_buf) {
                return;
            } else {
                guard.loading.insert(path_buf.clone());
            }
        }

        let cache = self.cache.clone();
        let sender = self.sender.clone();
        let proxy = self.proxy.clone();
        let loading_info = self.loading_info.clone();
        thread::spawn(move || {
            let res = load_uncached(&path_buf);
            let mut guard = loading_info.lock().unwrap();
            guard.loading.remove(&path_buf);
            if guard.target_file.as_ref() == Some(&path_buf) {
                guard.target_file = None;
            }

            match res {
                Ok(images) => {
                    let images = Arc::new(RwLock::new(images));
                    cache.put(path_buf.clone(), images.clone());
                    if guard.target_file.as_ref() == Some(&path_buf) {
                        sender
                            .send(Output::ImageLoaded(images, Some(path_buf)))
                            .unwrap();
                        let _ = proxy.send_event(UserEvent::Wake);
                    }
                }
                Err(error) => {
                    let _ = proxy.send_event(UserEvent::ErrorMessage(error.to_string()));
                }
            };
        });
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
            self.sender
                .send(Output::ImageLoaded(images, Some(path_buf)))
                .unwrap();
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
                    let _ = proxy.send_event(UserEvent::ErrorMessage(error.to_string()));
                }
            };
        });
    }

    pub fn working(&self) -> bool {
        self.working
    }
}
