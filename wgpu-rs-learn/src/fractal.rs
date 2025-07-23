use anyhow::{Context, Result};
use log::info;
use pollster::block_on;
use std::borrow::Cow;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// 对应 compute_mandelbrot.wgsl 中的 Params 结构体
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeParams {
    width: u32,
    height: u32,
    scale: f32,
    _padding1: u32,
    center: [f32; 2],
    max_iter: u32,
    _padding2: u32,
}

// 对应 render_mandelbrot.wgsl 中的 params vec4
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RenderParams {
    width: f32,
    height: f32,
    max_iter: f32,
    color_mode: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FractalType {
    Mandelbrot,
    Julia,
    BurningShip,
    Multibrot,
    Tricorn,
    Newton,
}

impl FractalType {
    fn all() -> &'static [FractalType] {
        &[
            FractalType::Mandelbrot,
            FractalType::Julia,
            FractalType::BurningShip,
            FractalType::Multibrot,
            FractalType::Tricorn,
            FractalType::Newton,
        ]
    }
    fn shader_src(&self) -> &'static str {
        match self {
            FractalType::Mandelbrot => include_str!("../shaders/mandelbrot.wgsl"),
            FractalType::Julia => include_str!("../shaders/julia.wgsl"),
            FractalType::BurningShip => include_str!("../shaders/burning_ship.wgsl"),
            FractalType::Multibrot => include_str!("../shaders/multibrot.wgsl"),
            FractalType::Tricorn => include_str!("../shaders/tricorn.wgsl"),
            FractalType::Newton => include_str!("../shaders/newton.wgsl"),
        }
    }
    fn name(&self) -> &'static str {
        match self {
            FractalType::Mandelbrot => "Mandelbrot",
            FractalType::Julia => "Julia",
            FractalType::BurningShip => "Burning Ship",
            FractalType::Multibrot => "Multibrot",
            FractalType::Tricorn => "Tricorn",
            FractalType::Newton => "Newton",
        }
    }
}

// App 结构体，用于持有所有GPU和窗口状态
struct App<'a> {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'a>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<wgpu::SurfaceConfiguration>,

    render_pipeline: Option<wgpu::RenderPipeline>,
    compute_pipeline: Option<wgpu::ComputePipeline>,

    render_bind_group: Option<wgpu::BindGroup>,
    compute_bind_group: Option<wgpu::BindGroup>,

    image_buffer: Option<wgpu::Buffer>,
    compute_params_buffer: Option<wgpu::Buffer>,
    render_params_buffer: Option<wgpu::Buffer>,

    compute_params: ComputeParams,

    fractal_type: FractalType,
    color_mode: u32,

    // 鼠标拖动与缩放支持
    dragging: bool,
    last_cursor: Option<(f64, f64)>,
}

