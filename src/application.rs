use deref_derive::Deref;
use flecs_ecs::{core::flecs::rest::Rest, prelude::*};
use std::error::Error;
use wgpu::{SurfaceTargetUnsafe, TextureFormat};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

use crate::{
    render::WGPU,
    window::{Window, WindowMap},
    Cursor, Input, RenderModule, TextWriter, VelloShapeModule,
};

#[derive(Component, Deref)]
pub struct Resize(PhysicalSize<u32>);

#[derive(Component)]
pub struct MainWindow;

#[derive(Component)]
pub struct WindowPrefab;

#[derive(Default)]
pub struct Application {
    pub world: World,
    initialized: bool,
}

impl Application {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn initial_window(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<Entity, Box<dyn Error>> {
        let instance = wgpu::Instance::default();
        let window_attributes =
            winit::window::Window::default_attributes().with_title("flecs-wgpu-rs");

        let window = event_loop.create_window(window_attributes)?;

        let surface = unsafe {
            let surface_target = SurfaceTargetUnsafe::from_window(&window)
                .expect("Failed to create surface target.");
            instance
                .create_surface_unsafe(surface_target)
                .expect("Failed to create surface.")
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let mut size: winit::dpi::PhysicalSize<u32> = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();

        // For vello
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .into_iter()
            .find(|it| matches!(it, TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm))
            .expect("surface should support Rgba8Unorm or Bgra8Unorm");
        config.format = format;
        config.usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;

        surface.configure(&device, &config);

        let wgpu = WGPU {
            adapter,
            device,
            instance,
            queue,
            format,
        };

        let window_id = window.id();
        let window_e = self
            .world
            .entity_named("window")
            .set(Window {
                window,
                surface,
                config,
                redraw: true,
                texture: None,
                view: None,
            })
            .is_a::<WindowPrefab>();

        self.world.set(wgpu);
        self.world.get::<&mut WindowMap>(|map| {
            map.insert(window_id, window_e.id());
        });
        Ok(window_e.id())
    }

    pub async fn initialize(&mut self, event_loop: &ActiveEventLoop) {
        // flecs will manage our frame time
        event_loop.set_control_flow(ControlFlow::Poll);

        self.world.set_target_fps(60.0);
        self.world.set(WindowMap::default());
        self.world.set(Rest::default());

        self.world
            .prefab_type::<WindowPrefab>()
            .set(Cursor::default());

        let initial_window = self
            .initial_window(event_loop)
            .await
            .expect("Failed to create initial window.")
            .entity_view(&self.world);

        self.world.set(Input::default());
        self.world.set(TextWriter::new());

        self.world.add_first::<MainWindow>(initial_window.id());

        self.world.import::<ApplicationModule>();
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
        let window_e = self.world.get::<&WindowMap>(|map| {
            map.get(&window_id)
                .expect("Event for non-existent window.")
                .entity_view(&self.world)
        });
        match event {
            WindowEvent::Resized(new_size) => {
                window_e.get::<&mut Window>(|w| w.request_redraw());

                self.world
                    .event()
                    .add::<Window>()
                    .entity(window_e)
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

#[derive(Component)]
pub struct ApplicationModule;

impl Module for ApplicationModule {
    fn module(world: &World) {
        world.module::<Self>("module");

        system!("clear_input", world, &mut Input($))
            .kind::<flecs::pipeline::OnStore>()
            .each(|input| {
                input.clear_frame();
            });
    }
}
