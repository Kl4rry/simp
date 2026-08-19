use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, SystemTime},
};

use winit::event_loop::EventLoopProxy;

use super::op_queue::{LoadingInfo, prefetch};
use crate::{
    app::{cache::Cache, preferences::SortOrder},
    util::{UserEvent, extensions::*},
};

#[derive(Default, Clone)]
struct ImageListEntry {
    path: PathBuf,
    accessed: Option<SystemTime>,
    created: Option<SystemTime>,
    modified: Option<SystemTime>,
    exif_date: Option<SystemTime>,
    size: u64,
}

impl ImageListEntry {
    pub fn from_path(path: PathBuf) -> ImageListEntry {
        let mut output = Self::default();
        if let Ok(metadata) = std::fs::metadata(&path) {
            output.accessed = metadata.accessed().ok();
            output.created = metadata.created().ok();
            output.modified = metadata.modified().ok();
            output.size = metadata.len();
        }

        if let Ok(exif) = rexif::parse_file(&path) {
            for entry in exif.entries {
                if rexif::ExifTag::DateTime != entry.tag {
                    continue;
                }
                let value = entry.value_more_readable;
                let format = time::macros::format_description!(
                    "[year]:[month]:[day] [hour]:[minute]:[second]"
                );
                let Ok(datetime) = time::PlainDateTime::parse(&value, &format) else {
                    continue;
                };
                let seconds_since_unix_epoch = datetime
                    .assume_offset(time::macros::offset!(UTC))
                    .unix_timestamp();
                if seconds_since_unix_epoch < 0 {
                    continue;
                }
                let systemtime =
                    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds_since_unix_epoch as u64);
                output.exif_date = Some(systemtime);
            }
        }

        output.path = path;

        output
    }
}

impl PartialEq for ImageListEntry {
    fn eq(&self, other: &ImageListEntry) -> bool {
        self.path == other.path
    }
}

type List = Arc<Mutex<Option<Vec<ImageListEntry>>>>;

pub struct ImageList {
    list: List,
    index: Arc<AtomicUsize>,
    path: Option<PathBuf>,
    cache: Arc<Cache>,
    proxy: EventLoopProxy<UserEvent>,
    loading_info: Arc<Mutex<LoadingInfo>>,
    sort_order: Arc<Mutex<SortOrder>>,
}

impl ImageList {
    pub fn new(
        cache: Arc<Cache>,
        proxy: EventLoopProxy<UserEvent>,
        loading_info: Arc<Mutex<LoadingInfo>>,
    ) -> Self {
        Self {
            list: Arc::new(Mutex::new(None)),
            index: Arc::new(AtomicUsize::new(0)),
            path: None,
            proxy,
            cache,
            loading_info,
            sort_order: Arc::new(Mutex::new(SortOrder::default())),
        }
    }

    pub fn clear(&mut self) {
        *self.list.lock().unwrap() = None;
        self.path = None;
        self.index.store(0, Ordering::SeqCst)
    }

    pub fn set_sort_order(&mut self, sort_order: SortOrder) {
        {
            let mut lock = self.sort_order.lock().unwrap();
            if *lock == sort_order {
                return;
            }
            *lock = sort_order;
        }

        let index = self.index.clone();
        let list = self.list.clone();
        thread::spawn(move || {
            let current = index.load(Ordering::SeqCst);
            let mut lock = list.lock().unwrap();
            let Some(ref mut list) = *lock else {
                return;
            };
            let current_entry = list[current].clone();
            sort_list(list, sort_order);
            for (i, list_entry) in list.iter().enumerate() {
                if list_entry.path == current_entry.path {
                    index.store(i, Ordering::SeqCst);
                }
            }
        });
    }

