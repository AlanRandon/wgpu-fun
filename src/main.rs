use futures_lite::future;
use renderer::buffer::{Mesh, MeshBuilder, Vertex};
use std::cell::UnsafeCell;
use std::f32::consts::{FRAC_PI_8, TAU};
use std::sync::Arc;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;

mod renderer;

#[cfg(feature = "egl")]
#[link(name = "EGL")]
#[link(name = "GLESv2")]
extern "C" {}

struct Rectangle {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: [f32; 3],
}

impl Rectangle {
    fn push(&self, mesh: &mut MeshBuilder) {
        let Self {
            x,
            y,
            width,
            height,
            color,
        } = self;

        mesh.push(
            [
                [*x, *y, 0.],
                [x + width, *y, 0.],
                [x + width, y + height, 0.],
                [*x, y + height, 0.],
            ]
            .map(|position| Vertex {
                position,
                color: *color,
            }),
            [0, 1, 2, 0, 2, 3],
        )
    }
}

struct Ball {
    position: [f32; 2],
    velocity: [f32; 2],
}

impl Ball {
    const SEGMENTS: usize = 20;
    const RADIUS: f32 = 0.05;

    fn push(&self, mesh: &mut MeshBuilder) {
        let Self { position, .. } = self;
        let [x, y] = position;

        let vertices = std::iter::once(Vertex {
            position: [*x, *y, 0.],
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
                    position: [x, y, 0.],
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

    fn push(&self, mesh: &mut MeshBuilder) {
        let Self { x, velocity } = self;
        let angle = velocity * Self::ANGLE_MULTIPLIER;
        let (s, c) = angle.sin_cos();

        const FRAC_WIDTH_2: f32 = Paddle::WIDTH / 2.;
        const FRAC_HEIGHT_2: f32 = Paddle::HEIGHT / 2.;

        mesh.push(
            [
                [-FRAC_WIDTH_2, -FRAC_HEIGHT_2],
                [FRAC_WIDTH_2, -FRAC_HEIGHT_2],
                [FRAC_WIDTH_2, FRAC_HEIGHT_2],
                [-FRAC_WIDTH_2, FRAC_HEIGHT_2],
            ]
            .map(|[x, y]| [x * c - y * s, x * s + y * c])
            .map(|[vert_x, y]| [x + vert_x, y + Self::Y])
            .map(|[x, y]| Vertex {
                position: [x, y, 0.],
                color: [1., 1., 1.],
            }),
            [0, 1, 2, 0, 2, 3],
        )
    }
}

struct Controls {
    left: ElementState,
    right: ElementState,
}

struct UnsafeRef<T>(UnsafeCell<T>);

unsafe impl<T: Sync> Sync for UnsafeRef<T> {}

impl<T> UnsafeRef<T> {
    fn update(&self, f: impl FnOnce(T) -> T) {
        let ptr = self.0.get();
        let mut data = unsafe { std::ptr::read(ptr) };
        data = f(data);
        unsafe { std::ptr::write(ptr, data) }
    }

    fn get(&self) -> &T {
        unsafe { self.0.get().as_ref().unwrap() }
    }

    fn new(data: T) -> Self {
        Self(UnsafeCell::new(data))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("WGPU fun")
        .build(&event_loop)?;
    let window = Arc::new(window);

    let mut renderer = future::block_on(renderer::Renderer::new(window.as_ref()));

    let lose_zone = Rectangle {
        x: -1.,
        y: -1.,
        width: 2.,
        height: 0.1,
        color: [1., 0.6, 0.],
    };

    let paddle = Arc::new(UnsafeRef::new(Paddle {
        x: 0.,
        velocity: 0.,
    }));

    let ball = Ball {
        position: [0., 0.7],
        velocity: [0., 0.],
    };

    let controls = Arc::new(UnsafeRef::new(Controls {
        left: ElementState::Released,
        right: ElementState::Released,
    }));

    std::thread::spawn({
        let window = Arc::clone(&window);
        let controls = Arc::clone(&controls);
        let paddle = Arc::clone(&paddle);
        move || loop {
            let controls = controls.get();

            paddle.update(|mut paddle| {
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
                paddle.x = (paddle.x + paddle.velocity / 20.).clamp(-1.0, 1.0);
                paddle
            });

            window.request_redraw();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    event_loop.run(move |event, elwt| match event {
        Event::WindowEvent {
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
                Key::Named(NamedKey::ArrowRight) => controls.update(|mut controls| {
                    controls.right = *state;
                    controls
                }),
                Key::Named(NamedKey::ArrowLeft) => controls.update(|mut controls| {
                    controls.left = *state;
                    controls
                }),
                Key::Named(NamedKey::Escape) => elwt.exit(),
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                let mut mesh = Mesh::builder();
                lose_zone.push(&mut mesh);
                paddle.get().push(&mut mesh);
                ball.push(&mut mesh);

                match renderer.render(mesh.build(&renderer.device)) {
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
