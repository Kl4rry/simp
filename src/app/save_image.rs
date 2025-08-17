use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
    thread,
};

use image::{
    ImageFormat,
    imageops::{flip_horizontal_in_place, flip_vertical_in_place},
};
use winit::event_loop::EventLoopProxy;

use super::{
    dialog_manager::DialogProxy,
    op_queue::{Output, UserEventLoopProxyExt},
    preferences::PREFERENCES,
};
use crate::{
    WgpuState,
    image_io::save::{gif, jpeg, save_with_format, webp, webp_animation},
    util::{Image, ImageData, UserEvent},
};

pub fn open(name: String, proxy: EventLoopProxy<UserEvent>, wgpu: &WgpuState) {
    let dialog = rfd::FileDialog::new()
        .set_file_name(name)
        .set_parent(&wgpu.window)
        .add_filter("PNG", &["png", "apng"])
        .add_filter("JPEG", &["jpg", "jpeg", "jpe", "jif", "jfif"])
        .add_filter("JPEG XL", &["jxl"])
        .add_filter("GIF", &["gif"])
        .add_filter("ICO", &["ico"])
        .add_filter("BMP", &["bmp"])
        .add_filter("TIFF", &["tiff", "tif"])
        .add_filter("WEBP", &["webp"])
        .add_filter("farbfeld", &["ff", "farbfeld"])
        .add_filter("TGA", &["tga"])
        .add_filter("QOI", &["qoi"])
        .add_filter("HDR", &["hdr"])
        .add_filter("EXR", &["exr"]);

    thread::spawn(move || {
        if let Some(path) = dialog.save_file() {
            let _ = proxy.send_event(UserEvent::QueueSave(path));
        }
    });
}

pub fn save(
    proxy: EventLoopProxy<UserEvent>,
    dialog_proxy: DialogProxy,
    mut path: PathBuf,
    image_data: Arc<RwLock<Arc<ImageData>>>,
    rotation: i32,
    horizontal_flip: bool,
    vertical_flip: bool,
) {
    let os_str = path.extension();
    let ext = match os_str {
        Some(ext) => ext.to_string_lossy().to_string().to_lowercase(),
        None => String::from("png"),
    };
    path.set_extension(&ext);

    thread::spawn(move || {
        let guard = image_data.read().unwrap();
        let old_frames = &guard.frames;
        let mut frames = Vec::new();
        for frame in old_frames {
            let buffer = match rotation {
                0 => frame.buffer().clone(),
                1 => frame.buffer().rotate270(),
                2 => frame.buffer().rotate180(),
                3 => frame.buffer().rotate90(),
                _ => unreachable!("image is rotated more then 360 degrees"),
            };
            frames.push(Image::with_delay(buffer, frame.delay));
        }

        for frame in frames.iter_mut() {
            if horizontal_flip {
                flip_horizontal_in_place(frame.buffer_mut());
            }

            if vertical_flip {
                flip_vertical_in_place(frame.buffer_mut());
            }
        }

        let res = match ext.as_str() {
            "hdr" => save_with_format(path, &frames[0], ImageFormat::Hdr),
            "png" => save_with_format(path, &frames[0], ImageFormat::Png),
            "qoi" => save_with_format(path, &frames[0], ImageFormat::Qoi),
            "jpg" | "jpeg" | "jpe" | "jif" | "jfif" => {
                let quality = match get_jpeg_quality(dialog_proxy.clone()) {
                    Some(quality) => quality,
                    None => {
                        proxy.send_output(Output::Done);
                        return;
                    }
                };

                jpeg(path, &frames[0], quality)
            }
            "ico" => save_with_format(path, &frames[0], ImageFormat::Ico),
            "tga" => save_with_format(path, &frames[0], ImageFormat::Tga),
            "ff" | "farbfeld" => save_with_format(path, &frames[0], ImageFormat::Farbfeld),
            "tiff" | "tif" => save_with_format(path, &frames[0], ImageFormat::Tiff),
            "gif" => gif(path, frames),
            "webp" => {
                let (quality, lossy) = match get_webp_quality(dialog_proxy.clone()) {
                    Some(quality) => quality,
                    None => {
                        proxy.send_output(Output::Done);
                        return;
                    }
                };

                if frames.len() > 1 {
                    webp_animation(path, frames, quality, lossy)
                } else {
                    webp(path, &frames[0], quality, lossy)
                }
            }
            "exr" => save_with_format(path, &frames[0], ImageFormat::OpenExr),
            #[cfg(feature = "jxl")]
            "jxl" => {
                let (quality, lossy) = match get_jxl_quality(dialog_proxy.clone()) {
                    Some(quality) => quality,
                    None => {
                        proxy.send_output(Output::Done);
                        return;
                    }
                };

                crate::image_io::save::jpeg_xl(path, &frames[0], quality, lossy)
            }
            _ => {
                path.set_extension("png");
                save_with_format(path, &frames[0], ImageFormat::Png)
            }
        };

        proxy.send_output(Output::Saved);
        if let Err(error) = res {
            let _ = proxy.send_event(UserEvent::ErrorMessage(error.to_string()));
        }
    });
}

