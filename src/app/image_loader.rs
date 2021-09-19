use std::{collections::HashSet, path::PathBuf};

pub struct ImageLoader {
    pub target_file: Option<PathBuf>,
    pub loading: HashSet<PathBuf>,
}

impl ImageLoader {
    pub fn new() -> Self {
        Self {
            target_file: None,
            loading: HashSet::new(),
        }
    }
}

impl Default for ImageLoader {
    fn default() -> Self {
        Self::new()
    }
}
