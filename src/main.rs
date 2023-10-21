use futures_lite::future;
use renderer::{Shape, Vertex};
use std::f32::consts::{FRAC_PI_8, TAU};
use wgpu::util::DeviceExt;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
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
    fn create_shape(&self, device: &wgpu::Device) -> Shape {
        let Self {
            x,
            y,
            width,
            height,
            color,
        } = self;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(
                &[
                    [*x, *y, 0.],
                    [x + width, *y, 0.],
                    [x + width, y + height, 0.],
                    [*x, y + height, 0.],
                ]
                .map(|position| Vertex {
                    position,
                    color: *color,
                }),
            ),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices = [0u16, 1, 2, 0, 2, 3];
        let index_count = indices.len() as u32;
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Shape {
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }
}

struct Ball {
    position: [f32; 2],
    velocity: [f32; 2],
}

impl Ball {
    const SEGMENTS: usize = 20;
    const RADIUS: f32 = 0.05;

    fn create_shape(&self, device: &wgpu::Device) -> Shape {
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ball Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices = (1..vertices.len())
            .flat_map(|i| [0u16, (i as u16 + 1) % vertices.len() as u16, i as u16])
            .collect::<Vec<_>>();
        let index_count = indices.len() as u32;
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ball Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Shape {
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }
}

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

    fn create_shape(&self, device: &wgpu::Device) -> Shape {
        let Self { x, velocity } = self;
        let angle = velocity * Self::ANGLE_MULTIPLIER;
        let (s, c) = angle.sin_cos();

        const FRAC_WIDTH_2: f32 = Paddle::WIDTH / 2.;
        const FRAC_HEIGHT_2: f32 = Paddle::HEIGHT / 2.;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Paddle Vertex Buffer"),
            contents: bytemuck::cast_slice(
                &[
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
            ),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices = [0u16, 1, 2, 0, 2, 3];
        let index_count = indices.len() as u32;
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Paddle Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Shape {
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }
}

struct Controls {
    left: ElementState,
    right: ElementState,
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("WGPU fun")
        .build(&event_loop)
        .unwrap();

    let mut renderer = future::block_on(renderer::Renderer::new(window));

    let lose_zone = Rectangle {
        x: -1.,
        y: -1.,
        width: 2.,
        height: 0.1,
        color: [1., 0.6, 0.],
    };
    let loze_zone_shape = lose_zone.create_shape(&renderer.device);

    let mut paddle = Paddle {
        x: 0.,
        velocity: 0.,
    };
    let mut paddle_shape = paddle.create_shape(&renderer.device);

    let ball = Ball {
        position: [0., 0.7],
        velocity: [0., 0.],
    };
    let ball_shape = ball.create_shape(&renderer.device);

    let mut controls = Controls {
        left: ElementState::Released,
        right: ElementState::Released,
    };

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == renderer.window.id() => match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(size) => renderer.resize(*size),
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                renderer.resize(**new_inner_size);
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(key),
                        ..
                    },
                ..
            } => match key {
                VirtualKeyCode::Right => controls.right = *state,
                VirtualKeyCode::Left => controls.left = *state,
                _ => {}
            },
            _ => {}
        },
        Event::RedrawRequested(window_id) if renderer.window.id() == window_id => {
            match renderer.render([&loze_zone_shape, &paddle_shape, &ball_shape]) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {
                    renderer.resize(renderer.size);
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    *control_flow = ControlFlow::Exit;
                }
                Err(err) => {
                    eprintln!("{err:?}");
                }
            };
        }
        Event::MainEventsCleared => {
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
            paddle_shape = paddle.create_shape(&renderer.device);
            renderer.window.request_redraw();
        }
        _ => {}
    });
}
