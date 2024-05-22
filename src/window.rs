use std::collections::HashMap;

use deref_derive::{Deref, DerefMut};
use flecs_ecs::prelude::*;
use wgpu::{Surface, SurfaceConfiguration, SurfaceTexture, TextureView};
use winit::window::WindowId;

#[derive(Component, Default, Deref, DerefMut)]
pub struct WindowMap(HashMap<WindowId, Entity>);

#[derive(Component, Deref, DerefMut)]
pub struct Window {
    #[deref]
    pub(crate) window: winit::window::Window,
    pub(crate) surface: Surface<'static>,
    pub(crate) config: SurfaceConfiguration,
    pub(crate) redraw: bool,
    pub(crate) texture: Option<SurfaceTexture>,
    pub(crate) view: Option<TextureView>,
}

impl Window {
    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }
}
