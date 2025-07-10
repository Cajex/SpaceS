use std::sync::{Arc, MutexGuard};

use anyhow::{Ok, Result};
use getset::{Getters, Setters};
use imgui::{FontConfig, FontSource, TextureId, Ui};
use imgui_wgpu::TextureConfig;
use pollster::FutureExt;
use tracing::{info, warn};
use wgpu::{InstanceFlags, Surface, SurfaceConfiguration, naga::FastHashMap};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::Event,
    event_loop::{EventLoop, EventLoopProxy},
    keyboard::{self, KeyCode},
    window::{Window, WindowAttributes},
};

use crate::graphics::{self, SimulationGraphcisInterface};

pub enum ApplicationSimulationEvent {
    ApplicationDrawImguiGraphics(MutexGuard<'static, &'static mut Ui>),
}

#[derive(derive_new::new, Setters, Getters)]
pub struct ApplicationSimulationInterface<'w> {
    pub winit_window_handle: Arc<Window>,
    pub graphics_interface: Option<SimulationGraphcisInterface<'w>>,
    pub imgui_context: imgui::Context,
    pub imgui_platform: imgui_winit_support::WinitPlatform,
    pub imgui_renderer: imgui_wgpu::Renderer,
    pub event_proxy: EventLoopProxy<ApplicationSimulationEvent>,
    pub texture_map: FastHashMap<&'static str, TextureId>,
}

pub fn execute() -> Result<()> {
    tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_ansi(true)
        .with_file(true)
        .init();
    info!("Executing SpaceS simulation application...");
    self::enable_event_loop()?;
    Ok(())
}

pub fn enable_event_loop() -> Result<()> {
    let event_loop: EventLoop<ApplicationSimulationEvent> = EventLoop::with_user_event().build()?;
    let window = Arc::new(
        event_loop
            .create_window(
                WindowAttributes::default()
                    .with_active(true)
                    .with_inner_size(LogicalSize::new(1200, 600))
                    .with_decorations(false)
                    .with_resizable(false)
                    .with_title("SpaceS"),
            )
            .expect("Failed to construct main window."),
    );
    let graphics_interface = ApplicationSimulationInterface::on_enable_interface(window.clone())?;

    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);
    let mut imgui_platform = imgui_winit_support::WinitPlatform::new(&mut imgui_context);
    imgui_platform.attach_window(
        imgui_context.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Default,
    );

    imgui_context
        .fonts()
        .add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                size_pixels: 13.,
                ..Default::default()
            }),
        }]);

    let renderer_config = imgui_wgpu::RendererConfig {
        texture_format: graphics_interface.surface_configuration.format,
        ..Default::default()
    };

    let mut imgui_renderer = imgui_wgpu::Renderer::new(
        &mut imgui_context,
        &graphics_interface.gpu_interface,
        &graphics_interface.gpu_queue,
        renderer_config,
    );

    let tex_load = Arc::new(
        graphics::write_image_from_path_msaa_off(
            &graphics_interface.surface_configuration,
            &graphics_interface.gpu_interface,
            &graphics_interface.gpu_queue,
            "design/Hintergrund.png".parse()?,
        )
        .expect("failed to write texture to gpu."),
    );

    let mut texture_map = FastHashMap::default();
    let icon_texture_id = imgui_renderer.textures.insert(imgui_wgpu::Texture::new(
        &graphics_interface.gpu_interface,
        &imgui_renderer,
        TextureConfig {
            size: tex_load.size(),
            label: Some("ila"),
            format: Some(tex_load.format()),
            usage: tex_load.usage(),
            mip_level_count: tex_load.mip_level_count(),
            sample_count: tex_load.sample_count(),
            dimension: tex_load.dimension(),
            sampler_desc: wgpu::SamplerDescriptor {
                label: Some("Image Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            },
        },
    ));
    info!("load icon :[{:?}]", icon_texture_id);
    texture_map.insert("tex.icon", icon_texture_id);

    let mut application = ApplicationSimulationInterface::new(
        window,
        Some(graphics_interface),
        imgui_context,
        imgui_platform,
        imgui_renderer,
        event_loop.create_proxy(),
        texture_map,
    );

    event_loop.run_app(&mut application)?;
    Ok(())
}

impl<'a> ApplicationHandler<ApplicationSimulationEvent> for ApplicationSimulationInterface<'a> {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let abstract_deprecated_event: Event<()> = winit::event::Event::WindowEvent {
            window_id: window_id,
            event: event.clone(),
        };
        self.imgui_platform.handle_event(
            self.imgui_context.io_mut(),
            &self.winit_window_handle,
            &abstract_deprecated_event,
        );
        match event {
            winit::event::WindowEvent::RedrawRequested => {
                match graphics::render(
                    self.winit_window_handle.clone(),
                    self.graphics_interface.as_ref().unwrap(),
                    &mut self.imgui_context,
                    &mut self.imgui_platform,
                    &mut self.imgui_renderer,
                    &mut self.event_proxy,
                    &self.texture_map,
                ) {
                    Result::Ok(_) => {}
                    Err(_) => {}
                };
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                if let keyboard::PhysicalKey::Code(key_code) = event.physical_key {
                    self.on_key_input(key_code, event_loop);
                }
            }
            _ => {}
        }
    }
}

impl ApplicationSimulationInterface<'_> {
    pub fn on_enable_interface<'a>(window: Arc<Window>) -> Result<SimulationGraphcisInterface<'a>> {
        self::ApplicationSimulationInterface::enable_graphics_interface(window)
    }

    /* initializes the graphics interface for the simulation */
    pub fn enable_graphics_interface<'a>(
        window: Arc<Window>,
    ) -> Result<SimulationGraphcisInterface<'a>> {
        let backend_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: InstanceFlags::from_build_config(),
            backend_options: wgpu::BackendOptions::from_env_or_default(),
        });
        graphics::display_evailable_graphic_adapters(&backend_instance);
        let surface: Surface<'_> = backend_instance.create_surface(window.clone()).unwrap();
        let graphics_adapter = backend_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .block_on()?;
        match graphics_adapter.get_info().device_type {
            wgpu::DeviceType::IntegratedGpu
            | wgpu::DeviceType::VirtualGpu
            | wgpu::DeviceType::Cpu => {
                warn!(
                    "Your graphics card may not be capable enough to run the simulation on PowerPreference::HighPerformance. [{:?}]",
                    graphics_adapter.get_info().device_type
                );
            }
            _ => {}
        }
        let interface = graphics_adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("logical adapter interface"),
                required_features: wgpu::Features::empty(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .block_on()?;
        let surface_caps = surface.get_capabilities(&graphics_adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap();
        let surface_configuration = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: surface_caps.present_modes.first().unwrap().clone(),
            desired_maximum_frame_latency: 1,
            alpha_mode: surface_caps.alpha_modes.first().unwrap().clone(),
            view_formats: vec![],
        };
        surface.configure(&interface.0, &surface_configuration);

        Ok(SimulationGraphcisInterface::new(
            surface,
            graphics_adapter,
            interface.0,
            interface.1,
            surface_configuration,
        ))
    }

    pub fn on_key_input(
        &mut self,
        key_code: KeyCode,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        match key_code {
            KeyCode::Escape => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    pub fn imgui_graphical_interface() {}
}
