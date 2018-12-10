use crate::prelude::*;
use crate::window::Window;

use log::{info};

pub type AdapterType = gfx_hal::Adapter<gfx_backend::Backend>;
pub type CommandPoolType = gfx_hal::CommandPool<gfx_backend::Backend, gfx_hal::queue::capability::Graphics>;
pub type DeviceType = gfx_backend::Device;
pub type PhysicalDeviceType = gfx_backend::PhysicalDevice;
pub type QueueType = gfx_hal::queue::family::QueueGroup<gfx_backend::Backend, gfx_hal::queue::capability::Graphics>;
pub type SurfaceCapabilities = gfx_hal::window::SurfaceCapabilities;
pub type ImageViewType = <gfx_backend::Backend as gfx_hal::Backend>::ImageView;
pub type ImageType = <gfx_backend::Backend as gfx_hal::Backend>::Image;

pub type SwapchainType = <gfx_backend::Backend as gfx_hal::Backend>::Swapchain;
pub type BackbufferType = gfx_hal::Backbuffer<gfx_backend::Backend>;

pub type SurfaceType = gfx_hal::Surface<gfx_backend::Backend>;

pub struct Backend {
    instance: gfx_backend::Instance,
    adapter: usize,
    available_adapters: Vec<AdapterType>,
    pub device: gfx_backend::Device,
    pub queue_group: QueueType,
    pub surface_colour_format: Format,
    pub surface_caps: SurfaceCapabilities,
    pub surface: <gfx_backend::Backend as gfx_hal::Backend>::Surface,
}

impl Backend {
    pub fn new(window: &Window) -> Self {
        let instance = gfx_backend::Instance::create("jadis", 1);
        let surface = instance.create_surface(&window.window);
        let mut available_adapters = instance.enumerate_adapters();
        
        for adapter in &available_adapters {
            info!("Found adapter: {} ({:?})", adapter.info.name, adapter.info.device_type);
        }

        let adapter = Backend::select_adapter(&available_adapters);
        let (device, physical_device, mut queue_group) = {
            let actual_adapter = &mut available_adapters[adapter];
            info!("==> Using adapter: {} ({:?})", actual_adapter.info.name, actual_adapter.info.device_type);
            let num_queues = 1;
            let (device, queue_group) = actual_adapter
                .open_with::<_, Graphics>(num_queues, |family| surface.supports_queue_family(family))
                .unwrap();
            let physical_device = &actual_adapter.physical_device;

            (device, physical_device, queue_group)
        };

        let (surface_caps, formats, _) = surface.compatibility(physical_device);
        let surface_colour_format = Backend::pick_surface_colour_format(formats);

        Backend {
            instance,
            adapter,
            available_adapters,
            device,
            queue_group,
            surface_colour_format,
            surface_caps,
            surface,
        }
    }

    pub fn create_command_pool(&self, max_buffers: usize) -> CommandPoolType {
        self.device.create_command_pool_typed(&self.queue_group, CommandPoolCreateFlags::empty(), max_buffers)
    }

    pub fn get_swapchain_config(&self) -> SwapchainConfig {
        SwapchainConfig::from_caps(&self.surface_caps, self.surface_colour_format)
    }

    pub fn create_swapchain(&mut self, config: SwapchainConfig, old_swapchain: Option<SwapchainType>) -> (SwapchainType, BackbufferType) {
        self.device.create_swapchain(&mut self.surface, config, old_swapchain)
    }

    pub fn map_to_image_views(
        &self,
        images: &[ImageType],
        view_kind: ViewKind,
        swizzle: Swizzle,
        range: SubresourceRange) -> Result<Vec<ImageViewType>, ViewError> {
        images.iter()
                .map(|image| self.create_image_view(image, view_kind, swizzle, range.clone()))
                .collect()
    }

    pub fn create_image_view(
        &self,
        image: &ImageType,
        view_kind: ViewKind,
        swizzle: Swizzle,
        range: SubresourceRange) -> Result<ImageViewType, ViewError> {
        self.device.create_image_view(image,
                                      view_kind,
                                      self.surface_colour_format,
                                      swizzle,
                                      range)
    }

    fn select_adapter(_adapters: &Vec<AdapterType>) -> usize {
        0
    }

    /// We pick a colour format from the list of supported formats. If there 
    /// is no list, we default to 'Rgba8Srgb'.
    fn pick_surface_colour_format(formats: Option<Vec<Format>>) -> Format {
        match formats {
                Some(choices) => choices.into_iter()
                                        .find(|format| format.base_format().1 == ChannelType::Srgb)
                                        .unwrap(),
                None => Format::Rgba8Srgb,
            }
    }
}
