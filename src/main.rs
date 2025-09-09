//! 一个用 winit + egui + wgpu 渲染的简单计数器 Demo。
//!
//! 学点什么：
//! - Rust 基础（struct/impl/mut/match/Result）
//! - winit 0.29 事件循环（closure 形式）、窗口事件、重绘
//! - egui 0.28 即时模式 UI 与 egui-wgpu 渲染管线

use std::time::Instant;

use egui::Context as EguiContext;
use egui::ViewportId;
use egui_wgpu::{Renderer as EguiRenderer, wgpu};
use egui_winit::State as EguiWinitState;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

// 应用状态：简单计数器
struct CounterApp {
    count: i32,
}

impl Default for CounterApp {
    fn default() -> Self {
        Self { count: 0 }
    }
}

impl CounterApp {
    fn ui(&mut self, ctx: &EguiContext) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("winit + egui 计数器");
            ui.horizontal(|ui| {
                if ui.button("-1").clicked() {
                    self.count -= 1;
                }
                if ui.button("+1").clicked() {
                    self.count += 1;
                }
                if ui.button("重置").clicked() {
                    self.count = 0;
                }
            });
            ui.label(format!("当前计数: {}", self.count));
        });
    }
}

// WGPU 图形状态
struct GfxState<'w> {
    // 实例对象目前未直接使用，但保留其生命周期；用下划线前缀抑制未使用警告
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'w>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
}

impl<'w> GfxState<'w> {
    async fn new(window: &'w Window) -> Self {
        let size = window.inner_size();
        // 在 Windows 上优先使用 DX12，避免 Vulkan Loader 报错（例如第三方覆盖层缺失 JSON）。
        // 其他平台使用默认的 PRIMARY（Vulkan/Metal/GL 等）。
        let backends = if cfg!(target_os = "windows") {
            wgpu::Backends::DX12
        } else {
            wgpu::Backends::PRIMARY
        };
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let surface = instance.create_surface(window).expect("创建 Surface 失败");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("未找到合适的 GPU 适配器");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .expect("创建设备失败");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            wgpu::PresentMode::Mailbox
        } else if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            wgpu::PresentMode::Fifo
        } else {
            surface_caps.present_modes[0]
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        Self {
            _instance: instance,
            surface,
            device,
            queue,
            surface_config,
            size,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }
}

fn main() {
    env_logger::init();

    // 事件循环与窗口
    let event_loop = EventLoop::new().expect("创建事件循环失败");
    let window = WindowBuilder::new()
        .with_title("learn_egui - 计数器")
        .with_inner_size(PhysicalSize::new(900, 600))
        .build(&event_loop)
        .expect("创建窗口失败");

    // 初始化图形与 egui（注意：GfxState 持有对 window 的借用）
    let mut gfx = pollster::block_on(GfxState::new(&window));
    let egui_ctx = EguiContext::default();
    // 安装中文字体回退，避免 UI 中文显示为方块
    install_cjk_fonts(&egui_ctx);
    let mut egui_winit = EguiWinitState::new(
        egui_ctx.clone(),
        ViewportId::ROOT,
        &window,
        Some(window.scale_factor() as f32),
        None,
    );
    let mut egui_renderer = EguiRenderer::new(&gfx.device, gfx.surface_config.format, None, 1);
    let mut app = CounterApp::default();
    let mut last_frame = Instant::now();

    event_loop
        .run(|event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent { window_id, event } if window_id == window.id() => {
                    // 交给 egui 先处理输入
                    let response = egui_winit.on_window_event(&window, &event);
                    if response.repaint {
                        window.request_redraw();
                    }

                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(size) => gfx.resize(size),
                        WindowEvent::ScaleFactorChanged { .. } => {
                            gfx.resize(window.inner_size());
                        }
                        WindowEvent::RedrawRequested => {
                            let now = Instant::now();
                            let _dt = now - last_frame;
                            last_frame = now;

                            // 开始 egui 帧
                            let raw_input = egui_winit.take_egui_input(&window);
                            egui_ctx.begin_frame(raw_input);
                            app.ui(&egui_ctx);
                            let full_output = egui_ctx.end_frame();

                            // 细分网格
                            let clipped_primitives = egui_ctx
                                .tessellate(full_output.shapes, egui_ctx.pixels_per_point());

                            // 纹理更新
                            for (id, delta) in &full_output.textures_delta.set {
                                egui_renderer.update_texture(&gfx.device, &gfx.queue, *id, delta);
                            }
                            for id in &full_output.textures_delta.free {
                                egui_renderer.free_texture(id);
                            }

                            // 获取绘制目标
                            let surface_tex = match gfx.surface.get_current_texture() {
                                Ok(frame) => frame,
                                Err(err) => {
                                    log::warn!("获取表面纹理失败，重试配置: {err}");
                                    gfx.resize(gfx.size);
                                    return;
                                }
                            };
                            let view = surface_tex
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let mut encoder = gfx.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("encoder"),
                                },
                            );

                            let screen_desc = egui_wgpu::ScreenDescriptor {
                                size_in_pixels: [
                                    gfx.surface_config.width,
                                    gfx.surface_config.height,
                                ],
                                pixels_per_point: egui_ctx.pixels_per_point(),
                            };
                            egui_renderer.update_buffers(
                                &gfx.device,
                                &gfx.queue,
                                &mut encoder,
                                &clipped_primitives,
                                &screen_desc,
                            );

                            // 渲染 UI
                            {
                                let mut rpass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("egui-pass"),
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                                        r: 0.1,
                                                        g: 0.1,
                                                        b: 0.12,
                                                        a: 1.0,
                                                    }),
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        depth_stencil_attachment: None,
                                        occlusion_query_set: None,
                                        timestamp_writes: None,
                                    });
                                egui_renderer.render(&mut rpass, &clipped_primitives, &screen_desc);
                            }

                            gfx.queue.submit(std::iter::once(encoder.finish()));
                            surface_tex.present();

                            // 处理平台输出（剪贴板等）
                            egui_winit.handle_platform_output(&window, full_output.platform_output);
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
        .expect("事件循环失败");
}

// 为 egui 安装常见中文字体作为回退；若未找到系统字体，则保持默认设置
fn install_cjk_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};
    let mut fonts = FontDefinitions::default();

    // Windows 常见中文字体路径
    #[cfg(target_os = "windows")]
    let candidates: [&str; 8] = [
        "C:/Windows/Fonts/msyh.ttc", // 微软雅黑
        "C:/Windows/Fonts/msyh.ttf",
        "C:/Windows/Fonts/msyhbd.ttc",  // 微软雅黑 Bold
        "C:/Windows/Fonts/simhei.ttf",  // 黑体
        "C:/Windows/Fonts/simsun.ttc",  // 宋体
        "C:/Windows/Fonts/Deng.ttf",    // 等线
        "C:/Windows/Fonts/msjh.ttc",    // 微软正黑
        "C:/Windows/Fonts/NsimSun.ttc", // 新宋体
    ];

    #[cfg(not(target_os = "windows"))]
    let candidates: [&str; 0] = [];

    let mut loaded_key: Option<String> = None;
    for path in candidates.iter() {
        if let Ok(bytes) = std::fs::read(path) {
            let stem = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("cjk");
            let key = format!("cjk-{}", stem);
            fonts
                .font_data
                .insert(key.clone(), FontData::from_owned(bytes));
            loaded_key = Some(key);
            break;
        }
    }

    if let Some(key) = loaded_key {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, key.clone());
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, key);
        ctx.set_fonts(fonts);
    }
}
