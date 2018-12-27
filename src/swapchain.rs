use gfx_hal;
use gfx_hal::device::Device;
use gfx_hal::image::Extent;
use gfx_hal::SwapchainConfig;

use crate::hal_prelude::*;

use log::info;

use crate::context::Context;

pub struct SwapchainState<B: gfx_hal::Backend> {
    pub swapchain: Option<B::Swapchain>,
    pub back_buffer: Option<gfx_hal::Backbuffer<B>>,
    pub extent: Extent,
}

impl<B: gfx_hal::Backend> SwapchainState<B> {
    pub fn new(backend: &mut Context<B>) -> Self {
        let (caps, _, _, _) = backend.get_compatibility();
        let swap_config = SwapchainConfig::from_caps(&caps, backend.surface_colour_format, Extent2D { height: 1024, width: 768 });
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
        let (caps, _, _, _) = backend.get_compatibility();
        let swap_config = SwapchainConfig::from_caps(&caps, backend.surface_colour_format, Extent2D { height: 1024, width: 768 });
        let extent = swap_config.extent.to_extent();
        let (swapchain, back_buffer) = backend.create_swapchain(swap_config, None);
        self.swapchain = Some(swapchain);
        self.back_buffer = Some(back_buffer);
        self.extent = extent;
    }

    /// Destroy the swapchain.
    pub fn destroy(&mut self, device: &B::Device) {
        if let Some(swapchain) = self.swapchain.take() {
            unsafe {device.destroy_swapchain(swapchain)};
        }
        self.back_buffer.take();
    }
}

pub struct FramebufferState<B: gfx_hal::Backend> {
    framebuffers: Option<Vec<B::Framebuffer>>,
    image_views: Option<Vec<B::ImageView>>,
}

impl<B: gfx_hal::Backend> FramebufferState<B> {
    pub fn new(
        context: &Context<B>,
        render_pass: &B::RenderPass,
        swap_state: &mut SwapchainState<B>,
    ) -> Self {
        let mut fbs = FramebufferState::new_empty();
        fbs.rebuild_from_swapchain(context, render_pass, swap_state);
        fbs
    }

    pub fn new_empty() -> Self {
        FramebufferState {
            framebuffers: None,
            image_views: None,
        }
    }

    pub fn rebuild_from_swapchain(
        &mut self,
        context: &Context<B>,
        render_pass: &B::RenderPass,
        swap_state: &mut SwapchainState<B>,
    ) {
        let (image_views, framebuffers) = match swap_state.back_buffer.take().unwrap() {
            Backbuffer::Images(images) => {
                let color_range = SubresourceRange {
                    aspects: Aspects::COLOR,
                    levels: 0..1,
                    layers: 0..1,
                };

                let image_views = context
                    .map_to_image_views(&images, ViewKind::D2, Swizzle::NO, color_range)
                    .unwrap();
                let fbos = context
                    .image_views_to_fbos(&image_views, &render_pass, swap_state.extent)
                    .unwrap();

                (image_views, fbos)
            }
            Backbuffer::Framebuffer(fbo) => (Vec::new(), vec![fbo]),
        };
        self.framebuffers = Some(framebuffers);
        self.image_views = Some(image_views);
    }

    pub fn is_some(&self) -> bool {
        self.framebuffers.is_some() && self.image_views.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.framebuffers.is_none() || self.image_views.is_none()
    }

    pub fn get_mut(&mut self) -> (&mut Vec<B::ImageView>, &mut Vec<B::Framebuffer>) {
        (
            self.image_views.as_mut().unwrap(),
            self.framebuffers.as_mut().unwrap(),
        )
    }

    pub fn destroy(&mut self, device: &B::Device) {
        if let Some(framebuffers) = self.framebuffers.take() {
            unsafe {
                for framebuffer in framebuffers {
                    device.destroy_framebuffer(framebuffer);
                }
            }
        }
        if let Some(image_views) = self.image_views.take() {
            unsafe {
                for image_view in image_views {
                    device.destroy_image_view(image_view);
                }
            }
        }
    }
}
