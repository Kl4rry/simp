use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use lru::LruCache;

use crate::util::Image;

pub struct Cache {
    lru: Mutex<LruCache<PathBuf, Arc<RwLock<Vec<Image>>>>>,
    total_size: AtomicUsize,
    max_size: usize,
}

impl Cache {
    pub fn new(max_size: usize) -> Cache {
        Self {
            lru: Mutex::new(LruCache::new(100)),
            total_size: AtomicUsize::new(0),
            max_size,
        }
    }

    pub fn put(&self, path: PathBuf, image: Arc<RwLock<Vec<Image>>>) {
        let size: usize = image
            .read()
            .unwrap()
            .iter()
            .map(|image| image.buffer().as_bytes().len())
            .sum();
        if size >= self.max_size {
            return;
        }

        let mut guard = self.lru.lock().unwrap();
        while size + self.total_size.load(Ordering::SeqCst) > self.max_size {
            let removed = guard.pop_lru();
            match removed {
                Some((_, value)) => {
                    let removed: usize = value
                        .read()
                        .unwrap()
                        .iter()
                        .map(|image| image.buffer().as_bytes().len())
                        .sum();
                    self.total_size.fetch_sub(removed, Ordering::SeqCst);
                }
                None => break,
            }
        }

        self.total_size.fetch_add(size, Ordering::SeqCst);
        guard.put(path, image);
    }

    #[allow(clippy::ptr_arg)]
    pub fn get(&self, path: &PathBuf) -> Option<Arc<RwLock<Vec<Image>>>> {
        self.lru.lock().unwrap().get(path).cloned()
    }

    #[allow(clippy::ptr_arg)]
    pub fn contains(&self, path: &PathBuf) -> bool {
        self.lru.lock().unwrap().contains(path)
    }

    pub fn clear(&self) {
        self.total_size.store(0, Ordering::SeqCst);
        self.lru.lock().unwrap().clear();
    }
}
