use std::{
    borrow::Cow,
    sync::{mpsc::Sender, Arc, RwLock},
    thread,
};

use glium::glutin::event_loop::EventLoopProxy;
use image::{
    imageops::{flip_horizontal_in_place, flip_vertical_in_place},
    EncodableLayout, GenericImageView, ImageBuffer, Rgba,
};

use super::{image_view::ImageView, op_queue::Output};
use crate::util::{Image, ImageData, UserEvent};

pub fn copy(view: &ImageView, proxy: EventLoopProxy<UserEvent>, sender: Sender<Output>) {
    let image_data = view.image_data.clone();
    let rotation = view.rotation;
    let horizontal_flip = view.horizontal_flip;
    let vertical_flip = view.vertical_flip;

    thread::spawn(move || {
        let guard = image_data.read().unwrap();
        let frame = guard.frames.first().unwrap();
        let buffer = match rotation {
            0 => Cow::Borrowed(frame.buffer()),
            1 => Cow::Owned(frame.buffer().rotate270()),
            2 => Cow::Owned(frame.buffer().rotate180()),
            3 => Cow::Owned(frame.buffer().rotate90()),
            _ => unreachable!("image is rotated more then 360 degrees"),
        };

        let buffer = match buffer {
            Cow::Owned(mut buffer) => {
                if horizontal_flip {
                    flip_horizontal_in_place(&mut buffer);
                }

                if vertical_flip {
                    flip_vertical_in_place(&mut buffer);
                }
                Cow::Owned(buffer)
            }
            Cow::Borrowed(buffer) => {
                if !horizontal_flip && !vertical_flip {
                    Cow::Borrowed(buffer)
                } else {
                    let mut buffer = buffer.clone();
                    if horizontal_flip {
                        flip_horizontal_in_place(&mut buffer);
                    }

                    if vertical_flip {
                        flip_vertical_in_place(&mut buffer);
                    }
                    Cow::Owned(buffer)
                }
            }
        };

        let (width, height) = buffer.dimensions();
        let buffer = buffer.to_rgba8();
        let image_data = arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Borrowed(buffer.as_bytes()),
        };

        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_image(image_data);
        }

        let _ = sender.send(Output::Done);
        let _ = proxy.send_event(UserEvent::Wake);
    });
}

pub fn paste(proxy: EventLoopProxy<UserEvent>, sender: Sender<Output>) {
    thread::spawn(move || {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(image_data) = clipboard.get_image() {
                let width = image_data.width;
                let height = image_data.height;
                let mut data = Vec::with_capacity(image_data.bytes.len());
                data.extend_from_slice(&*image_data.bytes);
                let image = ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, data)
                    .unwrap();
                let _ = sender.send(Output::ImageLoaded(
                    Arc::new(RwLock::new(ImageData::from(vec![Image::from(image)]))),
                    None,
                ));
                let _ = proxy.send_event(UserEvent::Wake);
                return;
            }
        }
        // if it fails we must still notify the main thread that we are not doing work
        let _ = sender.send(Output::Done);
        let _ = proxy.send_event(UserEvent::Wake);
    });
}
