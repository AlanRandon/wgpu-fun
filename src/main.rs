use cgmath::prelude::*;
use cgmath::Vector2;
use futures_lite::future;
use rand::Rng;
use renderer::buffer::{Mesh, MeshBuilder, Vertex};
use std::f32::consts::{FRAC_PI_8, TAU};
use std::sync::{Arc, Mutex};
use winit::event::{ElementState, Event as WinitEvent, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;

mod collison;
mod renderer;

#[cfg(feature = "egl")]
#[link(name = "EGL")]
#[link(name = "GLESv2")]
extern "C" {}

struct LoseZone;

impl LoseZone {
    const HEIGHT: f32 = 0.1;
    const COLOR: [f32; 3] = [1., 0.6, 0.];

    fn push(&self, mesh: &mut MeshBuilder) {
        mesh.push(
            [
                [-10., -1.],
                [10., -1.],
                [10., -1. + Self::HEIGHT],
                [-10., -1. + Self::HEIGHT],
            ]
            .map(|position| Vertex {
                position,
                color: Self::COLOR,
            }),
            [0, 1, 2, 0, 2, 3],
        )
    }

    fn contains(&self, point: Vector2<f32>) -> bool {
        point.y < -1. + Self::HEIGHT
    }
}

struct Ball {
    position: Vector2<f32>,
    velocity: Vector2<f32>,
}

impl Ball {
    const SEGMENTS: usize = 20;
    const RADIUS: f32 = 0.05;

    fn push(&self, mesh: &mut MeshBuilder) {
        let Self { position, .. } = self;
        let Vector2 { x, y } = position;

        let vertices = std::iter::once(Vertex {
            position: [*x, *y],
            color: [1., 1., 1.],
        })
        .chain(
            (0..=Self::SEGMENTS)
                .map(|i| {
                    [
                        (i as f32 / Self::SEGMENTS as f32 * TAU).sin() * Self::RADIUS,
                        (i as f32 / Self::SEGMENTS as f32 * TAU).cos() * Self::RADIUS,
                    ]
                })
                .map(|[vert_x, vert_y]| [vert_x + x, vert_y + y])
                .map(|[x, y]| Vertex {
                    position: [x, y],
                    color: [1., 1., 1.],
                }),
        )
        .collect::<Vec<_>>();

        let indices = (1..Self::SEGMENTS + 1)
            .flat_map(|i| [0u16, (i as u16 + 1) % vertices.len() as u16, i as u16])
            .collect::<Vec<_>>();

        mesh.push(vertices, indices)
    }
}

#[derive(Debug)]
struct Paddle {
    x: f32,
    /// A value in -1..=1 for the paddle's x velocity
    velocity: f32,
}

impl Paddle {
    const WIDTH: f32 = 0.4;
    const HEIGHT: f32 = 0.2;
    const Y: f32 = -0.7;
    const ANGLE_MULTIPLIER: f32 = FRAC_PI_8;
    const NORMAL_ANGLE_MULTIPLIER: f32 = FRAC_PI_8 / 2.;

    fn points(&self) -> [Vector2<f32>; 4] {
        let Self { x, velocity } = self;
        let angle = velocity * Self::ANGLE_MULTIPLIER;
        let (s, c) = angle.sin_cos();

        const FRAC_WIDTH_2: f32 = Paddle::WIDTH / 2.;
        const FRAC_HEIGHT_2: f32 = Paddle::HEIGHT / 2.;

        [
            [-FRAC_WIDTH_2, -FRAC_HEIGHT_2],
            [FRAC_WIDTH_2, -FRAC_HEIGHT_2],
            [FRAC_WIDTH_2, FRAC_HEIGHT_2],
            [-FRAC_WIDTH_2, FRAC_HEIGHT_2],
        ]
        .map(|[x, y]| [x * c - y * s, x * s + y * c])
        .map(|[vert_x, y]| [x + vert_x, y + Self::Y])
        .map(|v| v.into())
    }

    fn push(&self, mesh: &mut MeshBuilder) {
        mesh.push(
            self.points().map(|v| Vertex {
                position: [v.x, v.y],
                color: [1., 1., 1.],
            }),
            [0, 1, 2, 0, 2, 3],
        )
    }

    fn contains(&self, ball: &Ball) -> bool {
        let [a, b, c, d] = self.points();
        collison::circle_intersects_triangle(ball.position, Ball::RADIUS, a, b, c)
            | collison::circle_intersects_triangle(ball.position, Ball::RADIUS, a, c, d)
    }

    fn normal(&self) -> Vector2<f32> {
        let angle = self.velocity * Self::NORMAL_ANGLE_MULTIPLIER;
        let rotation: cgmath::Basis2<f32> = cgmath::Rotation2::from_angle(cgmath::Rad(angle));
        let velocity = rotation.rotate_vector(Vector2::unit_y());

        let angle = self.x * Self::NORMAL_ANGLE_MULTIPLIER;
        let rotation: cgmath::Basis2<f32> = cgmath::Rotation2::from_angle(cgmath::Rad(angle));
        let position = rotation.rotate_vector(Vector2::unit_y());

        velocity * 0.5 + position * 0.5
    }
}

#[derive(Debug, Clone, Copy)]
enum Event {
    Left(ElementState),
    Right(ElementState),
    Reset,
}

struct Controls {
    left: ElementState,
    right: ElementState,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("WGPU fun")
        .build(&event_loop)?;
    let window = Arc::new(window);

    let mut renderer = future::block_on(renderer::Renderer::new(window.as_ref()));
    let (event_send, event_recv) = crossbeam::channel::unbounded();

    let lose_zone = LoseZone;

    let mut paddle = Paddle {
        x: 0.,
        velocity: 0.,
    };

    let mut ball = Ball {
        position: [0., 0.7].into(),
        velocity: [0., 0.].into(),
    };

    let mut controls = Controls {
        left: ElementState::Released,
        right: ElementState::Released,
    };

    let mesh = Arc::new(Mutex::new(Mesh::builder()));
    let camera_x = Arc::new(Mutex::new(0.0));

    std::thread::spawn({
        let window = Arc::clone(&window);
        let mesh = Arc::clone(&mesh);
        let camera_x = Arc::clone(&camera_x);
        let event_send = event_send.clone();

        move || {
            let mut rng = rand::thread_rng();
            loop {
                for event in event_recv.try_iter() {
                    match event {
                        Event::Left(state) => controls.left = state,
                        Event::Right(state) => controls.right = state,
                        Event::Reset => {
                            ball.position = [0., 0.7].into();
                        }
                    }
                }

                match controls {
                    Controls {
                        left: ElementState::Pressed,
                        right: ElementState::Released,
                    } => {
                        paddle.velocity = (paddle.velocity - 0.05).max(-1.0);
                    }
                    Controls {
                        left: ElementState::Released,
                        right: ElementState::Pressed,
                    } => {
                        paddle.velocity = (paddle.velocity + 0.05).min(1.0);
                    }
                    _ => {
                        paddle.velocity *= 0.95;
                    }
                }

                paddle.x = (paddle.x + paddle.velocity / 20.).clamp(-5.5, 5.5);

                // gravity
                ball.velocity.y = ball.velocity.y + ball.velocity.y.clamp(-0.5, -0.1) * 0.01;

                if paddle.contains(&ball) {
                    ball.velocity += paddle.normal();
                    ball.velocity.x += ((rng.gen::<f32>() * 2.) - 0.5) * 0.01;
                }

                ball.velocity = ball.velocity.map(|x| x * 0.95);
                ball.velocity = ball.velocity.map(|i| i.clamp(-0.1, 0.1));

                ball.position += ball.velocity;
                ball.position.x = ball.position.x.clamp(-5.5, 5.5);

                if lose_zone.contains(ball.position) {
                    event_send.send(Event::Reset).unwrap();
                }

                *mesh.lock().unwrap() = {
                    let mut mesh = Mesh::builder();
                    lose_zone.push(&mut mesh);
                    paddle.push(&mut mesh);
                    ball.push(&mut mesh);
                    mesh
                };

                {
                    let mut camera_x = camera_x.lock().unwrap();
                    *camera_x = ((*camera_x * 10. + paddle.x) / 11.).clamp(-5.0, 5.0);
                }

                window.request_redraw();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    });

    event_loop.run(move |event, elwt| match event {
        WinitEvent::WindowEvent {
            ref event,
            window_id,
        } if window_id == renderer.window.id() => match event {
            WindowEvent::CloseRequested => elwt.exit(),
            WindowEvent::Resized(size) => renderer.resize(*size),
            WindowEvent::ScaleFactorChanged { .. } => {
                renderer.resize(renderer.window.inner_size());
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state, logical_key, ..
                },
                ..
            } => match logical_key {
                Key::Named(NamedKey::ArrowRight) => event_send.send(Event::Right(*state)).unwrap(),
                Key::Named(NamedKey::ArrowLeft) => event_send.send(Event::Left(*state)).unwrap(),
                Key::Named(NamedKey::Space) if state == &ElementState::Pressed => {
                    event_send.send(Event::Reset).unwrap()
                }
                Key::Named(NamedKey::Escape) => elwt.exit(),
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                let mesh = mesh.lock().unwrap().clone().build(&renderer.device);
                match renderer.render(mesh, *camera_x.lock().unwrap()) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        renderer.resize(renderer.size);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        elwt.exit();
                    }
                    Err(err) => {
                        eprintln!("{err:?}");
                    }
                };
            }
            _ => {}
        },
        _ => {}
    })?;

    Ok(())
}
