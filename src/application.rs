use deref_derive::Deref;
use flecs_ecs::{core::flecs::rest::Rest, prelude::*};
use std::error::Error;
use wgpu::SurfaceTargetUnsafe;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

use crate::{
    render::{DefaultFormat, RenderModule, WGPU},
    window::{Window, WindowMap},
    Cursor, Input, TextWriter, VelloShapeModule,
};

#[derive(Component, Deref)]
pub struct Resize(PhysicalSize<u32>);

#[derive(Component)]
pub struct MainWindow;

#[derive(Component)]
pub struct WindowPrefab;
pub struct Application {
    pub world: World,
    initialized: bool,
}

impl Application {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            initialized: false,
        }
    }

    pub fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<Entity, Box<dyn Error>> {
        self.world.map::<&mut WGPU, _>(|wgpu| {
            let window_attributes =
                winit::window::Window::default_attributes().with_title("flecs-wgpu-rs");

            let window = event_loop.create_window(window_attributes)?;

            let surface = unsafe {
                let surface_target = SurfaceTargetUnsafe::from_window(&window)
                    .expect("Failed to create surface target.");
                wgpu.instance
                    .create_surface_unsafe(surface_target)
                    .expect("Failed to create surface.")
            };

            let mut size: winit::dpi::PhysicalSize<u32> = window.inner_size();
            size.width = size.width.max(1);
            size.height = size.height.max(1);

            let mut config = surface
                .get_default_config(&wgpu.adapter, size.width, size.height)
                .unwrap();
            // For vello
            config.format = wgpu::TextureFormat::Rgba8Unorm;
            config.usage |= wgpu::TextureUsages::STORAGE_BINDING;

            surface.configure(&wgpu.device, &config);

            let window_id = window.id();
            let window_e = self
                .world
                .entity()
                .set(Window {
                    window,
                    surface,
                    config,
                    redraw: true,
                    texture: None,
                    view: None,
                })
                .is_a::<WindowPrefab>();

            self.world.get::<&mut WindowMap>(|map| {
                map.insert(window_id, window_e.id());
            });
            Ok(window_e.id())
        })
    }

    pub async fn initialize(&mut self, event_loop: &ActiveEventLoop) {
        // flecs will manage our frame time
        event_loop.set_control_flow(ControlFlow::Poll);

        self.world.set_target_fps(60.0);
        self.world.set(WindowMap::default());
        self.world.set(Rest::default());

        let instance = wgpu::Instance::default();
        self.world.set(WGPU::new(instance).await);

        self.world
            .prefab_type::<WindowPrefab>()
            .set(Cursor::default());

        let initial_window = self
            .create_window(event_loop)
            .expect("Failed to create initial window.")
            .entity_view(&self.world);

        self.world.get::<&mut WGPU>(|wgpu| {
            initial_window.get::<&mut Window>(|window| {
                let capabilities = window.surface.get_capabilities(&wgpu.adapter);
                self.world.set(DefaultFormat(capabilities.formats[0]));
            });
        });

        self.world.set(Input::default());
        system!(self.world, &mut Input($))
            .kind::<flecs::pipeline::PostFrame>()
            .each(|input| {
                input.clear_frame();
            });

        self.world.set(TextWriter::new());

        self.world.add_first::<MainWindow>(initial_window.id());
        self.world.import::<RenderModule>();
        self.world.import::<VelloShapeModule>();
    }

    pub fn run(mut self) -> Result<(), Box<dyn Error>> {
        let event_loop = EventLoop::new().expect("Failed to create winit event loop.");
        event_loop.run_app(&mut self).map_err(Into::into)
    }
}

impl ApplicationHandler<()> for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.initialized {
            pollster::block_on(self.initialize(event_loop));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window_e = self.world.map::<&WindowMap, _>(|map| {
            map.get(&window_id)
                .expect("Event for non-existant window.")
                .entity_view(&self.world)
        });
        match event {
            WindowEvent::Resized(new_size) => {
                window_e.get::<&mut Window>(|w| w.request_redraw());

                self.world
                    .event()
                    .add::<Window>()
                    .target(window_e)
                    .emit(&Resize(new_size));
            }
            WindowEvent::RedrawRequested => {
                window_e.get::<&mut Window>(|w| w.redraw = true);
                self.world.progress();
                window_e.get::<&mut Window>(|w| w.request_redraw());
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        };

        window_e.get::<&mut Cursor>(|cursor| cursor.process_event(&event));
        self.world
            .get::<&mut Input>(|input| input.process_event(&event));
    }
}
