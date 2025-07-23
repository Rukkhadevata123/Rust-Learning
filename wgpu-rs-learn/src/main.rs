use anyhow::{Context, Result};
use futures::channel::oneshot;
use log::{error, info};
use pollster::block_on;
use std::borrow::Cow;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[cfg(not(target_arch = "wasm32"))]
use winit::event_loop::ControlFlow;

const BUFFER_SIZE: u32 = 1000;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    buffer_size: u32,
    scale: f32,
    offset: f32,
    _pad: f32,
}

// Triangle vertex data
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
];

// App struct to hold all GPU and window state
struct App<'a> {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'a>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    compute_pipeline: Option<wgpu::ComputePipeline>,
    compute_bind_group: Option<wgpu::BindGroup>,
    output_buffer: Option<wgpu::Buffer>,
    staging_buffer: Option<wgpu::Buffer>,
    vertex_buffer: Option<wgpu::Buffer>,
    config: Option<wgpu::SurfaceConfiguration>,
}

impl App<'_> {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            render_pipeline: None,
            compute_pipeline: None,
            compute_bind_group: None,
            output_buffer: None,
            staging_buffer: None,
            vertex_buffer: None,
            config: None,
        }
    }

    // 1. Initialize all wgpu resources and pipelines
    async fn init_webgpu(&mut self) -> Result<()> {
        // 1.1 Create window and set minimum size
        let window = self.window.as_ref().unwrap().clone();
        let mut size = window.inner_size();
        size.width = size.width.max(800);
        size.height = size.height.max(600);

        // 1.2 Create wgpu instance and surface
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window)
            .context("Failed to create surface")?;

        // 1.3 Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Failed to request adapter")?;

        // 1.4 Request device and queue
        let limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: 8,
            max_storage_buffer_binding_size: 1 << 24,
            max_compute_workgroup_size_x: 256,
            max_compute_workgroup_size_y: 8,
            max_compute_workgroup_size_z: 8,
            max_compute_invocations_per_workgroup: 256,
            max_compute_workgroups_per_dimension: 65535,
            ..wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .context("Failed to request device")?;

        // 1.5 Configure surface
        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // 1.6 Load shaders
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../shaders/graphic_shader.wgsl"
            ))),
        });
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../shaders/compute_shader.wgsl"
            ))),
        });

        // 1.7 Create render pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 15,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 14,
                },
            ],
        };
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: Default::default(),
                buffers: &[vertex_buffer_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: Default::default(),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: swapchain_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: swapchain_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // 1.8 Create vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // 1.9 Create compute buffers and bind group
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Output Buffer"),
            size: BUFFER_SIZE as wgpu::BufferAddress * size_of::<f32>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = Params {
            buffer_size: BUFFER_SIZE,
            scale: 1000.0,
            offset: 0.0,
            _pad: 0.0,
        };
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params Buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Compute Pipeline Layout"),
                    bind_group_layouts: &[&compute_bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: BUFFER_SIZE as wgpu::BufferAddress * size_of::<f32>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 1.10 Save all resources
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.render_pipeline = Some(render_pipeline);
        self.compute_pipeline = Some(compute_pipeline);
        self.compute_bind_group = Some(compute_bind_group);
        self.output_buffer = Some(output_buffer);
        self.staging_buffer = Some(staging_buffer);
        self.vertex_buffer = Some(vertex_buffer);
        self.config = Some(config);

        Ok(())
    }

    // 2. Run compute shader and read back results asynchronously
    fn run_compute(&mut self) {
        if let (
            Some(device),
            Some(queue),
            Some(compute_pipeline),
            Some(compute_bind_group),
            Some(output_buffer),
            Some(staging_buffer),
        ) = (
            &self.device,
            &self.queue,
            &self.compute_pipeline,
            &self.compute_bind_group,
            &self.output_buffer,
            &self.staging_buffer,
        ) {
            // 2.1 Encode compute and copy commands
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(compute_pipeline);
                cpass.set_bind_group(0, compute_bind_group, &[]);
                cpass.dispatch_workgroups(BUFFER_SIZE.div_ceil(64), 1, 1);
            }
            encoder.copy_buffer_to_buffer(
                output_buffer,
                0,
                staging_buffer,
                0,
                BUFFER_SIZE as wgpu::BufferAddress * size_of::<f32>() as wgpu::BufferAddress,
            );
            queue.submit(Some(encoder.finish()));

            // 2.2 Request async buffer mapping
            let buffer_slice = staging_buffer.slice(..);
            let (sender, receiver) = oneshot::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
                sender.send(v).expect("Failed to send map_async result");
            });

            // 2.3 Poll device to drive GPU and callback
            device
                .poll(wgpu::PollType::Wait)
                .expect("Failed to poll device");

            // 2.4 Await mapping result and print first 10 values
            block_on(async {
                if let Ok(Ok(())) = receiver.await {
                    let data = buffer_slice.get_mapped_range();
                    let result: &[f32] = bytemuck::cast_slice(&data);
                    println!("Compute results: {:?}", &result[..100]);
                    drop(data);
                    staging_buffer.unmap();
                } else {
                    eprintln!("Failed to map buffer");
                }
            });
        }
    }

    // 3. Render triangle
    fn render(&mut self) {
        if let (
            Some(surface),
            Some(device),
            Some(queue),
            Some(pipeline),
            Some(buffer),
            Some(config),
        ) = (
            &self.surface,
            &self.device,
            &self.queue,
            &self.render_pipeline,
            &self.vertex_buffer,
            &self.config,
        ) {
            let frame = match surface.get_current_texture() {
                Ok(frame) => frame,
                Err(e) => {
                    error!("Failed to get current texture: {e}");
                    surface.configure(device, config);
                    return;
                }
            };
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let another_texture = device.create_texture(&wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                label: Some("Color Target 1"),
                view_formats: &[],
            });
            let another_view = another_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                        Some(wgpu::RenderPassColorAttachment {
                            depth_slice: None,
                            view: &another_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                    ],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                render_pass.set_pipeline(pipeline);
                render_pass.set_vertex_buffer(0, buffer.slice(..));
                render_pass.draw(0..VERTICES.len() as u32, 0..1);
            }
            queue.submit(std::iter::once(encoder.finish()));
            frame.present();
        }
    }
}

