use futures_lite::future;
use renderer::{Render, Vertex};
use wgpu::util::DeviceExt;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

mod renderer;

#[cfg(feature = "egl")]
#[link(name = "EGL")]
#[link(name = "GLESv2")]
extern "C" {}

struct Shape {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl Render for Shape {
    fn render<'a>(&'a self, encoder: &mut wgpu::RenderPass<'a>) {
        encoder.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        encoder.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        encoder.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut renderer = future::block_on(renderer::Renderer::new(window));

    use std::f32::consts::TAU;

    let vertices = std::iter::once(Vertex {
        position: [0., 0., 0.],
        color: [0., 0., 0.],
    })
    .chain((0..=16u8).map(f32::from).map(|i| Vertex {
        position: [(i / 16. * TAU).sin(), (i / 16. * TAU).cos(), 0.],
        color: [1., 1., 1.],
    }))
    .collect::<Vec<_>>();

    dbg!(&vertices);

    let vertex_buffer = renderer
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let indices = (1..vertices.len())
        .flat_map(|i| [0u16, (i as u16 + 1) % vertices.len() as u16, i as u16])
        .collect::<Vec<_>>();

    let index_buffer = renderer
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

    renderer.create_object(Shape {
        index_buffer,
        index_count: indices.len() as u32,
        vertex_buffer,
    });

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
            _ => {}
        },
        Event::RedrawRequested(window_id) if renderer.window.id() == window_id => {
            match renderer.render() {
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
            renderer.window.request_redraw();
        }
        _ => {}
    });
}
