use gfx_hal;
use gfx_hal::SwapchainConfig;
use gfx_hal::image::Extent;
use gfx_hal::device::Device;

use log::{info};

use crate::context::Context;


pub struct SwapchainState<B: gfx_hal::Backend> {
    pub swapchain: Option<B::Swapchain>,
    pub back_buffer: Option<gfx_hal::Backbuffer<B>>,
    pub extent: Extent,
}

impl<B: gfx_hal::Backend> SwapchainState<B> {
    pub fn new(backend: &mut Context<B>) -> Self {
        let (caps, _, _) = backend.get_compatibility();
        let swap_config = SwapchainConfig::from_caps(&caps, backend.surface_colour_format);
        let extent = swap_config.extent.to_extent();
        let (swapchain, back_buffer) = backend.create_swapchain(swap_config, None);
        SwapchainState {
            swapchain: Some(swapchain),
            back_buffer: Some(back_buffer),
            extent,
        }
    }

    /// Check if the swapchain is in a valid state for drawing.
    pub fn is_valid(&self) -> bool {
        self.swapchain.is_some()
    }
    
    /// Rebuild the swapchain.
    pub fn rebuild(&mut self, backend: &mut Context<B>) {
        self.destroy(&backend.device);
        let (caps, _, _) = backend.get_compatibility();
        let swap_config = SwapchainConfig::from_caps(&caps, backend.surface_colour_format);
        let extent = swap_config.extent.to_extent();
        let (swapchain, back_buffer) = backend.create_swapchain(swap_config, None);
        self.swapchain = Some(swapchain);
        self.back_buffer = Some(back_buffer);
        info!("{:?}", extent);
        self.extent = extent;
    }

    /// Destroy the swapchain.
    pub fn destroy(&mut self, device: &B::Device) {
        if let Some(swapchain) = self.swapchain.take() {
            device.destroy_swapchain(swapchain);
        }
        self.back_buffer.take();
    }
}