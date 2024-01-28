use std::{borrow::Cow, sync::Arc, thread};

use image::{
    imageops::{flip_horizontal_in_place, flip_vertical_in_place},
    EncodableLayout, GenericImageView, ImageBuffer, Rgba,
};
use winit::event_loop::EventLoopProxy;

use super::{
    image_view::ImageView,
    op_queue::{Output, UserEventLoopProxyExt},
};
use crate::util::{Image, ImageData, UserEvent};

pub fn copy(view: &ImageView, proxy: EventLoopProxy<UserEvent>) {
    let image_data = view.image_data.clone();
    let rotation = view.rotation();
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

        proxy.send_output(Output::Done);
    });
}

pub fn paste(proxy: EventLoopProxy<UserEvent>) {
    thread::spawn(move || {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(image_data) = clipboard.get_image() {
                let width = image_data.width;
                let height = image_data.height;
                let mut data = Vec::with_capacity(image_data.bytes.len());
                data.extend_from_slice(&image_data.bytes);
                let image = ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, data)
                    .unwrap();
                proxy.send_output(Output::ImageLoaded(
                    Arc::new(ImageData::from(vec![Image::from(image)])),
                    None,
                ));
                return;
            }
        }
        // if it fails we must still notify the main thread that we are not doing work
        proxy.send_output(Output::Done);
    });
}