impl App<'_> {
    fn new_with_params(width: u32, height: u32, scale: f32, max_iter: u32) -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            config: None,
            render_pipeline: None,
            compute_pipeline: None,
            render_bind_group: None,
            compute_bind_group: None,
            image_buffer: None,
            compute_params_buffer: None,
            render_params_buffer: None,
            fractal_type: FractalType::Mandelbrot,
            color_mode: 0,
            compute_params: ComputeParams {
                width,
                height,
                scale,
                center: [-0.0, 0.0],
                max_iter,
                _padding1: 0,
                _padding2: 0,
            },
            dragging: false,
            last_cursor: None,
        }
    }

    fn update_params(&mut self) {
        if let (Some(queue), Some(compute_params_buffer), Some(render_params_buffer)) = (
            &self.queue,
            &self.compute_params_buffer,
            &self.render_params_buffer,
        ) {
            let render_params = RenderParams {
                width: self.compute_params.width as f32,
                height: self.compute_params.height as f32,
                max_iter: self.compute_params.max_iter as f32,
                color_mode: self.color_mode as f32,
            };
            queue.write_buffer(
                compute_params_buffer,
                0,
                bytemuck::cast_slice(&[self.compute_params]),
            );
            queue.write_buffer(
                render_params_buffer,
                0,
                bytemuck::cast_slice(&[render_params]),
            );
        }
    }

    fn rebuild_compute_pipeline(&mut self) -> Result<()> {
        let device = self.device.as_ref().unwrap();
        let shader_src = self.fractal_type.shader_src();
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_src)),
        });

        let compute_bind_group_layout = self
            .compute_pipeline
            .as_ref()
            .unwrap()
            .get_bind_group_layout(0);

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        self.compute_pipeline = Some(compute_pipeline);
        Ok(())
    }

    // 初始化所有 wgpu 资源和管线
    async fn init_webgpu(&mut self) -> Result<()> {
        let window = self.window.as_ref().unwrap().clone();
        let size = window.inner_size();

        self.compute_params.width = size.width;
        self.compute_params.height = size.height;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .context("Failed to create surface")?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Failed to request adapter")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::default(),
            })
            .await
            .context("Failed to request device")?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // 加载着色器
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../shaders/render_fractal.wgsl"
            ))),
        });
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(self.fractal_type.shader_src())),
        });

        // 创建缓冲
        let image_buffer_size =
            (size.width * size.height * std::mem::size_of::<u32>() as u32) as wgpu::BufferAddress;
        let image_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Image Buffer"),
            size: image_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Compute Params Buffer"),
            contents: bytemuck::cast_slice(&[self.compute_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let render_params = RenderParams {
            width: size.width as f32,
            height: size.height as f32,
            max_iter: self.compute_params.max_iter as f32,
            color_mode: self.color_mode as f32,
        };
        let render_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Render Params Buffer"),
            contents: bytemuck::cast_slice(&[render_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 创建计算管线
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
                    resource: image_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: compute_params_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // 创建渲染管线
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Render Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: image_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: render_params_buffer.as_entire_binding(),
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[], // 顶点在着色器中生成，所以这里为空
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // 保存所有资源
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.render_pipeline = Some(render_pipeline);
        self.compute_pipeline = Some(compute_pipeline);
        self.image_buffer = Some(image_buffer);
        self.compute_params_buffer = Some(compute_params_buffer);
        self.render_params_buffer = Some(render_params_buffer);
        self.compute_bind_group = Some(compute_bind_group);
        self.render_bind_group = Some(render_bind_group);

        Ok(())
    }

    // 运行计算和渲染
    fn render(&mut self) {
        info!(
            "max_iter: {}, width: {}, height: {}, scale: {}, center: [{}, {}]",
            self.compute_params.max_iter,
            self.compute_params.width,
            self.compute_params.height,
            self.compute_params.scale,
            self.compute_params.center[0],
            self.compute_params.center[1]
        );

        let (
            device,
            queue,
            surface,
            config,
            compute_pipeline,
            render_pipeline,
            compute_bind_group,
            render_bind_group,
        ) = match (
            self.device.as_ref(),
            self.queue.as_ref(),
            self.surface.as_ref(),
            self.config.as_ref(),
            self.compute_pipeline.as_ref(),
            self.render_pipeline.as_ref(),
            self.compute_bind_group.as_ref(),
            self.render_bind_group.as_ref(),
        ) {
            (Some(d), Some(q), Some(s), Some(c), Some(cp), Some(rp), Some(cbg), Some(rbg)) => {
                (d, q, s, c, cp, rp, cbg, rbg)
            }
            _ => {
                log::error!("Render resources not initialized!");
                return;
            }
        };

        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                log::error!("Failed to get current texture: {e:?}");
                surface.configure(device, config);
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });

        // 计算通道
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(compute_pipeline);
            compute_pass.set_bind_group(0, compute_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                self.compute_params.width.div_ceil(16), // workgroup_size is 16
                self.compute_params.height.div_ceil(16), // workgroup_size is 16
                1,
            );
        }

        // 必须按顺序先计算再渲染

        // 渲染通道
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_bind_group(0, render_bind_group, &[]);
            // 绘制一个覆盖全屏的四边形（由6个顶点组成两个三角形）
            render_pass.draw(0..6, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("WGPU Mandelbrot")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.compute_params.width,
                self.compute_params.height,
            ));
        let window = event_loop.create_window(window_attributes).unwrap();
        self.window = Some(Arc::new(window));

        if let Err(e) = block_on(self.init_webgpu()) {
            log::error!("Failed to initialize WebGPU: {e:?}");
        } else {
            self.window.as_ref().unwrap().request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    return;
                }

                // 1. 先更新 compute_params
                self.compute_params.width = size.width;
                self.compute_params.height = size.height;
                self.update_params();

                // 2. 再更新 config
                let config = self.config.as_mut().unwrap();
                config.width = size.width;
                config.height = size.height;

                // 3. 重新配置 surface
                let surface = self.surface.as_ref().unwrap();
                let device = self.device.as_ref().unwrap();
                surface.configure(device, config);

                // 4. 重建 Image Buffer 和 Bind Group（因为尺寸变了）
                let image_buffer_size =
                    (size.width * size.height * std::mem::size_of::<u32>() as u32)
                        as wgpu::BufferAddress;
                let image_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Image Buffer"),
                    size: image_buffer_size,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Compute Bind Group"),
                    layout: &self
                        .compute_pipeline
                        .as_ref()
                        .unwrap()
                        .get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: image_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self
                                .compute_params_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                    ],
                });

                let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Render Bind Group"),
                    layout: &self
                        .render_pipeline
                        .as_ref()
                        .unwrap()
                        .get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: image_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self
                                .render_params_buffer
                                .as_ref()
                                .unwrap()
                                .as_entire_binding(),
                        },
                    ],
                });

                // 5. 更新 App 状态
                self.image_buffer = Some(image_buffer);
                self.compute_bind_group = Some(compute_bind_group);
                self.render_bind_group = Some(render_bind_group);

                // 6. 请求重绘
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.dragging = state == ElementState::Pressed;
                    if !self.dragging {
                        self.last_cursor = None;
                    }
                }
                if button == MouseButton::Right && state == ElementState::Pressed {
                    self.color_mode = (self.color_mode + 1) % 3; // 3种配色，可扩展
                    self.update_params();
                    self.window.as_ref().unwrap().request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.dragging {
                    if let Some((last_x, last_y)) = self.last_cursor {
                        let dx = position.x - last_x;
                        let dy = position.y - last_y;
                        self.compute_params.center[0] -= dx as f32 * self.compute_params.scale;
                        self.compute_params.center[1] += dy as f32 * self.compute_params.scale;
                        self.update_params();
                        self.window.as_ref().unwrap().request_redraw();
                    }
                    self.last_cursor = Some((position.x, position.y));
                } else {
                    self.last_cursor = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(pos) => pos.y,
                };
                let factor = if scroll > 0.0 { 0.8 } else { 1.25 };
                self.compute_params.scale *= factor as f32;
                self.update_params();
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ArrowUp) => {
                            self.compute_params.max_iter += 16;
                            self.update_params();
                            self.window.as_ref().unwrap().request_redraw();
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown) => {
                            self.compute_params.max_iter =
                                self.compute_params.max_iter.saturating_sub(16).max(1);
                            self.update_params();
                            self.window.as_ref().unwrap().request_redraw();
                        }
                        PhysicalKey::Code(KeyCode::ArrowLeft) => {
                            let all = FractalType::all();
                            let idx = all.iter().position(|&t| t == self.fractal_type).unwrap();
                            let new_idx = if idx == 0 { all.len() - 1 } else { idx - 1 };
                            self.fractal_type = all[new_idx];
                            info!("Switch to fractal: {}", self.fractal_type.name());
                            self.rebuild_compute_pipeline().unwrap();
                            self.window.as_ref().unwrap().request_redraw();
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight) => {
                            let all = FractalType::all();
                            let idx = all.iter().position(|&t| t == self.fractal_type).unwrap();
                            let new_idx = if idx + 1 == all.len() { 0 } else { idx + 1 };
                            self.fractal_type = all[new_idx];
                            info!("Switch to fractal: {}", self.fractal_type.name());
                            self.rebuild_compute_pipeline().unwrap();
                            self.window.as_ref().unwrap().request_redraw();
                        }
                        _ => {}
                    }
                }
            }
            _ => (),
        }
    }
}

impl<'a> Drop for App<'a> {
    fn drop(&mut self) {
        info!("Application is exiting");
        // 先手动 drop surface
        self.surface = None;
        // 再 drop window
        self.window = None;
        // 其它资源自动 drop
    }
}

fn main() {
    env_logger::init();
    info!("Starting application");
    let args: Vec<String> = std::env::args().collect();
    let width = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1024);
    let height = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(768);
    let scale = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3.0 / width as f32);
    let max_iter = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(256);
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new_with_params(width, height, scale, max_iter);
    event_loop.run_app(&mut app).unwrap();
}
