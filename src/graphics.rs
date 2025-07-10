use std::{fmt::Debug, ops::Deref, path::PathBuf, sync::Arc};

use anyhow::{Ok, Result};
use colored::Colorize;
use getset::Getters;
use image::GenericImageView;
use imgui::TextureId;
use strum::{EnumIter, IntoEnumIterator};
use tracing::info;
use wgpu::{
    Backends, Device, Extent3d, Queue, SurfaceConfiguration, Texture, TextureDescriptor,
    naga::FastHashMap,
};
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::application::ApplicationSimulationEvent;

#[derive(Getters, derive_new::new)]
pub struct SimulationGraphcisInterface<'window> {
    pub application_surface: wgpu::Surface<'window>,
    pub _gpu_handle: wgpu::Adapter,
    pub gpu_interface: wgpu::Device,
    pub gpu_queue: wgpu::Queue,
    pub surface_configuration: SurfaceConfiguration,
}

pub fn display_evailable_graphic_adapters(instance: &wgpu::Instance) {
    info!(
        " - Venfor list: (0x10DE = {}, 0x1002 = {}, 0x8086 = {}). ",
        "NVIDIA".bright_yellow(),
        "AMD".bright_yellow(),
        "Intel".bright_yellow()
    );
    info!(" ");
    instance
        .enumerate_adapters(Backends::all())
        .iter()
        .for_each(|gpu_handle| {
            let info = gpu_handle.get_info();
            info!(" + GPU Handle: [{}]", info.name.yellow());
            info!(" +-----------------------------------");
            info!(" + ");
            PhysicalAdapterProperty::iter()
                .for_each(|item| info!("{}", display_adapter_property(gpu_handle, item)));
        });
}

#[derive(Debug, EnumIter, strum_macros::Display)]
pub enum PhysicalAdapterProperty {
    #[strum(to_string = "Vendor")]
    Vendor,
    #[strum(to_string = "Device Type")]
    DeviceType,
    #[strum(to_string = "Backend")]
    Backend,
    #[strum(to_string = "Features which supports the GPU")]
    Features,
    #[strum(to_string = "Max Limits - indication of how powerful the GPU can be.")]
    Limits,
    #[strum(to_string = "Inegrated GPU")]
    Integrated,
}

fn display_adapter_property(adapter: &wgpu::Adapter, property: PhysicalAdapterProperty) -> String {
    let information: Box<dyn Debug>;
    match property {
        PhysicalAdapterProperty::Vendor => information = Box::new(adapter.get_info().vendor),
        PhysicalAdapterProperty::DeviceType => {
            information = Box::new(adapter.get_info().device_type)
        }
        PhysicalAdapterProperty::Backend => information = Box::new(adapter.get_info().backend),
        PhysicalAdapterProperty::Features => information = Box::new(adapter.features()),
        PhysicalAdapterProperty::Limits => information = Box::new(adapter.limits()),
        PhysicalAdapterProperty::Integrated => {
            information = Box::new(adapter.get_info().device_type)
        }
    };
    format!(
        " + {} : of Adapter: [{:?}]",
        property.to_string().yellow(),
        information.deref()
    )
    .to_string()
}

pub fn render(
    window_handle: Arc<Window>,
    graphics_interface: &SimulationGraphcisInterface,
    imgui_context: &mut imgui::Context,
    imgui_winit_platform: &mut imgui_winit_support::WinitPlatform,
    imgui_renderer: &mut imgui_wgpu::Renderer,
    _event_proxy: &mut EventLoopProxy<ApplicationSimulationEvent>,
    texture_map: &FastHashMap<&'static str, TextureId>,
) -> Result<()> {
    window_handle.request_redraw();

    /* imgui stuf */
    imgui_winit_platform
        .prepare_frame(imgui_context.io_mut(), &window_handle)
        .unwrap();
    let ui = imgui_context.frame();
    ui.main_menu_bar(|| {
        ui.image_button(
            "str_id",
            texture_map.get("tex.icon").unwrap().clone(),
            mint::Vector2 { x: 64., y: 64. },
        );
    });

    let output = graphics_interface
        .application_surface
        .get_current_texture()?;
    let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut command_ecoder =
        graphics_interface
            .gpu_interface
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encode"),
            });
    {
        let mut object_render_pass =
            command_ecoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Default object Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        /* rgb(32, 31, 34) */
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 3.2 / 255.,
                            g: 3.1 / 255.,
                            b: 3.4 / 255.,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

        imgui_winit_platform.prepare_render(ui, &window_handle);
        let imgui_data_buf = imgui_context.render();

        imgui_renderer.render(
            &imgui_data_buf,
            &graphics_interface.gpu_queue,
            &graphics_interface.gpu_interface,
            &mut object_render_pass,
        )?;
    }

    graphics_interface
        .gpu_queue
        .submit(std::iter::once(command_ecoder.finish()));
    output.present();

    Ok(())
}

/* this should be called in the init application state */
pub fn write_image_from_path_msaa_off(
    surface_conf: &SurfaceConfiguration,
    device: &Device,
    queue: &Queue,
    path: PathBuf,
) -> Result<Texture> {
    let image_load = image::open(path.clone())?;
    let size = Extent3d {
        width: image_load.dimensions().0,
        height: image_load.dimensions().1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&TextureDescriptor {
        label: Some(path.to_str().unwrap()),
        size: size.clone(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: surface_conf.format.clone(),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image_load.to_rgba8(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * image_load.dimensions().0),
            rows_per_image: Some(image_load.dimensions().1),
        },
        size,
    );
    Ok(texture)
}
