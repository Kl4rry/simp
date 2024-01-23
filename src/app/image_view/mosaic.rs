use std::{iter, sync::Arc};

use image::{DynamicImage, SubImage};
use wgpu::{util::DeviceExt, Limits};

use super::{image_renderer::Vertex, texture};
use crate::{util::ImageData, vec2::Vec2, WgpuState};

pub struct Tile {
    pub vertices: wgpu::Buffer,
    pub texture: texture::Texture,
}

pub struct Mosaic {
    pub tiles: Vec<Tile>,
    pub indices: wgpu::Buffer,
}

impl Mosaic {
    pub fn from_images(wgpu: &WgpuState, images: Arc<ImageData>) -> Vec<Self> {
        let limit = Limits::default().max_texture_dimension_2d;

        let mut output = Vec::new();
        for image in &images.frames {
            let mut tiles = Vec::new();
            let image = &image.image;

            let tile_width = (image.width() / limit) + 1;
            let tile_height = (image.height() / limit) + 1;

            // TODO: make parallel
            for x in 0..tile_width {
                for y in 0..tile_height {
                    let start_x = x * limit;
                    let start_y = y * limit;
                    let end_x = ((x + 1) * limit).min(image.width());
                    let end_y = ((y + 1) * limit).min(image.height());

                    if end_x.saturating_sub(start_x) == 0 || end_x.saturating_sub(start_x) == 0 {
                        continue;
                    }

                    let vertices = get_vertex_buffer(
                        wgpu,
                        start_x as f32,
                        start_y as f32,
                        end_x as f32,
                        end_y as f32,
                    );

                    // Fast path for small images
                    let texture = if tile_height == 1 && tile_width == 1 {
                        texture::Texture::from_image(&wgpu.device, &wgpu.queue, image, None)
                    } else {
                        let sub_image =
                            get_tile(image, start_x, start_y, end_x - start_x, end_y - start_y);
                        texture::Texture::from_image(&wgpu.device, &wgpu.queue, &sub_image, None)
                    };

                    tiles.push(Tile { vertices, texture });
                }
            }

            let indices: &[u32] = &[0, 1, 2, 2, 1, 3];
            let indices = wgpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Image Index Buffer"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });

            output.push(Mosaic { tiles, indices })
        }
        wgpu.queue.submit(iter::empty());
        output
    }
}

fn get_tile(image: &DynamicImage, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
    match image {
        DynamicImage::ImageLuma8(image) => {
            DynamicImage::ImageLuma8(SubImage::new(image, x, y, width, height).to_image())
        }
        DynamicImage::ImageLumaA8(image) => {
            DynamicImage::ImageLumaA8(SubImage::new(image, x, y, width, height).to_image())
        }
        DynamicImage::ImageRgb8(image) => {
            DynamicImage::ImageRgb8(SubImage::new(image, x, y, width, height).to_image())
        }
        DynamicImage::ImageRgba8(image) => {
            DynamicImage::ImageRgba8(SubImage::new(image, x, y, width, height).to_image())
        }
        DynamicImage::ImageLuma16(image) => {
            DynamicImage::ImageLuma16(SubImage::new(image, x, y, width, height).to_image())
        }
        DynamicImage::ImageLumaA16(image) => {
            DynamicImage::ImageLumaA16(SubImage::new(image, x, y, width, height).to_image())
        }
        DynamicImage::ImageRgb16(image) => {
            DynamicImage::ImageRgb16(SubImage::new(image, x, y, width, height).to_image())
        }
        DynamicImage::ImageRgba16(image) => {
            DynamicImage::ImageRgba16(SubImage::new(image, x, y, width, height).to_image())
        }
        DynamicImage::ImageRgb32F(image) => {
            DynamicImage::ImageRgb32F(SubImage::new(image, x, y, width, height).to_image())
        }
        DynamicImage::ImageRgba32F(image) => {
            DynamicImage::ImageRgba32F(SubImage::new(image, x, y, width, height).to_image())
        }
        image => DynamicImage::ImageRgba8(SubImage::new(image, x, y, width, height).to_image()),
    }
}

fn get_vertex_buffer(
    wgpu: &WgpuState,
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
) -> wgpu::Buffer {
    let texture_cords = (
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, 0.0),
    );
    let shape = [
        Vertex::new(start_x, start_y, texture_cords.3.x(), texture_cords.3.y()),
        Vertex::new(start_x, end_y, texture_cords.2.x(), texture_cords.2.y()),
        Vertex::new(end_x, start_y, texture_cords.1.x(), texture_cords.1.y()),
        Vertex::new(end_x, end_y, texture_cords.0.x(), texture_cords.0.y()),
    ];

    wgpu.device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image Vertex Buffer"),
            contents: bytemuck::cast_slice(shape.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        })
}
