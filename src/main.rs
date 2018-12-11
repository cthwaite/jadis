use std::sync::{Arc, Mutex};

use jadis::context::{InstanceWrapper};
use jadis::config::Config;
use jadis::input::{Blackboard, InputHandler};
use jadis::shader::{ShaderHandle, ShaderSource};
use jadis::window::Window;
use jadis::swapchain::SwapchainState;

use jadis::hal_prelude::*;


use log::{info, warn/* error, debug, */};


static JADIS_CONFIG_ENV : &'static str = "JADIS_CONFIG";
static JADIS_CONFIG_DEFAULT_PATH : &'static str = "config.toml";



pub struct FramebufferState<B: gfx_hal::Backend> {
    framebuffers: Option<Vec<B::Framebuffer>>,
    image_views: Option<Vec<B::ImageView>>
}

impl<B: gfx_hal::Backend> FramebufferState<B> {
    pub fn new(context: &Context<B>, render_pass: &B::RenderPass, swap_state: &mut SwapchainState<B>) -> Self {
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

    pub fn rebuild_from_swapchain(&mut self, context: &Context<B>, render_pass: &B::RenderPass, swap_state: &mut SwapchainState<B>) {
        let (image_views, framebuffers) = match swap_state.back_buffer.take().unwrap() {
            Backbuffer::Images(images) => {
                let color_range = SubresourceRange {
                    aspects: Aspects::COLOR,
                    levels: 0..1,
                    layers: 0..1,
                };

                let image_views = context.map_to_image_views(
                    &images,
                    ViewKind::D2,
                    Swizzle::NO,
                    color_range,
                ).unwrap();
                let fbos = context.image_views_to_fbos(&image_views, &render_pass, swap_state.extent.clone()).unwrap();

                (image_views, fbos)
            },
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
            for framebuffer in framebuffers {
                device.destroy_framebuffer(framebuffer);
            }
        }
        if let Some(image_views) = self.image_views.take() {
            for image_view in image_views {
                device.destroy_image_view(image_view);
            }
        }
    }
}

fn run_loop(window: &mut Window) {
    let instance = InstanceWrapper::new();
    let mut context = instance.create_context(&window);

    let source = ShaderSource::from_glsl_path("assets\\tri.vert").expect("Couldn't find fragment shader");
    let mut vert = ShaderHandle::new(&context.device, source).expect("failed to load fragment shader");
    info!("loaded vertex shader");
    
    let source = ShaderSource::from_glsl_path("assets\\tri.frag").expect("Couldn't find vertex shader");
    let mut frag = ShaderHandle::new(&context.device, source).expect("failed to load vertex shader");
    info!("loaded fragment shader");
    
    let render_pass = {
        let colour_attachment = Attachment {
            format: Some(context.surface_colour_format),
            samples: 1,
            ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::Store),
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: Layout::Undefined..Layout::Present
        };

        let subpass = SubpassDesc {
            colors: &[(0, Layout::ColorAttachmentOptimal)],
            depth_stencil: None,
            inputs: &[],
            preserves: &[],
            resolves: &[]
        };

        let dependency = SubpassDependency {
            passes: SubpassRef::External..SubpassRef::Pass(0),
            stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT..PipelineStage::COLOR_ATTACHMENT_OUTPUT,
            accesses: Access::empty()..(Access::COLOR_ATTACHMENT_READ | Access::COLOR_ATTACHMENT_WRITE),
        };

        context.device.create_render_pass(&[colour_attachment], &[subpass], &[dependency]).unwrap()
    };


    let pipeline_layout = context.device.create_pipeline_layout(&[], &[]).unwrap();
    let pipeline = {
        let shader_entries = GraphicsShaderSet {
            vertex: vert.entry_point("main").unwrap(),
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(frag.entry_point("main").unwrap()),
        };

        let subpass = Subpass {
            index: 0,
            main_pass: &render_pass
        };

        let mut pipeline_desc = GraphicsPipelineDesc::new(shader_entries,
                                                            Primitive::TriangleList,
                                                            Rasterizer::FILL,
                                                            &pipeline_layout,
                                                            subpass);

        pipeline_desc.blender
                    .targets
                    .push(ColorBlendDesc(ColorMask::ALL, BlendState::ALPHA));

        backend.device.create_graphics_pipeline(&pipeline_desc, None)
            .unwrap()
    };


    let blackboard = Arc::new(Mutex::new(Blackboard::default()));
    let mut input_handler = InputHandler::new(blackboard.clone());


