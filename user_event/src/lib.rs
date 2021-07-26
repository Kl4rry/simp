use glium::glutin::window::CursorIcon;
use image::Frame;
use std::{path::PathBuf, time::Instant};

pub enum UserEvent {
    ImageLoaded(Option<Vec<Frame>>, Option<PathBuf>, Instant),
    ImageError(String),
    SetCursor(CursorIcon),
}
