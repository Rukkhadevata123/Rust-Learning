use anyhow::{Context, Result};
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

// 1、定义三角形的顶点数据
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

// 2、定义应用程序结构体，保存窗口、GPU相关对象
struct App<'a> {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'a>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    config: Option<wgpu::SurfaceConfiguration>,
}

impl App<'_> {
    // 3、构造函数，初始化各字段为None
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            render_pipeline: None,
            vertex_buffer: None,
            config: None,
        }
    }

    // 4、初始化WebGPU相关资源
    async fn init_webgpu(&mut self) -> Result<()> {
        // 4.1、获取窗口并设置大小
        let window = self.window.as_ref().unwrap().clone();
        let mut size = window.inner_size();
        size.width = size.width.max(800);
        size.height = size.height.max(600);

        info!("Initializing wgpu instance and surface");
        // 4.2、创建wgpu实例和surface
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window)
            .context("Failed to create surface")?;

        info!("Requesting adapter");
        // 4.3、请求适配器
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Failed to request adapter")?;

        info!("Requesting device and queue");
        // 4.4、请求设备和队列
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .context("Failed to request device")?;

        // 4.5、获取表面能力和格式，配置表面
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

        // 4.6、加载着色器
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../shaders/graphic_shader.wgsl"
            ))),
        });

        // 4.7、创建渲染管线布局
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        // 4.8、定义顶点缓冲区布局
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

        // 4.9、创建渲染管线
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
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

        // 4.10、创建顶点缓冲区
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // 4.11、保存所有资源到结构体
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.render_pipeline = Some(render_pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.config = Some(config);

        info!("WebGPU initialized successfully");
        Ok(())
    }

    // 5、渲染函数，绘制三角形
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
            // 5.1、获取当前帧
            let frame = match surface.get_current_texture() {
                Ok(frame) => frame,
                Err(e) => {
                    error!("Failed to get current texture: {e}");
                    surface.configure(device, config);
                    return;
                }
            };

            // 5.2、创建视图和命令编码器
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
                // 5.3、开始渲染通道
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

                // 5.4、设置管线和顶点缓冲区，绘制
                render_pass.set_pipeline(pipeline);
                render_pass.set_vertex_buffer(0, buffer.slice(..));
                render_pass.draw(0..VERTICES.len() as u32, 0..1);
            }

            // 5.5、提交命令并展示帧
            queue.submit(std::iter::once(encoder.finish()));
            frame.present();
        }
    }
}

// 6、实现winit的ApplicationHandler，处理窗口事件
impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // 6.1、创建窗口
        let window_attributes = Window::default_attributes()
            .with_title("WebGPU Triangle")
            .with_inner_size(winit::dpi::LogicalSize::new(1024, 768));

        let window = event_loop.create_window(window_attributes).unwrap();
        self.window = Some(Arc::new(window));

        // 6.2、初始化WebGPU
        if let Err(e) = block_on(self.init_webgpu()) {
            error!("Failed to initialize WebGPU: {e:?}");
        }
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
                    // 跳过最小化或隐藏时的resize
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
// 7、桌面端入口函数
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
// 8、Web端入口函数
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
