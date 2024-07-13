use deref_derive::{Deref, DerefMut};
use flecs_ecs::prelude::*;
use std::num::NonZeroUsize;
use wgpu::{Adapter, Device, Instance, Queue, TextureFormat};

use crate::{application::Resize, window::Window};

#[derive(Component)]
pub struct WGPU {
    pub adapter: Adapter,
    pub device: Device,
    pub instance: Instance,
    pub queue: Queue,
    pub format: TextureFormat,
}

#[derive(Component)]
pub struct Vello {
    renderer: vello::Renderer,
}

impl Vello {
    pub fn new(wgpu: &mut WGPU) -> Self {
        Self {
            renderer: vello::Renderer::new(
                &wgpu.device,
                vello::RendererOptions {
                    surface_format: Some(wgpu.format),
                    use_cpu: false,
                    antialiasing_support: vello::AaSupport::area_only(),
                    num_init_threads: NonZeroUsize::new(1),
                },
            )
            .expect("Failed to create vello renderer."),
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
            base_color: vello::peniko::Color::GRAY,
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

        world.get::<&mut WGPU>(|wgpu| {
            world.set(Vello::new(wgpu));
        });

        // Respond to window events
        observer!("resize_window", world, Resize, &WGPU($), &mut Window).each_iter(
            |it, _, (wgpu, window)| {
                let data = it.param();
                // Reconfigure the surface with the new size
                window.config.width = data.width.max(1);
                window.config.height = data.height.max(1);
                window.surface.configure(&wgpu.device, &window.config);
            },
        );

        world
            .system_named::<&mut Window>("create_texture")
            .kind::<flecs::pipeline::OnStore>()
            .each(|window| {
                if !window.redraw {
                    return;
                }
                let Ok(frame) = window.surface.get_current_texture() else {
                    return;
                };

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                window.texture = Some(frame);
                window.view = Some(view);
            });

        system!("render_vello_scene", world, &mut WGPU($), &mut Vello($), &mut Window(up), &mut VelloScene)
            .kind::<flecs::pipeline::OnStore>()
            .each(|(wgpu, vello, window, scene)| {
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
                if let Some(surface) = &window.texture {
                    vello
                        .renderer
                        .render_to_surface(
                            &wgpu.device,
                            &wgpu.queue,
                            scene,
                            surface,
                            &vello::RenderParams {
                                base_color: scene.base_color,
                                width: window.config.width,
                                height: window.config.height,
                                antialiasing_method: vello::AaConfig::Area,
                            },
                        )
                        .expect("Failed to render scene.");
                };
                scene.reset()
            });

        world
            .system_named::<&mut Window>("present_texture")
            .kind::<flecs::pipeline::OnStore>()
            .each(|window| {
                if let Some(texture) = window.texture.take() {
                    texture.present();
                    window.redraw = false;
                    window.view = None;
                }
            });
    }
}
