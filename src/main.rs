use jadis::context::{Context, InstanceWrapper};
use jadis::config::Config;
use jadis::input::{Blackboard, RootEventHandler};
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

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct UniformBlock {
    projection: [[f32; 4]; 4]
}


fn build_mesh(width: usize, height: usize) -> Vec<Vertex> {
    let mut mesh = Vec::with_capacity(width * height * 6);

    for x in 0..width {
        for y in 0..height {
            mesh.push(Vertex {
                position: [0.0 + x as f32, 0.0 + y as f32, 0.0],
                colour: [1.0, 0.0, 0.0, 1.0],
            });
            mesh.push(Vertex {
                position: [0.0 + x as f32, 1.0 + y as f32, 0.0],
                colour: [0.0, 1.0, 0.0, 1.0],
            });
            mesh.push(Vertex {
                position: [1.0 + x as f32, 0.0 + y as f32, 0.0],
                colour: [0.0, 0.0, 1.0, 1.0],
            });
            mesh.push(Vertex {
                position: [1.0 + x as f32, 0.0 + y as f32, 0.0],
                colour: [0.0, 0.0, 1.0, 1.0],
            });
            mesh.push(Vertex {
                position: [0.0 + x as f32, 1.0 + y as f32, 0.0],
                colour: [0.0, 1.0, 0.0, 1.0],
            });
            mesh.push(Vertex {
                position: [1.0 + x as f32, 1.0 + y as f32, 0.0],
                colour: [0.0, 1.0, 0.0, 1.0],
            });
        }
    }
    mesh
}

fn get_matrix_for_grid(width: usize, height: usize) -> [[f32; 4]; 4] {
    let width = width as f32;
    let height = height as f32;
    let left = -width / 2.0;
    let right = width / 2.0;
    let top = height / 2.0; 
    let bottom = -height / 2.0;
    let near = 0.0;
    let far = 1.0;
    [
        [2.0 / (right - left), 0.0, 0.0, -(right + left) / (right - left)],
        [0.0,  2.0 / (top - bottom), 0.0, -(top + bottom) / (top - bottom)],
        [0.0, 0.0, 2.0/(far-near), -(far + near) / (far - near)],
        [-1.0, -1.0, 0.0, 1.0],
    ]
}

struct MeshConsoleRenderer<B: gfx_hal::Backend> {
    pub render_pass: B::RenderPass,
    vert: ShaderHandle<B>,
    frag: ShaderHandle<B>,
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    mesh: Vec<Vertex>,
    vertex_buffer: jadis::buffer::Buffer<B>,
    projection: jadis::buffer::Buffer<B>,
    desc_set: B::DescriptorSet,
}


impl<B: gfx_hal::Backend> MeshConsoleRenderer<B> {
    pub unsafe fn new(context: &Context<B>) -> Self {
        use jadis::buffer::Buffer;
        let (vert, frag) = MeshConsoleRenderer::load_shaders(context);
        let render_pass = MeshConsoleRenderer::build_render_pass(context);
        let set_layout = context.device.create_descriptor_set_layout(
            &[DescriptorSetLayoutBinding {
                binding: 0,
                ty: DescriptorType::UniformBuffer,
                count: 1,
                stage_flags: ShaderStageFlags::VERTEX,
                immutable_samplers: false,
            }],
            &[],
        ).expect("Failed to create descriptor set layout!");

        let mut desc_pool = context.device.create_descriptor_pool(
            1, // maximum number of descriptor sets
            &[DescriptorRangeDesc {
                ty: DescriptorType::UniformBuffer,
                count: 1 // amount of space
            }]
        ).expect("Unable to create descriptor pool!");
        let desc_set = desc_pool.allocate_set(&set_layout).unwrap();
        let pipeline_layout = context.device
            .create_pipeline_layout(&[set_layout], &[])
            .expect("Failed to create pipeline layout!");

        
        let pipeline = MeshConsoleRenderer::build_pipeline(context, &vert, &frag, &render_pass, &pipeline_layout);
        let mesh = build_mesh(80, 50);
        let memory_types = &context.physical_device().memory_properties().memory_types;
        let vertex_buffer = Buffer::new(
            &context.device,
            &mesh,
            &memory_types,
            Properties::CPU_VISIBLE,
            buffer::Usage::VERTEX,
        ).expect("Unable to create vertex buffer!");
        let projection = Buffer::new_uniform(
            &context.device,
            &[UniformBlock {
                projection: get_matrix_for_grid(80, 50)
            }],
            &memory_types,
            Properties::CPU_VISIBLE
        ).expect("Unable to create uniform buffer!");
        context.device.write_descriptor_sets(vec![DescriptorSetWrite{
            set: &desc_set,
            binding: 0,
            array_offset: 0,
            descriptors: Some(Descriptor::Buffer(projection.buffer.as_ref().unwrap(), None..None))
        }]);
        MeshConsoleRenderer {
            vert,
            frag,
            render_pass,
            pipeline,
            pipeline_layout,
            mesh,
            vertex_buffer,
            projection,
            desc_set,            
        }
    }

