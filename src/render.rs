use deref_derive::{Deref, DerefMut};
use flecs_ecs::prelude::*;
use std::{num::NonZeroUsize, sync::Mutex};
use wgpu::{
    Adapter, BindGroup, BindGroupLayout, Device, Instance, Queue, RenderPipeline, Sampler,
    TextureFormat,
};

use crate::{application::Resize, window::Window};

#[derive(Component)]
#[flecs(traits(Singleton))]
pub struct WGPU {
    pub adapter: Adapter,
    pub device: Device,
    pub instance: Instance,
    pub queue: Queue,
    pub format: TextureFormat,
}

#[derive(Component)]
#[flecs(traits(Singleton))]
pub struct BlitPipeline {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
}

impl BlitPipeline {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                r#"
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Full screen triangle
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = f32((vertex_index & 2u) << 1u) - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pos.xy / vec2<f32>(textureDimensions(input_texture));
    return textureSample(input_texture, texture_sampler, uv);
}
                "#,
            )),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Blit Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blit Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    pub fn create_bind_group(
        &self,
        device: &Device,
        texture_view: &wgpu::TextureView,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Blit Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }
}

#[derive(Component)]
#[flecs(traits(Singleton))]
pub struct Vello {
    // The `Mutex` makes this `Sync` (it is already `Send`) so that we
    // can store it as a component.
    renderer: Mutex<vello::Renderer>,
}

impl Vello {
    pub fn new(wgpu: &mut WGPU) -> Self {
        Self {
            renderer: Mutex::new(
                vello::Renderer::new(
                    &wgpu.device,
                    vello::RendererOptions {
                        pipeline_cache: None,
                        use_cpu: false,
                        antialiasing_support: vello::AaSupport::area_only(),
                        num_init_threads: NonZeroUsize::new(1),
                    },
                )
                .expect("Failed to create vello renderer."),
            ),
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct VelloScene {
    #[deref]
    scene: vello::Scene,
    pub base_color: vello::peniko::Color,
    pub camera: vello::kurbo::Affine,
    pub scale: f64,
    pub transform: vello::kurbo::Affine,
}

impl Default for VelloScene {
    fn default() -> Self {
        Self {
            scene: vello::Scene::new(),
            base_color: vello::peniko::Color::new([0.5, 0.5, 0.5, 1.0]),
            camera: vello::kurbo::Affine::IDENTITY,
            scale: 1.0,
            transform: vello::kurbo::Affine::IDENTITY,
        }
    }
}

#[derive(Component)]
pub struct RenderModule;

impl Module for RenderModule {
    fn module(world: &World) {
        world.module::<Self>("module");

        // So the singleton trait gets applied in non-deferred context
        world.component::<Vello>();
        world.component::<BlitPipeline>();

        world.get::<&mut WGPU>(|wgpu| {
            world.set(Vello::new(wgpu));
            world.set(BlitPipeline::new(&wgpu.device, wgpu.format));
        });

        // Respond to window events
        observer!("resize_window", world, Resize, &WGPU, &mut Window).each_iter(
            |it, _, (wgpu, window)| {
                let data = it.param();
                // Reconfigure the surface with the new size
                window.config.width = data.width.max(1);
                window.config.height = data.height.max(1);
                window.surface.configure(&wgpu.device, &window.config);
            },
        );

        system!("create_texture", world, &WGPU, &mut Window)
            .kind(flecs::pipeline::OnStore)
            .each(|(wgpu, window)| {
                if !window.redraw {
                    return;
                }
                let Ok(frame) = window.surface.get_current_texture() else {
                    return;
                };

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                // Create or recreate the intermediate render texture if needed
                let needs_new_texture = window.render_texture.is_none()
                    || window
                        .render_texture
                        .as_ref()
                        .map(|t| {
                            t.width() != window.config.width || t.height() != window.config.height
                        })
                        .unwrap_or(true);

                if needs_new_texture {
                    let render_texture = wgpu.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Vello Render Texture"),
                        size: wgpu::Extent3d {
                            width: window.config.width,
                            height: window.config.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::COPY_SRC
                            | wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::STORAGE_BINDING,
                        view_formats: &[],
                    });

                    let render_view =
                        render_texture.create_view(&wgpu::TextureViewDescriptor::default());
                    window.render_texture = Some(render_texture);
                    window.render_view = Some(render_view);
                }

                window.texture = Some(frame);
                window.view = Some(view);
            });

        system!(
            "render_vello_scene",
            world,
            &mut WGPU,
            &mut Vello,
            &BlitPipeline,
            &mut Window(up),
            &mut VelloScene
        )
        .kind(flecs::pipeline::OnStore)
        .each(|(wgpu, vello, blit_pipeline, window, scene)| {
            if scene.encoding().is_empty() {
                // Add no-op shape to avoid debug assert
                scene.fill(
                    vello::peniko::Fill::EvenOdd,
                    vello::kurbo::Affine::default(),
                    vello::peniko::Color::BLACK,
                    None,
                    &vello::kurbo::Rect::new(0.0, 0.0, 0.0, 0.0),
                );
            }

            // Render to the intermediate Rgba8Unorm texture
            if let Some(render_view) = &window.render_view {
                // Lock the mutex to access the renderer
                let mut renderer = vello
                    .renderer
                    .lock()
                    .expect("Failed to lock vello renderer");
                
                renderer
                    .render_to_texture(
                        &wgpu.device,
                        &wgpu.queue,
                        scene,
                        render_view,
                        &vello::RenderParams {
                            base_color: scene.base_color,
                            width: window.config.width,
                            height: window.config.height,
                            antialiasing_method: vello::AaConfig::Area,
                        },
                    )
                    .expect("Failed to render scene.");

                // Blit from render texture to surface texture using a render pass
                if let Some(surface_view) = &window.view {
                    let bind_group = blit_pipeline.create_bind_group(&wgpu.device, render_view);

                    let mut encoder =
                        wgpu.device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Blit Encoder"),
                            });

                    {
                        let mut render_pass =
                            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Blit Render Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: surface_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                        store: wgpu::StoreOp::Store,
                                    },
                                    depth_slice: None,
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });

                        render_pass.set_pipeline(&blit_pipeline.pipeline);
                        render_pass.set_bind_group(0, &bind_group, &[]);
                        render_pass.draw(0..3, 0..1); // Full screen triangle
                    }

                    wgpu.queue.submit(Some(encoder.finish()));
                }
            }
            scene.reset()
        });

        world
            .system_named::<&mut Window>("present_texture")
            .kind(flecs::pipeline::OnStore)
            .each(|window| {
                if let Some(texture) = window.texture.take() {
                    texture.present();
                    window.redraw = false;
                    window.view = None;
                }
            });
    }
}
