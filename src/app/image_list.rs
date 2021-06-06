use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
};

use lazy_static::*;

type List = Arc<Mutex<Option<Vec<PathBuf>>>>;

pub struct ImageList {
    list: List,
    index: Arc<AtomicUsize>,
    path: Option<PathBuf>,
}

lazy_static! {
    static ref EXTENSIONS: HashSet<&'static str> = [
        "png", "jpg", "gif", "bmp", "ico", "tiff", "webp", "avif", "pnm", "pbm", "pgm", "ppm",
        "pam", "dds", "tga", "ff", "svg",
    ]
    .iter()
    .cloned()
    .collect();
}

impl ImageList {
    pub fn new() -> Self {
        Self {
            list: Arc::new(Mutex::new(None)),
            index: Arc::new(AtomicUsize::new(0)),
            path: None,
        }
    }

    pub fn change_dir(&mut self, path: impl AsRef<Path>) {
        let path_buf = path.as_ref().to_path_buf();
        let mut dir_path = path_buf.clone();
        dir_path.pop();

        if let Some(ref p) = self.path {
            if *p == dir_path && self.list.lock().unwrap().is_some() {
                return;
            }
        }

        self.path = Some(dir_path.clone());

        let t_list = self.list.clone();
        let t_index = self.index.clone();
        thread::spawn(move || {
            let mut list = Vec::new();
            let dirs = std::fs::read_dir(dir_path).unwrap();

            for (index, result) in dirs.enumerate() {
                if let Ok(dir) = result {
                    if let Ok(file_type) = dir.file_type() {
                        if file_type.is_file() {
                            let path = dir.path();
                            if path == path_buf {
                                t_index.store(index, Ordering::SeqCst);
                            }
                            match dir.path().extension() {
                                Some(ext) if EXTENSIONS.contains(ext.to_str().unwrap()) => {
                                    list.push(path)
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
            *t_list.lock().unwrap() = Some(list);
        });
    }

    pub fn next(&mut self) -> Option<PathBuf> {
        let lock = self.list.lock().unwrap();
        if let Some(ref list) = *lock {
            self.index.fetch_add(1, Ordering::SeqCst);
            if list.len() <= self.index.load(Ordering::SeqCst) {
                self.index.store(0, Ordering::SeqCst);
            }
            Some(list[self.index.load(Ordering::SeqCst)].clone())
        } else {
            None
        }
    }

    pub fn previous(&mut self) -> Option<PathBuf> {
        let lock = self.list.lock().unwrap();
        if let Some(ref list) = *lock {
            if self.index.load(Ordering::SeqCst) == 0 {
                self.index.store(list.len() - 1, Ordering::SeqCst);
            } else {
                self.index.fetch_sub(1, Ordering::SeqCst);
            }
            Some(list[self.index.load(Ordering::SeqCst)].clone())
        } else {
            None
        }
    }
}
