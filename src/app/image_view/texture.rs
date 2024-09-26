use std::{borrow::Cow, cmp};

use image::GenericImageView;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub dimensions: (u32, u32),
    pub diffuse_bind_group: wgpu::BindGroup,
}

/*fn to_u8<T: Copy + bytemuck::Pod>(input: Vec<T>) -> Vec<u8> {
    use std::mem;
    let cap = input.capacity() * mem::size_of::<T>();
    let len = input.len() * mem::size_of::<T>();
    let ptr = input.as_ptr() as *mut u8;
    mem::forget(input);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}*/

impl Texture {
    pub fn from_image(
        command_encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Self {
        let (width, height) = img.dimensions();
        let mip_level_count = (cmp::max(width, height).ilog2() + 1).min(10);

        // TODO maybe reenable this sometime
        /*#[rustfmt::skip]
        let (bytes_per_pixel, format, bytes):  (_, _, Cow<[u8]>) = match img {
            image::DynamicImage::ImageLuma8(_) => (4, wgpu::TextureFormat::Rgba8UnormSrgb, img.to_rgba8().into_raw().into()),
            image::DynamicImage::ImageLumaA8(_) => (4, wgpu::TextureFormat::Rgba8UnormSrgb, img.to_rgba8().into_raw().into()),
            image::DynamicImage::ImageRgb8(_) => (4, wgpu::TextureFormat::Rgba8UnormSrgb, img.to_rgba8().into_raw().into()),
            image::DynamicImage::ImageRgba8(_) => (4, wgpu::TextureFormat::Rgba8UnormSrgb, img.as_bytes().into()),
            image::DynamicImage::ImageLuma16(_) => (16, wgpu::TextureFormat::Rgba32Float, to_u8(img.to_rgba32f().into_raw()).into()),
            image::DynamicImage::ImageLumaA16(_) => (16, wgpu::TextureFormat::Rgba32Float, to_u8(img.to_rgba32f().into_raw()).into()),
            image::DynamicImage::ImageRgb16(_) => (16, wgpu::TextureFormat::Rgba32Float, to_u8(img.to_rgba32f().into_raw()).into()),
            image::DynamicImage::ImageRgba16(_) => (16, wgpu::TextureFormat::Rgba32Float, img.as_bytes().into()),
            image::DynamicImage::ImageRgb32F(_) => (16, wgpu::TextureFormat::Rgba32Float, to_u8(img.to_rgba32f().into_raw()).into()),
            image::DynamicImage::ImageRgba32F(_) => (16, wgpu::TextureFormat::Rgba32Float, img.as_bytes().into()),
            _ => (4, wgpu::TextureFormat::Rgba8UnormSrgb, img.to_rgba8().into_raw().into()),
        };*/

        let (bytes_per_pixel, format, bytes): (_, _, Cow<[u8]>) = (
            4,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            img.to_rgba8().into_raw().into(),
        );

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
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
                bytes_per_row: Some(bytes_per_pixel * width),
                rows_per_image: Some(height),
            },
            size,
        );

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

        Self::generate_mipmaps(command_encoder, device, &texture, format, mip_level_count);

        Self {
            texture,
            view,
            sampler,
            dimensions: (width, height),
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

    fn generate_mipmaps(
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
        texture_format: wgpu::TextureFormat,
        mip_count: u32,
    ) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../../shader/blit.wgsl"))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(texture_format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mip"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let views = (0..mip_count)
            .map(|mip| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("mip"),
                    format: None,
                    dimension: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                })
            })
            .collect::<Vec<_>>();

        for target_mip in 1..mip_count as usize {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&views[target_mip - 1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: None,
            });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &views[target_mip],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }
    }
}