    let mut command_pool = context.create_command_pool(16);

    let clear_colours = &[ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0]))];


    let frame_semaphore = context.device.create_semaphore().unwrap();
    let present_semaphore = context.device.create_semaphore().unwrap();

    info!("starting main loop");
    let mut swapchain = SwapchainState::new(&mut context);
    let mut framebuffer_state = FramebufferState::new(&context, &render_pass, &mut swapchain);

    'main: loop {
        window.events_loop.poll_events(|event| input_handler.handle_event(event));

        let (should_quit, should_rebuild_swapchain) = {
            let bb = &mut blackboard.lock().unwrap();
            let ret = (bb.should_quit, bb.should_rebuild_swapchain);
            bb.reset();
            ret
        };
        if (should_quit ||should_rebuild_swapchain) && framebuffer_state.is_some() {
            context.device.wait_idle().unwrap();
            command_pool.reset();

            framebuffer_state.destroy(&context.device);

            swapchain.destroy(&context.device);
        }

        if should_quit {
            info!("got quit signal, breaking from 'main loop");
            break 'main;
        }

        if framebuffer_state.is_none() || should_rebuild_swapchain {
            info!("rebuilding swapchain");
            swapchain.rebuild(&mut context);

            framebuffer_state.rebuild_from_swapchain(&context, &render_pass, &mut swapchain);
        }

        let (_, framebuffers) = framebuffer_state.get_mut();
        let swapchain_itself = swapchain.swapchain.as_mut().unwrap();

        command_pool.reset();
        let frame_index: SwapImageIndex = {
            match swapchain_itself.acquire_image(!0, FrameSync::Semaphore(&frame_semaphore)) {
                Ok(i) => i,
                Err(_) => {
                    warn!("Rebuilding the swapchain because acquire_image errored");
                    blackboard.lock().unwrap().should_rebuild_swapchain = true;
                    continue 'main;
                }
            }
        };

        let finished_command_buffer =  {
            let mut command_buffer = command_pool.acquire_command_buffer(false);

            let viewport = Viewport {
                rect: Rect {
                    x: 0, y: 0,
                    w: swapchain.extent.width as i16,
                    h: swapchain.extent.height as i16,
                },
                depth: 0.0..1.0,
            };

            command_buffer.set_viewports(0, &[viewport.clone()]);
            command_buffer.set_scissors(0, &[viewport.rect]);
            command_buffer.bind_graphics_pipeline(&pipeline);

            {
                let mut encoder = command_buffer.begin_render_pass_inline(
                    &render_pass,
                    &framebuffers[frame_index as usize],
                    viewport.rect,
                    clear_colours,
                );

                encoder.draw(0..3, 0..1);
            }

            command_buffer.finish()
        };

        let submission = Submission::new()
                            .wait_on(&[(&frame_semaphore, PipelineStage::BOTTOM_OF_PIPE)])
                            .signal(&[&present_semaphore])
                            .submit(vec![finished_command_buffer]);

        context.queue_group.queues[0].submit(submission, None);

        let result = swapchain_itself.present(
            &mut context.queue_group.queues[0],
            frame_index,
            vec![&present_semaphore],
        );

        if result.is_err() {
            warn!("Rebuilding the swapchain because present errored");
            blackboard.lock().unwrap().should_rebuild_swapchain = true;
        }
    }

    let device = &context.device;

    device.destroy_graphics_pipeline(pipeline);
    device.destroy_pipeline_layout(pipeline_layout);


    device.destroy_render_pass(render_pass);

    device.destroy_command_pool(command_pool.into_raw());
    device.destroy_semaphore(frame_semaphore);
    device.destroy_semaphore(present_semaphore);

    vert.destroy(device);
    frag.destroy(device);
}


fn load_config() -> Config {
    let config_path = std::env::var(JADIS_CONFIG_ENV)
                            .unwrap_or(JADIS_CONFIG_DEFAULT_PATH.to_owned());
    let config = Config::load_from_file(&config_path).unwrap_or_else(|err|{
        eprintln!("Unable to load config from {}, detail:", config_path);
        eprintln!("{:?}", err);
        eprintln!("Falling back on default config...");
        Default::default()
    });
    config.logging.setup_logging().expect("Failed to start logging!");
    info!("Config successfully loaded from {}", config_path);
    config
}


fn main() {
    let config = load_config();
    let mut window = Window::new(&config);

    run_loop(&mut window);

    info!("Done...");
}