pub fn get_jpeg_quality(dialog_proxy: DialogProxy) -> Option<u8> {
    dialog_proxy
        .spawn_dialog("Export settings", |ui, enter| {
            let mut preferences = PREFERENCES.lock().unwrap();
            let mut output = None;
            egui::Grid::new("jpeg export settings grid").show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.label("Jpeg Quality: ");
                });
                ui.add(egui::Slider::new(&mut preferences.jpeg_quality, 1..=100));
                ui.end_row();

                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        if ui.button("Save").clicked() {
                            output = Some(Some(preferences.jpeg_quality))
                        }
                    },
                );

                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        if ui.button("Cancel").clicked() {
                            output = Some(None);
                        }
                    },
                );

                if *enter {
                    *enter = false;
                    output = Some(Some(preferences.jpeg_quality))
                }
            });
            output
        })
        .wait()
        .flatten()
}

pub fn get_webp_quality(dialog_proxy: DialogProxy) -> Option<(f32, bool)> {
    dialog_proxy
        .spawn_dialog("Export settings", |ui, enter| {
            let mut preferences = PREFERENCES.lock().unwrap();
            let mut output = None;
            egui::Grid::new("webp export settings grid").show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.label("WebP lossy compression: ");
                });
                ui.add(egui::Checkbox::new(&mut preferences.webp_lossy, ""));
                ui.end_row();

                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.label("Webp Quality: ");
                });
                ui.add(egui::Slider::new(
                    &mut preferences.webp_quality,
                    0.0..=100.0,
                ));
                ui.end_row();

                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        if ui.button("Save").clicked() {
                            output = Some(Some((preferences.webp_quality, preferences.webp_lossy)))
                        }
                    },
                );

                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        if ui.button("Cancel").clicked() {
                            output = Some(None);
                        }
                    },
                );

                if *enter {
                    *enter = false;
                    output = Some(Some((preferences.webp_quality, preferences.webp_lossy)))
                }
            });
            output
        })
        .wait()
        .flatten()
}

#[cfg(feature = "jxl")]
pub fn get_jxl_quality(dialog_proxy: DialogProxy) -> Option<(f32, bool)> {
    dialog_proxy
        .spawn_dialog("Export settings", |ui, enter| {
            let mut preferences = PREFERENCES.lock().unwrap();
            let mut output = None;
            egui::Grid::new("jxl export settings grid").show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.label("JPEG XL lossy compression: ");
                });
                ui.add(egui::Checkbox::new(&mut preferences.jxl_lossy, ""));
                ui.end_row();

                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.label("JPEG XL Quality: ");
                });
                ui.add(egui::Slider::new(&mut preferences.jxl_quality, 0.0..=15.0));
                ui.end_row();

                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        if ui.button("Save").clicked() {
                            output = Some(Some((preferences.jxl_quality, preferences.jxl_lossy)))
                        }
                    },
                );

                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        if ui.button("Cancel").clicked() {
                            output = Some(None);
                        }
                    },
                );

                if *enter {
                    *enter = false;
                    output = Some(Some((preferences.jxl_quality, preferences.jxl_lossy)))
                }
            });
            output
        })
        .wait()
        .flatten()
}
