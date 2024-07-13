use deref_derive::{Deref, DerefMut};
use flecs_ecs::prelude::*;
use vello::kurbo::{Affine, Vec2};

use crate::VelloScene;

#[derive(Component)]
pub struct Fill {
    pub style: vello::peniko::Fill,
    pub color: vello::peniko::Color,
}

impl Fill {
    pub fn new(color: vello::peniko::Color) -> Self {
        Self {
            style: vello::peniko::Fill::NonZero,
            color,
        }
    }
}

#[derive(Component)]
pub struct Stroke {
    pub style: vello::kurbo::Stroke,
    pub color: vello::peniko::Color,
}

impl Stroke {
    pub fn new(width: f64, color: vello::peniko::Color) -> Self {
        Self {
            style: vello::kurbo::Stroke::new(width),
            color,
        }
    }
}

#[derive(Component, Deref, DerefMut, Default)]
pub struct Transform(pub Affine);

impl Transform {
    pub fn translate(x: f64, y: f64) -> Self {
        Self(Affine::translate(Vec2::new(x, y)))
    }
}

pub trait VelloShape: ComponentId {
    fn systems(world: &World) {
        system!(world, &mut VelloScene(up), &Stroke, &Transform, &Self).each(
            |(scene, stroke, transform, shape)| {
                shape.stroke(scene, stroke, scene.camera * **transform);
            },
        );

        system!(world, &mut VelloScene(up), &Fill, &Transform, &Self).each(
            |(scene, fill, transform, shape)| {
                shape.fill(scene, fill, scene.camera * **transform);
            },
        );
    }

    fn shape(&self) -> impl vello::kurbo::Shape;

    fn fill(&self, scene: &mut VelloScene, fill: &Fill, transform: impl Into<Affine>) {
        scene.fill(
            fill.style,
            transform.into(),
            fill.color,
            None,
            &self.shape(),
        );
    }

    fn stroke(&self, scene: &mut VelloScene, stroke: &Stroke, transform: impl Into<Affine>) {
        scene.stroke(
            &stroke.style,
            transform.into(),
            stroke.color,
            None,
            &self.shape(),
        );
    }
}

#[derive(Component)]
pub struct VelloShapeModule;

impl Module for VelloShapeModule {
    fn module(world: &World) {
        world.module::<Self>("module");

        Circle::systems(world);
        Rect::systems(world);
        Line::systems(world);
    }
}

#[derive(Component)]
pub struct Circle {
    pub radius: f64,
}

impl Circle {
    pub fn new(radius: f64) -> Self {
        Self { radius }
    }
}

impl VelloShape for Circle {
    fn shape(&self) -> impl vello::kurbo::Shape {
        vello::kurbo::Circle::new((0.0, 0.0), self.radius)
    }
}

#[derive(Component)]
pub struct Rect {
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

impl VelloShape for Rect {
    fn shape(&self) -> impl vello::kurbo::Shape {
        vello::kurbo::Rect::new(
            -self.width / 2.0,
            -self.height / 2.0,
            self.width / 2.0,
            self.height / 2.0,
        )
    }
}

#[derive(Component)]
pub struct Line {
    pub x: f64,
    pub y: f64,
}

impl Line {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl VelloShape for Line {
    fn shape(&self) -> impl vello::kurbo::Shape {
        vello::kurbo::Line::new((0.0, 0.0), (self.x, self.y))
    }

    fn fill(&self, scene: &mut VelloScene, fill: &Fill, transform: impl Into<Affine>) {
        scene.stroke(
            &vello::kurbo::Stroke::new(10.0),
            transform.into(),
            fill.color,
            None,
            &self.shape(),
        );
    }
}