    pub unsafe fn destroy(mut self, context: &Context<B>) {
        context.device.destroy_graphics_pipeline(self.pipeline);
        context.device.destroy_pipeline_layout(self.pipeline_layout);

        self.vertex_buffer.destroy(&context.device);
        self.projection.destroy(&context.device);
        context.device.destroy_render_pass(self.render_pass);

        self.vert.destroy(&context.device);
        self.frag.destroy(&context.device);
    }

    pub fn load_shaders(context: &Context<B>) -> (ShaderHandle<B>, ShaderHandle<B>) {
        #[cfg(os = "windows")]
        let vert_path = "assets\\mesh.vert";
        #[cfg(not(os = "windows"))]
        let vert_path = "assets/mesh.vert";
        let source = ShaderSource::from_glsl_path(vert_path).expect("Couldn't find fragment shader");
        let vert = ShaderHandle::new(&context.device, source).expect("Failed to load fragment shader");
        info!("loaded vertex shader");

        #[cfg(os = "windows")]
        let frag_path = "assets\\mesh.frag";
        #[cfg(not(os = "windows"))]
        let frag_path = "assets/mesh.frag";
        let source = ShaderSource::from_glsl_path(frag_path).expect("Couldn't find vertex shader");
        let frag = ShaderHandle::new(&context.device, source).expect("Failed to load vertex shader");
        info!("loaded fragment shader");

        (vert, frag)
    }

    pub fn build_render_pass(context: &Context<B>) -> B::RenderPass {
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

        unsafe {
            context.device
                .create_render_pass(&[colour_attachment], &[subpass], &[dependency])
                .unwrap()
        }
    }

    pub fn build_pipeline(context: &Context<B>, vert: &ShaderHandle<B>, frag: &ShaderHandle<B>, render_pass: &B::RenderPass, pipeline_layout: &B::PipelineLayout) -> B::GraphicsPipeline {
        let shader_entries = GraphicsShaderSet {
            vertex: vert.entry_point("main").unwrap(),
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(frag.entry_point("main").unwrap()),
        };

        let subpass = Subpass {
            index: 0,
            main_pass: render_pass
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
        unsafe {
            context.device.create_graphics_pipeline(&pipeline_desc, None)
                .unwrap()
        }
    }

    pub unsafe fn render<C, S>(&self,  command_buffer: &mut CommandBuffer<B, C, S>, framebuffers: &[B::Framebuffer], frame_index: u32, viewport: Viewport)
        where C: gfx_hal::queue::Supports<gfx_hal::queue::capability::Graphics>,
              S: gfx_hal::command::Shot {
        command_buffer.set_viewports(0, &[viewport.clone()]);
        command_buffer.set_scissors(0, &[viewport.rect]);
        command_buffer.bind_graphics_pipeline(&self.pipeline);
        command_buffer.bind_vertex_buffers(0, vec![(self.vertex_buffer.buffer.as_ref().unwrap(), 0)]);
        command_buffer.bind_graphics_descriptor_sets(&self.pipeline_layout, 0, vec![&self.desc_set], &[]);

        {
            let mut encoder = command_buffer.begin_render_pass_inline(
                &self.render_pass,
                &framebuffers[frame_index as usize],
                viewport.rect,
                &[ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0]))],
            );

            let num_vertices = self.mesh.len() as u32;
            encoder.draw(0..num_vertices, 0..1);
        }

        command_buffer.finish();
    }
} 

