use egui::{Context, CentralPanel, RawInput};
use egui_wgpu::wgpu::{self, Surface, Device, Queue};
use egui_winit::winit::event_loop::EventLoop;
use egui_winit::winit::window::WindowBuilder;
use egui_winit::winit::event::{Event, WindowEvent};
use winit::dpi::LogicalSize;
use pollster;
use std::time::Instant;

fn main() {
    // Create event loop for winit (windowing and event system)
    let event_loop = EventLoop::new();
    
    // Create a window using winit
    let window = WindowBuilder::new()
        .with_title("egui + wgpu")
        .with_inner_size(LogicalSize::new(1280, 720))
        .build(&event_loop)
        .unwrap();

    // Set up the wgpu backend (this handles rendering)
    let wgpu_state = pollster::block_on(wgpu_setup(window)); // Removed `mut`

    // Set up egui context
    let egui_ctx = Context::default();

    let start_time = Instant::now(); // Used for passing time to egui

    // Run the event loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // Construct the raw input for egui
                let raw_input = RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(
                            wgpu_state.window.inner_size().width as f32,
                            wgpu_state.window.inner_size().height as f32,
                        ),
                    )),
                    time: Some(start_time.elapsed().as_secs_f64()),
                    ..Default::default() // You can provide other inputs here like mouse, keyboard, etc.
                };

                // Handle UI updates every frame
                let _ = egui_ctx.run(raw_input, |ctx| { // Use `_` to ignore the return value
                    CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello, egui with wgpu!");
                        if ui.button("Click me!").clicked() {
                            println!("Button clicked!");
                        }
                    });
                });

                // Request window redraw
                wgpu_state.window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Render using wgpu
                render(&wgpu_state);
            }
            _ => {}
        }
    });
}

// Function to set up wgpu
async fn wgpu_setup(window: winit::window::Window) -> WgpuState { // Change the parameter to take Window by value
    let backend = wgpu::util::backend_bits_from_env().unwrap_or(wgpu::Backends::all());
    let instance = wgpu::Instance::new(backend);

    // Set up surface (the window)
    let surface = unsafe { instance.create_surface(&window) };

    // Set up device and queue
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_webgl2_defaults(),
                label: None,
            },
            None,
        )
        .await
        .unwrap();

    // Configure the surface (replace swap chain)
    let surface_format = surface.get_supported_formats(&adapter)[0];
    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        },
    );

    WgpuState {
        surface,
        device,
        queue,
        // surface_format: wgpu::TextureFormat, // Remove this line if not needed
        window,  // This is now correct as it takes ownership of the window
    }
}

// Function to render using wgpu
fn render(wgpu_state: &WgpuState) {
    let frame = wgpu_state
        .surface
        .get_current_texture()
        .expect("Timeout when acquiring next surface texture");

    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = wgpu_state
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });

    {
        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), // This clears to black
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        // Here you can add drawing commands, e.g., drawing shapes or UI elements
    }

    wgpu_state.queue.submit(std::iter::once(encoder.finish()));
    frame.present();
}

struct WgpuState {
    surface: Surface,
    device: Device,
    queue: Queue,
    // surface_format: wgpu::TextureFormat, // Remove this line if not needed
    window: winit::window::Window,  // Fixed the cloning issue
}