// ApplicationHandler for winit event loop
impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("WebGPU Triangle")
            .with_inner_size(winit::dpi::LogicalSize::new(1024, 768));
        let window = event_loop.create_window(window_attributes).unwrap();
        self.window = Some(Arc::new(window));

        if let Err(e) = block_on(self.init_webgpu()) {
            error!("Failed to initialize WebGPU: {e:?}");
        } else {
            self.run_compute();
        }
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                self.render();
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    return;
                }
                if let (Some(surface), Some(device), Some(config)) =
                    (&self.surface, &self.device, &mut self.config)
                {
                    config.width = size.width;
                    config.height = size.height;
                    surface.configure(device, config);
                    self.window.as_ref().unwrap().request_redraw();
                }
            }
            _ => (),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    unsafe {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    info!("Starting application");
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_arch = "wasm32")]
pub fn main() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::spawn_local;
    use winit::platform::web::WindowAttributesExtWebSys;

    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("could not initialize logger");

    let event_loop = EventLoop::new().unwrap();

    // 8.1、获取HTML中的canvas
    let canvas = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    enum WasmAppState {
        Initializing,
        Ready(Rc<RefCell<App<'static>>>),
    }

    struct WasmApp {
        state: WasmAppState,
        canvas: web_sys::HtmlCanvasElement,
    }

    // 8.2、实现ApplicationHandler，处理Web端事件
    impl ApplicationHandler for WasmApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if let WasmAppState::Initializing = self.state {
                let window_attributes = Window::default_attributes()
                    .with_title("WebGPU Triangle")
                    .with_canvas(Some(self.canvas.clone()))
                    .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

                // 8.3、创建窗口
                let window = event_loop.create_window(window_attributes).unwrap();

                let app = Rc::new(RefCell::new(App::new()));
                app.borrow_mut().window = Some(std::sync::Arc::new(window));
                let app_clone = app.clone();

                // 8.4、异步初始化WebGPU
                let state_ptr = &mut self.state as *mut WasmAppState;

                spawn_local(async move {
                    if let Err(e) = app_clone.borrow_mut().init_webgpu().await {
                        log::error!("Failed to initialize WebGPU: {:?}", e);
                    } else {
                        // app_clone.borrow_mut().run_compute();
                        let window_arc = app_clone.borrow().window.as_ref().unwrap().clone();
                        unsafe { *state_ptr = WasmAppState::Ready(app_clone) };
                        info!("GPU Initialized. Requesting initial redraw.");
                        window_arc.request_redraw();
                    }
                });
            }
        }

        fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
            if let WasmAppState::Ready(app) = &self.state {
                app.borrow_mut().window_event(event_loop, id, event);
            }
        }
    }

    // 8.5、运行Web端应用
    let mut wasm_app = WasmApp {
        state: WasmAppState::Initializing,
        canvas,
    };
    event_loop.run_app(&mut wasm_app).unwrap();
}
