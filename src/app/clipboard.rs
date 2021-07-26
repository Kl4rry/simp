use glium::glutin::event_loop::EventLoopProxy;
use image::{Frame, ImageBuffer, Rgba};
use std::{borrow::Cow, time::Instant};
use user_event::UserEvent;

use super::image_view::ImageView;

pub fn copy(view: &ImageView) {
    let image = view.frames[0].clone().into_buffer();
    let (width, height) = image.dimensions();
    let image_data = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Borrowed(image.as_raw()),
    };

    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_image(image_data);
    }
}

pub fn paste(proxy: &EventLoopProxy<UserEvent>) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Ok(image_data) = clipboard.get_image() {
            let width = image_data.width;
            let height = image_data.height;
            let mut data = Vec::with_capacity(image_data.bytes.len());
            data.extend_from_slice(&*image_data.bytes);
            let image =
                ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, data).unwrap();
            let event = UserEvent::ImageLoaded(Some(vec![Frame::new(image)]), None, Instant::now());
            let _ = proxy.send_event(event);
        }
    }
}
