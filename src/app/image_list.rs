// use std::{
//     path::{Path, PathBuf},
//     sync::{
//         atomic::{AtomicUsize, Ordering},
//         Arc, Mutex, RwLock,
//     },
//     thread,
// };

// use glium::glutin::event_loop::EventLoopProxy;

// use crate::{
//     app::{Cache},
//     util::{extensions::*, UserEvent},
// };

// type List = Arc<Mutex<Option<Vec<PathBuf>>>>;

// pub struct ImageList {
//     list: List,
//     index: Arc<AtomicUsize>,
//     path: Option<PathBuf>,
//     cache: Arc<Cache>,
//     proxy: EventLoopProxy<UserEvent>,
//     loader: Arc<RwLock<ImageLoader>>,
// }

// impl ImageList {
//     pub fn new(
//         cache: Arc<Cache>,
//         proxy: EventLoopProxy<UserEvent>,
//         loader: Arc<RwLock<ImageLoader>>,
//     ) -> Self {
//         Self {
//             list: Arc::new(Mutex::new(None)),
//             index: Arc::new(AtomicUsize::new(0)),
//             path: None,
//             proxy,
//             cache,
//             loader,
//         }
//     }

//     pub fn clear(&mut self) {
//         *self.list.lock().unwrap() = None;
//         self.path = None;
//         self.index.store(0, Ordering::SeqCst)
//     }

//     pub fn change_dir(&mut self, path: impl AsRef<Path>) {
//         let path_buf = path.as_ref().to_path_buf();
//         let mut dir_path = path_buf.clone();
//         dir_path.pop();

//         if let Some(ref p) = self.path {
//             if *p == dir_path && self.list.lock().unwrap().is_some() {
//                 let lock = self.list.lock().unwrap();
//                 if let Some(ref dirs) = *lock {
//                     for (index, path) in dirs.iter().enumerate() {
//                         if *path == path_buf {
//                             self.index.store(index, Ordering::SeqCst);
//                         }
//                     }
//                 }
//                 return;
//             }
//         }

//         self.path = Some(dir_path.clone());

//         let t_list = self.list.clone();
//         let t_index = self.index.clone();
//         let proxy = self.proxy.clone();
//         let cache = self.cache.clone();
//         let loader = self.loader.clone();
//         thread::spawn(move || {
//             let mut list = Vec::new();
//             let dirs = std::fs::read_dir(dir_path).unwrap();

//             for dir in dirs.flatten() {
//                 if let Ok(file_type) = dir.file_type() {
//                     if file_type.is_file() {
//                         let path = dir.path();
//                         match dir.path().extension() {
//                             Some(ext)
//                                 if EXTENSIONS
//                                     .contains(&*ext.to_string_lossy().to_ascii_lowercase()) =>
//                             {
//                                 list.push(path)
//                             }
//                             _ => (),
//                         }
//                     }
//                 }
//             }

//             list.sort_by(|a, b| b.cmp(a));

//             for (index, path) in list.iter().enumerate() {
//                 if *path == path_buf {
//                     t_index.store(index, Ordering::SeqCst);
//                 }
//             }

//             let next = list[next_index(t_index.load(Ordering::SeqCst), list.len())].clone();
//             prefetch(proxy.clone(), next, cache.clone(), loader.clone());

//             let prev = list[prev_index(t_index.load(Ordering::SeqCst), list.len())].clone();
//             prefetch(proxy.clone(), prev, cache.clone(), loader.clone());

//             *t_list.lock().unwrap() = Some(list);
//         });
//     }

//     pub fn next(&mut self) -> Option<PathBuf> {
//         let lock = self.list.lock().unwrap();
//         if let Some(ref list) = *lock {
//             self.index.fetch_add(1, Ordering::SeqCst);
//             if list.len() <= self.index.load(Ordering::SeqCst) {
//                 self.index.store(0, Ordering::SeqCst);
//             }
//             prefetch(
//                 self.proxy.clone(),
//                 list[next_index(self.index.load(Ordering::SeqCst), list.len())].clone(),
//                 self.cache.clone(),
//                 self.loader.clone(),
//             );
//             Some(list[self.index.load(Ordering::SeqCst)].clone())
//         } else {
//             None
//         }
//     }

//     pub fn previous(&mut self) -> Option<PathBuf> {
//         let lock = self.list.lock().unwrap();
//         if let Some(ref list) = *lock {
//             if self.index.load(Ordering::SeqCst) == 0 {
//                 self.index.store(list.len() - 1, Ordering::SeqCst);
//             } else {
//                 self.index.fetch_sub(1, Ordering::SeqCst);
//             }
//             prefetch(
//                 self.proxy.clone(),
//                 list[prev_index(self.index.load(Ordering::SeqCst), list.len())].clone(),
//                 self.cache.clone(),
//                 self.loader.clone(),
//             );
//             Some(list[self.index.load(Ordering::SeqCst)].clone())
//         } else {
//             None
//         }
//     }
// }

// fn next_index(index: usize, len: usize) -> usize {
//     let next = index + 1;
//     if len <= next {
//         0
//     } else {
//         next
//     }
// }

// fn prev_index(index: usize, len: usize) -> usize {
//     let current = index;
//     if current == 0 {
//         len - 1
//     } else {
//         current - 1
//     }
// }
