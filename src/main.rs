use jadis::config::Config;
use jadis::input::InputHandler;
use jadis::backend::Backend;
use jadis::window::Window;

use jadis::prelude::*;
use jadis::shader::{ShaderHandle, ShaderSource};

use jadis::gfx_backend;

use log::{info, /* error, debug, warn*/};


static JADIS_CONFIG_ENV : &'static str = "JADIS_CONFIG";
static JADIS_CONFIG_DEFAULT_PATH : &'static str = "config.toml";


struct DummyPipeline {
    frag: ShaderHandle,
    vert: ShaderHandle,
    pipeline: <gfx_backend::Backend as gfx_hal::Backend>::GraphicsPipeline,
}

fn run_loop(window: &mut Window) {
    let mut backend = Backend::new(&window);

    let source = ShaderSource::from_glsl_path("assets\\tri.vert").expect("Couldn't find fragment shader");
    let mut vert = ShaderHandle::new(&backend.device, source).expect("failed to load fragment shader");
    info!("loaded vertex shader");
    
    let source = ShaderSource::from_glsl_path("assets\\tri.frag").expect("Couldn't find vertex shader");
    let mut frag = ShaderHandle::new(&backend.device, source).expect("failed to load vertex shader");
    info!("loaded fragment shader");
    let render_pass = {
        let colour_attachment = Attachment {
            format: Some(backend.surface_colour_format),
            samples: 1,
            ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::Store),
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: Layout::Undefined..Layout::Present
        };

        // A reder pass oculd have multiple subpasses; we're using one for now.
        let subpass = SubpassDesc {
            colors: &[(0, Layout::ColorAttachmentOptimal)],
            depth_stencil: None,
            inputs: &[],
            preserves: &[],
            resolves: &[]
        };

        // This expresses the dependencies between subpasses.
        // Again, we only have one subpass for now.
        let dependency = SubpassDependency {
            passes: SubpassRef::External..SubpassRef::Pass(0),
            stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT..PipelineStage::COLOR_ATTACHMENT_OUTPUT,
            accesses: Access::empty()..(Access::COLOR_ATTACHMENT_READ | Access::COLOR_ATTACHMENT_WRITE),
        };

        backend.device.create_render_pass(&[colour_attachment], &[subpass], &[dependency])
    };


    let pipeline_layout = backend.device.create_pipeline_layout(&[], &[]);
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
    let swap_config = backend.get_swapchain_config();

    let extent = swap_config.extent.to_extent();

    let (mut swapchain, backbuffer) = backend.create_swapchain(swap_config, None);

    let (frame_views, framebuffers) = match backbuffer {
        // This arm is currently only used by the OpenGL backend,
        // which supplies an opaque framebuffer instead of giving us control
        // over individual images.
        Backbuffer::Framebuffer(fbo) => (vec![], vec![fbo]),
        Backbuffer::Images(images) => {
            let colour_range = SubresourceRange {
                aspects: Aspects::COLOR,
                levels: 0..1,
                layers: 0..1,
            };


            let image_views = backend.map_to_image_views(&images,
                                                        ViewKind::D2,
                                                        backend.surface_colour_format,
                                                        Swizzle::NO,
                                                        colour_range.clone()).unwrap();
            let fbos = image_views.iter()
                                  .map(|image_view| {
                                    backend.device.create_framebuffer(&render_pass, vec![image_view], extent)
                                          .unwrap()
                                  }).collect();
            (image_views, fbos)
        }
    };

    // The frame semaphore is used to allow us to wait for an image to be ready
    // before attempting to draw on it.
    let frame_semaphore = backend.device.create_semaphore();

    // The frame fence is used to allow us to wait until our draw commands have
    // finished before attempting to display the image.
    let frame_fence = backend.device.create_fence(false);


    let mut input_handler = InputHandler::default();

    info!("starting main loop");

    let mut command_pool = backend.create_command_pool(16);
    'main: loop {
        window.events_loop.poll_events(|event| input_handler.handle_event(event));
        if input_handler.should_quit() {
            info!("got quit signal, breaking from 'main loop");
            break 'main;
        }
         // Begin rendering.
        //
        backend.device.reset_fence(&frame_fence);
        command_pool.reset();

        // A swapchain contains multiple images - which one to draw on?
        // This returns the index of the image we use. The image may not be
        // ready for rendering yet, but will signal frame_semaphore when it is.
        let frame_index: SwapImageIndex = swapchain.acquire_image(!0, FrameSync::Semaphore(&frame_semaphore))
                                                   .expect("Failed to acquire frame!");

        // We have to build a command buffer before we send it off to be drawn.
        // We don't technically have to do this every frame, but if it changes
        // every frame, then we do.
        let finished_command_buffer = {
            // acquire_command_buffer(allow_pending_resubmit: bool)
            // you can only record to one command buffer per pool at the same time
            let mut command_buffer = command_pool.acquire_command_buffer(false);

            // Define a rectangle on screen to draw into: in this case, the
            // whole screen.
            let viewport = Viewport {
                rect: Rect {
                    x: 0, y: 0,
                    w: extent.width as i16,
                    h: extent.height as i16,
                },
                depth: 0.0..1.0,
            };

            command_buffer.set_viewports(0, &[viewport.clone()]);
            command_buffer.set_scissors(0, &[viewport.rect]);

            // Choose a pipeline.
            command_buffer.bind_graphics_pipeline(&pipeline);

            {
                // Clear the screen and begin the render pass.
                let mut encoder = command_buffer.begin_render_pass_inline(
                    &render_pass,
                    &framebuffers[frame_index as usize],
                    viewport.rect,
                    &[ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0]))]
                );

                // Draw the geometry. In this case 0..3 indicates the range of
                // vertices to be drawn. We have no vertex buffer as yet, so
                // this really just tells our shader to draw one triangle. The
                // specific vertices to draw at this point are encoded in the
                // shader itself.
                //
                // The 0..1 is the range of instances to draw. This is
                // irrelevant unless we're using instanced rendering.
                encoder.draw(0..3, 0..1);
            }

            // Finish building the command buffer; it is now ready to send to
            // the GPU.
            command_buffer.finish()
        };

        // This is what we submit to the command queeu. We wait until
        // frame_semaphore is signalled, at which point we know our chosen image
        // is available to draw on.
        let semaphore = (&frame_semaphore, PipelineStage::BOTTOM_OF_PIPE);
        let submission = Submission::new()
                            .wait_on(&[semaphore])
                            .submit(vec![finished_command_buffer]);

        // We submit the 'submission' to one of our command queues, which will
        // signal frame_fence once rendering is completed.
        backend.queue_group.queues[0].submit(submission, Some(&frame_fence));

        // We first wait for rendering to complete...
        backend.device.wait_for_fence(&frame_fence, !0);

        // ...and then present the image on screen.
        swapchain.present(&mut backend.queue_group.queues[0], frame_index, &[])
                 .expect("Failed to present");
    }
    backend.device.destroy_graphics_pipeline(pipeline);
    backend.device.destroy_pipeline_layout(pipeline_layout);

    for framebuffer in framebuffers {
        backend.device.destroy_framebuffer(framebuffer);
    }

    for image_view in frame_views {
        backend.device.destroy_image_view(image_view);
    }

    backend.device.destroy_render_pass(render_pass);
    backend.device.destroy_swapchain(swapchain);

    vert.destroy(&backend.device);
    frag.destroy(&backend.device);
    backend.device.destroy_command_pool(command_pool.into_raw());
    backend.device.destroy_fence(frame_fence);
    backend.device.destroy_semaphore(frame_semaphore);
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
