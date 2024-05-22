use std::collections::HashSet;

use flecs_ecs::prelude::*;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum Button {
    Key(PhysicalKey),
    Mouse(MouseButton),
}

impl From<KeyCode> for Button {
    fn from(value: KeyCode) -> Self {
        Button::Key(value.into())
    }
}

impl From<PhysicalKey> for Button {
    fn from(value: PhysicalKey) -> Self {
        Button::Key(value)
    }
}

impl From<MouseButton> for Button {
    fn from(value: MouseButton) -> Self {
        Button::Mouse(value)
    }
}

#[derive(Component, Default, Debug)]
pub struct Input {
    just_pressed: HashSet<Button>,
    pressed: HashSet<Button>,
    just_released: HashSet<Button>,
    scroll_x: f64,
    scroll_y: f64,
}

impl Input {
    pub(crate) fn process_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => match event.state {
                ElementState::Pressed => {
                    self.just_pressed.insert(event.physical_key.into());
                    self.pressed.insert(event.physical_key.into());
                }
                ElementState::Released => {
                    self.pressed.remove(&event.physical_key.into());
                    self.just_released.insert(event.physical_key.into());
                }
            },
            WindowEvent::MouseInput { state, button, .. } => {
                let button = *button;
                match state {
                    ElementState::Pressed => {
                        self.just_pressed.insert(button.into());
                        self.pressed.insert(button.into());
                    }
                    ElementState::Released => {
                        self.pressed.remove(&button.into());
                        self.just_released.insert(button.into());
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    self.scroll_x += *x as f64;
                    self.scroll_y += *y as f64;
                }
                MouseScrollDelta::PixelDelta(pos) => {
                    const PIXELS_PER_LINE: f64 = 20.0;
                    self.scroll_x += pos.x / PIXELS_PER_LINE;
                    self.scroll_y += pos.y / PIXELS_PER_LINE;
                }
            },
            _ => {}
        }
    }

    pub(crate) fn clear_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
    }

    pub fn pressed(&self, button: impl Into<Button>) -> bool {
        self.pressed.contains(&button.into())
    }

    pub fn just_pressed(&self, button: impl Into<Button>) -> bool {
        self.just_pressed.contains(&button.into())
    }

    pub fn just_released(&self, button: impl Into<Button>) -> bool {
        self.just_released.contains(&button.into())
    }

    pub fn scroll_x(&self) -> f64 {
        self.scroll_x
    }

    pub fn scroll_y(&self) -> f64 {
        self.scroll_y
    }
}

#[derive(Component, Default)]
pub struct Cursor {
    x: f64,
    y: f64,
    in_frame: bool,
}

impl Cursor {
    pub(crate) fn process_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorEntered { .. } => {
                self.in_frame = true;
            }
            WindowEvent::CursorLeft { .. } => {
                self.in_frame = false;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.x = position.x;
                self.y = position.y;
            }
            _ => {}
        }
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn in_frame(&self) -> bool {
        self.in_frame
    }
}
