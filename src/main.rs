use std::sync::{Arc, Mutex};

use jadis::context::{Context, InstanceWrapper};
use jadis::config::Config;
use jadis::input::{Blackboard, InputHandler};
use jadis::shader::{ShaderHandle, ShaderSource};
use jadis::window::Window;
use jadis::swapchain::{FramebufferState, SwapchainState};

use jadis::hal_prelude::*;


use log::{info, warn/* error, debug, */};


static JADIS_CONFIG_ENV : &'static str = "JADIS_CONFIG";
static JADIS_CONFIG_DEFAULT_PATH : &'static str = "config.toml";

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    colour: [f32; 4]
}


const MESH: &[Vertex] = &[
    Vertex {
        position: [0.0, -1.0, 0.0],
        colour: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-1.0, 0.0, 0.0],
        colour: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        colour: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, -1.0, 0.0],
        colour: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        colour: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0, 0.0],
        colour: [1.0, 1.0, 0.0, 1.0],
    },
];


fn run_loop(window: &mut Window) {
    let instance = InstanceWrapper::new();
    let mut context = instance.create_context(&window);

    let source = ShaderSource::from_glsl_path("assets\\mesh.vert").expect("Couldn't find fragment shader");
    let mut vert = ShaderHandle::new(&context.device, source).expect("failed to load fragment shader");
    info!("loaded vertex shader");
    
    let source = ShaderSource::from_glsl_path("assets\\mesh.frag").expect("Couldn't find vertex shader");
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

        pipeline_desc.vertex_buffers.push(VertexBufferDesc {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            rate: 0
        });

        pipeline_desc.attributes.push(AttributeDesc {
            location: 0,
            binding: 0,
            element: Element {
                format: Format::Rgb32Float,
                offset: 0
            }
        });
        pipeline_desc.attributes.push(AttributeDesc {
            location: 1,
            binding: 0,
            element: Element {
                format: Format::Rgba32Float,
                offset: 12
            }
        });
        context.device.create_graphics_pipeline(&pipeline_desc, None)
            .unwrap()
    };

    use jadis::buffer::Buffer;
    let memory_types = &context.physical_device().memory_properties().memory_types;
    use jadis::gfx_backend::Backend as ConcreteBackend;
    let mut vertex_buffer : Buffer<ConcreteBackend> = Buffer::new(
        &context.device,
        &MESH,
        &memory_types,
        buffer::Usage::VERTEX,
        Properties::CPU_VISIBLE
    );

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
            command_buffer.bind_vertex_buffers(0, vec![(vertex_buffer.buffer.as_ref().unwrap(), 0)]);

            {
                let mut encoder = command_buffer.begin_render_pass_inline(
                    &render_pass,
                    &framebuffers[frame_index as usize],
                    viewport.rect,
                    clear_colours,
                );

                let num_vertices = MESH.len() as u32;
                encoder.draw(0..num_vertices, 0..1);
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
