use flecs_ecs::prelude::*;
use flecs_wgpu_demo::Cursor as InputCursor;
use flecs_wgpu_demo::*;
use vello::{
    kurbo::{Affine, Vec2},
    peniko::{
        color::{AlphaColor, Oklch},
        Color,
    },
};
use winit::{event::MouseButton, keyboard::KeyCode};

// Type of shape to spawn
#[derive(Component, Debug)]
#[flecs(traits(Singleton))]
enum ShapeType {
    Line,
    Circle,
    Rectangle,
}

// Color to spawn shapes with, hue stored separately so it can easily be mutated
#[derive(Component)]
#[flecs(traits(Singleton))]
struct ShapeColor(Color, f32);

// Attached to shapes that are in the process of being created
#[derive(Component)]
struct Spawning;

// Created as a singleton when mid-pan
#[derive(Component)]
struct Panning {
    x: f64,
    y: f64,
}

// Trait for all types that can be spawned
trait Spawns: VelloShape {
    fn spawn_system(world: &World) {
        system!(
            world,
            &ShapeColor,
            &InputCursor(up),
            &VelloScene(up),
            &mut Fill,
            &Transform,
            &mut Self,
            Spawning
        )
        .each(|(color, cursor, scene, fill, tf, shape)| {
            let cursor_tf = scene.camera.inverse() * Affine::translate((cursor.x(), cursor.y()));
            let cursor_pos = cursor_tf.translation();
            shape.update(cursor_pos, tf);
            fill.color = color.0;
        });
    }

    fn update(&mut self, cursor: Vec2, tf: &Transform);
}

impl Spawns for Circle {
    fn update(&mut self, cursor: Vec2, tf: &Transform) {
        let delta = tf.translation() - cursor;
        self.radius = delta.length();
    }
}

impl Spawns for Rect {
    fn update(&mut self, cursor: Vec2, tf: &Transform) {
        let delta = tf.translation() - cursor;
        self.width = delta.x.abs() * 2.0;
        self.height = delta.y.abs() * 2.0;
    }
}

impl Spawns for Line {
    fn update(&mut self, cursor: Vec2, tf: &Transform) {
        let delta = cursor - tf.translation();
        self.x = delta.x;
        self.y = delta.y;
    }
}

#[derive(Component)]
struct ExampleModule;

impl Module for ExampleModule {
    fn module(world: &World) {
        world.module::<Self>("module");

        system!(
            "draw_ui",
            world,
            &mut TextWriter,
            &ShapeType,
            &ShapeColor,
            &Window(up),
            &mut VelloScene
        )
        .kind(flecs::pipeline::PostUpdate)
        .each(|(text, ty, color, window, scene)| {
            text.add(
                scene,
                Affine::translate((10.0, 4.0)),
                Color::WHITE,
                20.0,
                &format!("[1] Circle\n[2] Rectangle\n[3] Line\nCurrent: {:?}", ty),
            );

            text.add(
                scene,
                Affine::translate((10.0, window.height() as f64 - 60.0)),
                Color::WHITE,
                20.0,
                &format!("Shift + Scroll to change color.\nCurrent: "),
            );

            let color_preview = Rect::new(120.0, 16.0);
            let preview_tf = Affine::translate((150.0, window.height() as f64 - 18.0));
            color_preview.fill(scene, &Fill::new(color.0), preview_tf);
            color_preview.stroke(scene, &Stroke::new(3.0, Color::WHITE), preview_tf);
        });

        system!(
            "handle_input",
            world,
            &mut ShapeType,
            &mut ShapeColor,
            &Input,
            &InputCursor(up),
            &mut VelloScene
        )
        .each_entity(|e, (ty, color, input, cursor, scene)| {
            let world = e.world();
            let cursor_tf = scene.camera.inverse() * Affine::translate((cursor.x(), cursor.y()));
            let cursor_pos = cursor_tf.translation();
            if input.just_pressed(MouseButton::Left) && cursor.in_frame() {
                world.scope(e, |world| {
                    let shape = world
                        .entity()
                        .add(Spawning)
                        .set(Transform::translate(cursor_pos.x, cursor_pos.y))
                        .set(Fill::new(color.0));
                    match ty {
                        ShapeType::Line => shape.set(Line::new(0.0, 0.0)),
                        ShapeType::Circle => shape.set(Circle::new(0.0)),
                        ShapeType::Rectangle => shape.set(Rect::new(0.0, 0.0)),
                    };
                })
            }

            if input.just_released(MouseButton::Left) || !cursor.in_frame() {
                world.remove_all(Spawning)
            }

            if input.just_pressed(MouseButton::Right) {
                world.set(Panning {
                    x: cursor.x(),
                    y: cursor.y(),
                });
            }

            if input.just_released(MouseButton::Right) {
                world.remove(Panning::id());
            }

            if input.pressed(MouseButton::Right) {
                world.try_get::<&mut Panning>(|p| {
                    let delta = Vec2::new(cursor.x(), cursor.y()) - Vec2::new(p.x, p.y);
                    scene.camera = Affine::translate(delta) * scene.camera;
                    p.x = cursor.x();
                    p.y = cursor.y();
                });
            }

            if input.just_pressed(KeyCode::Digit1) {
                *ty = ShapeType::Circle;
            }

            if input.just_pressed(KeyCode::Digit2) {
                *ty = ShapeType::Rectangle;
            }

            if input.just_pressed(KeyCode::Digit3) {
                *ty = ShapeType::Line;
            }

            if input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight) {
                color.1 += (input.scroll_y() as f32) * 10.0;
                // Use Oklch color space: [Lightness, Chroma, Hue, Alpha]
                // L=0.8 (80% lightness), C=0.2 (moderate chroma), H=hue in degrees
                color.0 = AlphaColor::<Oklch>::new([0.8, 0.2, color.1, 1.0]).convert();
            } else {
                const BASE: f64 = 1.05;
                scene.camera = Affine::translate((cursor.x(), cursor.y()))
                    * Affine::scale(BASE.powf(input.scroll_y()))
                    * Affine::translate((-cursor.x(), -cursor.y()))
                    * scene.camera;
            }
        });

        // Create systems to handle shapes mid-creation
        Rect::spawn_system(world);
        Circle::spawn_system(world);
        Line::spawn_system(world);
    }
}

fn main() {
    let app = Application::new();

    app.world.import::<ExampleModule>();
    app.world
        .system_named::<()>("setup_scene")
        .kind(flecs::pipeline::OnStart)
        .run(|mut it| {
            let world = it.world();
            let window = world.target(MainWindow, Some(0));
            world
                .entity_named("scene")
                .child_of(window)
                .set(VelloScene::default());
            world.set(ShapeType::Circle);
            world.set(ShapeColor(
                AlphaColor::<Oklch>::new([0.8, 0.2, 180.0, 1.0]).convert(),
                180.0,
            ));
            while it.next() {}
        });

    app.run().unwrap();
}