    pub fn change_dir(&mut self, path: impl AsRef<Path>) {
        if let Some(ref current) = self.path {
            let new = path.as_ref().parent();
            if let Some(new) = new
                && current == new
            {
                return;
            }
        }
        let path_buf = path.as_ref().to_path_buf();
        let mut dir_path = path_buf.clone();
        dir_path.pop();
        if dir_path == Path::new("") {
            dir_path = PathBuf::from(".");
        }

        if let Some(ref p) = self.path
            && *p == dir_path
            && self.list.lock().unwrap().is_some()
        {
            let lock = self.list.lock().unwrap();
            if let Some(ref dirs) = *lock {
                for (index, list_entry) in dirs.iter().enumerate() {
                    if list_entry.path == path_buf {
                        self.index.store(index, Ordering::SeqCst);
                    }
                }
            }
            return;
        }

        self.path = Some(dir_path.clone());

        let t_list = self.list.clone();
        let t_index = self.index.clone();
        let t_sort_order = self.sort_order.clone();
        let proxy = self.proxy.clone();
        let cache = self.cache.clone();
        let loading_info = self.loading_info.clone();
        let mut list = vec![ImageListEntry::from_path(path_buf.clone())];
        thread::spawn(move || {
            let dirs = std::fs::read_dir(dir_path).unwrap();

            for dir in dirs.flatten() {
                if let Ok(file_type) = dir.file_type()
                    && file_type.is_file()
                {
                    let path = dir.path();

                    match dir.path().extension() {
                        Some(ext)
                            if EXTENSIONS
                                .contains(&&*ext.to_string_lossy().to_ascii_lowercase()) =>
                        {
                            list.push(ImageListEntry::from_path(path))
                        }
                        _ => (),
                    }
                }
            }

            sort_list(&mut list, *t_sort_order.lock().unwrap());

            for (index, list_entry) in list.iter().enumerate() {
                if list_entry.path == path_buf {
                    t_index.store(index, Ordering::SeqCst);
                }
            }

            let next = list[next_index(t_index.load(Ordering::SeqCst), list.len())].clone();
            prefetch(
                &next.path,
                cache.clone(),
                proxy.clone(),
                loading_info.clone(),
            );

            let prev = list[prev_index(t_index.load(Ordering::SeqCst), list.len())].clone();
            prefetch(
                &prev.path,
                cache.clone(),
                proxy.clone(),
                loading_info.clone(),
            );

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
            prefetch(
                list[next_index(self.index.load(Ordering::SeqCst), list.len())]
                    .path
                    .clone(),
                self.cache.clone(),
                self.proxy.clone(),
                self.loading_info.clone(),
            );
            Some(list[self.index.load(Ordering::SeqCst)].path.clone())
        } else {
            None
        }
    }

    pub fn prev(&mut self) -> Option<PathBuf> {
        let lock = self.list.lock().unwrap();
        if let Some(ref list) = *lock {
            if self.index.load(Ordering::SeqCst) == 0 {
                self.index.store(list.len() - 1, Ordering::SeqCst);
            } else {
                self.index.fetch_sub(1, Ordering::SeqCst);
            }
            prefetch(
                list[prev_index(self.index.load(Ordering::SeqCst), list.len())]
                    .path
                    .clone(),
                self.cache.clone(),
                self.proxy.clone(),
                self.loading_info.clone(),
            );
            Some(list[self.index.load(Ordering::SeqCst)].path.clone())
        } else {
            None
        }
    }

    /// Removes path from list and returns the path to the new current image in the dir.
    /// Will return None if there are no more images in the current dir.
    pub fn trash(&mut self, path: &PathBuf) -> Option<PathBuf> {
        self.cache.pop(path);
        let mut lock = self.list.lock().unwrap();
        if let Some(ref mut list) = *lock {
            let index = self.index.load(Ordering::SeqCst);
            let p = list[index].clone();
            if p.path == *path {
                list.remove(index);
                return list.get(index).map(|entry| &entry.path).cloned();
            }
        }

        None
    }
}

fn next_index(index: usize, len: usize) -> usize {
    let next = index + 1;
    if len <= next { 0 } else { next }
}

fn prev_index(index: usize, len: usize) -> usize {
    let current = index;
    if current == 0 { len - 1 } else { current - 1 }
}

fn sort_list(list: &mut [ImageListEntry], sort_order: SortOrder) {
    match sort_order {
        SortOrder::AccessTime => {
            list.sort_by_key(|value| value.accessed);
        }
        SortOrder::CreatedTime => {
            list.sort_by_key(|value| value.created);
        }
        SortOrder::MetadataTime => {
            // We fallback to modified time if no exif timestamp exists
            list.sort_by(|lhs, rhs| {
                lhs.exif_date
                    .unwrap_or_else(|| lhs.modified.unwrap_or(SystemTime::UNIX_EPOCH))
                    .cmp(
                        &rhs.exif_date
                            .unwrap_or_else(|| rhs.modified.unwrap_or(SystemTime::UNIX_EPOCH)),
                    )
            });
        }
        SortOrder::ModifiedTime => {
            list.sort_by_key(|value| value.modified);
        }
        SortOrder::Name => {
            list.sort_by(|lhs, rhs| {
                crate::util::natural_cmp::natural_cmp(
                    &lhs.path.to_string_lossy(),
                    &rhs.path.to_string_lossy(),
                )
            });
        }
        SortOrder::Size => {
            list.sort_by_key(|value| value.size);
        }
    }
}