fn run_loop(config: &Config) {

    #[cfg(not(feature = "gl"))]
    let (mut window, instance, mut context) = {
        let instance = InstanceWrapper::new();
        let mut window = Window::new(&config);
        let mut context = instance.create_context(&window);
        (window, instance, context)
    };

    #[cfg(feature = "gl")]
    let (mut window, instance, mut context) = {
        let instance = InstanceWrapper::new();
        let mut window = Window::new(&config);
        let context = instance.create_context(window.window.take().unwrap());
        (window, instance, context)
    };

    use jadis::gfx_backend::Backend as ConcreteBackend;

    let mut renderer = unsafe {
        MeshConsoleRenderer::<ConcreteBackend>::new(&context)
    };

    let mut blackboard = Blackboard::default();
    let mut event_handler = RootEventHandler::default();


    let mut command_pool = context.create_command_pool();

    let clear_colours = &[ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0]))];


    let frame_semaphore = context.device.create_semaphore().unwrap();
    let present_semaphore = context.device.create_semaphore().unwrap();

    info!("starting main loop");
    let mut swapchain = SwapchainState::new(&mut context);
    let mut framebuffer_state = FramebufferState::new(&context, &renderer.render_pass, &mut swapchain);

    'main: loop {
        blackboard.reset();
        event_handler.reset();
        window.events_loop.poll_events(|event| event_handler.handle_event(event));
        event_handler.sync(&mut blackboard);

        if (blackboard.should_quit || blackboard.should_rebuild_swapchain) && framebuffer_state.is_some() {
            context.device.wait_idle().unwrap();
            unsafe { command_pool.reset() };

            framebuffer_state.destroy(&context.device);

            swapchain.destroy(&context.device);
        }

        if blackboard.should_quit {
            info!("got quit signal, breaking from 'main loop");
            break 'main;
        }

        if blackboard.should_rebuild_swapchain || framebuffer_state.is_none() {
            info!("rebuilding swapchain ({} | {})", blackboard.should_rebuild_swapchain, framebuffer_state.is_none());
            swapchain.rebuild(&mut context);

            framebuffer_state.rebuild_from_swapchain(&context, &renderer.render_pass, &mut swapchain);
        }

        let (_, framebuffers) = framebuffer_state.get_mut();
        let swapchain_itself = swapchain.swapchain.as_mut().unwrap();

        let frame_index: SwapImageIndex = unsafe {
            command_pool.reset();
            match swapchain_itself.acquire_image(!0, FrameSync::Semaphore(&frame_semaphore)) {
                Ok(i) => i,
                Err(_) => {
                    warn!("Rebuilding the swapchain because acquire_image errored");
                    blackboard.should_rebuild_swapchain = true;
                    continue 'main;
                }
            }
        };

        let mut cmd_buffer = command_pool.acquire_command_buffer::<gfx_hal::command::OneShot>();
        unsafe {
            renderer.render(
                &mut cmd_buffer,
                &framebuffers,
                frame_index,
                Viewport {
                    rect: Rect {
                        x: 0, y: 0,
                        w: swapchain.extent.width as i16,
                        h: swapchain.extent.height as i16,
                    },
                    depth: 0.0..1.0,
                }
            );
        }

        use std::iter;
        let submission = Submission {
            wait_semaphores: Some((&frame_semaphore, PipelineStage::BOTTOM_OF_PIPE)),
            signal_semaphores: iter::once(&present_semaphore),
            command_buffers: Some(&cmd_buffer),
        };

        let result = unsafe {
            context.queue_group.queues[0].submit(submission, None);
            swapchain_itself.present(
                &mut context.queue_group.queues[0],
                frame_index,
                vec![&present_semaphore],
            )
        };

        if result.is_err() {
            warn!("Rebuilding the swapchain because present errored");
            blackboard.should_rebuild_swapchain = true;
        }
    }

    let device = &context.device;

    unsafe {
        renderer.destroy(&context);

        device.destroy_command_pool(command_pool.into_raw());
        device.destroy_semaphore(frame_semaphore);
        device.destroy_semaphore(present_semaphore);
    }
}


fn load_config() -> Config {
    let config_path = std::env::var(JADIS_CONFIG_ENV)
                            .unwrap_or_else(|_| JADIS_CONFIG_DEFAULT_PATH.to_owned());
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

    run_loop(&config);
    info!("Done...");
}
