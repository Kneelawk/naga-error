mod use_naga_lib;
mod use_naga_spv;
mod use_tint_spv;
mod use_wgsl_src;

#[macro_use]
extern crate log;

use core::num::NonZeroU32;
use image::{ImageBuffer, Rgba};
use std::{
    convert::TryFrom,
    mem::size_of,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::task;
use wgpu::{
    BackendBit, BlendState, Buffer, BufferAddress, BufferDescriptor, BufferUsage, Color,
    ColorTargetState, ColorWrite, CommandEncoderDescriptor, Device, Extent3d, Face, FragmentState,
    FrontFace, ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, Instance, LoadOp, Maintain,
    MapMode, MultisampleState, Operations, Origin3d, PipelineLayoutDescriptor, PolygonMode,
    PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsage, TextureView, VertexState,
};

const OUTPUT_FILE: &'static str = "output.png";

const TEXTURE_WIDTH: u32 = 256;
const TEXTURE_HEIGHT: u32 = 256;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    info!("Creating Instance...");
    let instance = Instance::new(BackendBit::PRIMARY);
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: Default::default(),
            compatible_surface: None,
        })
        .await
        .unwrap();

    info!("Requesting device...");
    let (device, queue) = adapter
        .request_device(&Default::default(), None)
        .await
        .unwrap();

    info!("Creating device poll task...");
    let device = Arc::new(device);
    let poll_device = device.clone();
    let status = Arc::new(AtomicBool::new(true));
    let poll_status = status.clone();
    let poll_task = tokio::spawn(async move {
        while poll_status.load(Ordering::Relaxed) {
            poll_device.poll(Maintain::Poll);
            task::yield_now().await;
        }
    });

    info!("Creating framebuffer...");
    let (texture, texture_view) = create_texture(&device, TEXTURE_WIDTH, TEXTURE_HEIGHT);
    let buffer = create_texture_buffer(&device, TEXTURE_WIDTH, TEXTURE_HEIGHT);

    info!("Creating shader module...");

    #[cfg(feature = "use-naga-lib")]
    let shader = use_naga_lib::use_naga_lib().await;

    #[cfg(feature = "use-naga-spv")]
    let shader = use_naga_spv::use_naga_spv();

    #[cfg(feature = "use-tint-spv")]
    let shader = use_tint_spv::use_tint_spv();

    #[cfg(feature = "use-wgsl-src")]
    let shader = use_wgsl_src::use_wgsl_src();

    let module = device.create_shader_module(&ShaderModuleDescriptor {
        label: Some("Vertex Shader"),
        source: shader,
        flags: Default::default(),
    });

    info!("Creating render pipeline...");
    let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: VertexState {
            module: &module,
            entry_point: "vert_main",
            buffers: &[],
        },
        fragment: Some(FragmentState {
            module: &module,
            entry_point: "frag_main",
            targets: &[ColorTargetState {
                format: TextureFormat::Rgba8UnormSrgb,
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrite::ALL,
            }],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            clamp_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
    });

    info!("Encoding command buffer...");
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Command Encoder"),
    });

    {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&render_pipeline);
        render_pass.draw(0..3, 0..1);
    }

    encoder.copy_texture_to_buffer(
        ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
        },
        ImageCopyBuffer {
            buffer: &buffer,
            layout: ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(
                    NonZeroU32::try_from(size_of::<u32>() as u32 * TEXTURE_WIDTH).unwrap(),
                ),
                rows_per_image: Some(NonZeroU32::try_from(TEXTURE_HEIGHT).unwrap()),
            },
        },
        Extent3d {
            width: TEXTURE_WIDTH,
            height: TEXTURE_HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    info!("Submitting command buffer...");
    queue.submit(Some(encoder.finish()));

    {
        info!("Reading framebuffer...");
        let buffer_slice = buffer.slice(..);
        buffer_slice.map_async(MapMode::Read).await.unwrap();

        let data = buffer_slice.get_mapped_range();

        info!("Writing image...");
        let image =
            ImageBuffer::<Rgba<u8>, _>::from_raw(TEXTURE_WIDTH, TEXTURE_HEIGHT, data).unwrap();
        image.save(OUTPUT_FILE).unwrap();
    }
    buffer.unmap();

    info!("Shutting down...");

    status.store(false, Ordering::Relaxed);
    poll_task.await.unwrap();

    info!("Done.");
}

fn create_texture(device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("Framebuffer"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsage::COPY_SRC | TextureUsage::RENDER_ATTACHMENT,
    });
    let texture_view = texture.create_view(&Default::default());

    (texture, texture_view)
}

fn create_texture_buffer(device: &Device, width: u32, height: u32) -> Buffer {
    let size = width * height * size_of::<u32>() as u32;
    let texture_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Framebuffer Buffer"),
        size: size as BufferAddress,
        usage: BufferUsage::COPY_DST | BufferUsage::MAP_READ,
        mapped_at_creation: false,
    });

    texture_buffer
}
