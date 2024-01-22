use std::{borrow::Cow, mem};

use image::GenericImageView;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub dimensions: (u32, u32),
    pub diffuse_bind_group: wgpu::BindGroup,
}

fn to_u8<T: Copy + bytemuck::Pod>(input: Vec<T>) -> Vec<u8> {
    let cap = input.capacity() * mem::size_of::<T>();
    let len = input.len() * mem::size_of::<T>();
    let ptr = input.as_ptr() as *mut u8;
    mem::forget(input);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

impl Texture {
    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Self {
        let dimensions = img.dimensions();

        #[rustfmt::skip]
        let (bytes_per_pixel, format, bytes):  (_, _, Cow<[u8]>) = match img {
            image::DynamicImage::ImageLuma8(_) => (4, wgpu::TextureFormat::R8Unorm, img.as_bytes().into()),
            image::DynamicImage::ImageLumaA8(_) => (4, wgpu::TextureFormat::Rgba8UnormSrgb, img.to_rgba8().into_raw().into()),
            image::DynamicImage::ImageRgb8(_) => (4, wgpu::TextureFormat::Rgba8UnormSrgb, img.to_rgba8().into_raw().into()),
            image::DynamicImage::ImageRgba8(_) => (4, wgpu::TextureFormat::Rgba8UnormSrgb, img.as_bytes().into()),
            image::DynamicImage::ImageLuma16(_) => (8, wgpu::TextureFormat::Rgba32Float, to_u8(img.to_rgba32f().into_raw()).into()),
            image::DynamicImage::ImageLumaA16(_) => (8, wgpu::TextureFormat::Rgba32Float, to_u8(img.to_rgba32f().into_raw()).into()),
            image::DynamicImage::ImageRgb16(_) => (8, wgpu::TextureFormat::Rgba32Float, to_u8(img.to_rgba32f().into_raw()).into()),
            image::DynamicImage::ImageRgba16(_) => (8, wgpu::TextureFormat::Rgba32Float, img.as_bytes().into()),
            image::DynamicImage::ImageRgb32F(_) => (16, wgpu::TextureFormat::Rgba32Float, to_u8(img.to_rgba32f().into_raw()).into()),
            image::DynamicImage::ImageRgba32F(_) => (16, wgpu::TextureFormat::Rgba32Float, img.as_bytes().into()),
            _ => (4, wgpu::TextureFormat::Rgba8UnormSrgb, img.to_rgba8().into_raw().into()),
        };

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_pixel * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        // TODO: add mip maps
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let layout = Self::get_bind_group_layout(device);

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("Diffuse Bind Group"),
        });

        Self {
            texture,
            view,
            sampler,
            dimensions,
            diffuse_bind_group,
        }
    }

    pub fn get_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Texture Bind Group Layout"),
        })
    }
}
