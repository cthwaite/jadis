use crate::config::Config;
use winit::{self, EventsLoop};

/// Window abstraction.
pub struct Window {
    pub events_loop: EventsLoop,
    #[cfg(not(feature = "gl"))]
    pub window: winit::Window,
    #[cfg(feature = "gl")]
    pub window: Option<gfx_backend::glutin::GlWindow>,
}

impl Window {
    /// Create a new window from the passed Config.
    #[cfg(not(feature = "gl"))]
    pub fn new(config: &Config) -> Self {
        let events_loop = EventsLoop::new();
        let window = config.window.build(&events_loop).expect("Failed to build window!");
        Window {
            events_loop,
            window
        }
    }

    #[cfg(feature = "gl")]
    pub fn new(config: &Config) -> Self {
        use gfx_hal::format::{AsFormat, ChannelType, Rgba8Srgb as ColorFormat, Swizzle};
        let events_loop = EventsLoop::new();

        let window = {
            let builder =
                gfx_backend::config_context(gfx_backend::glutin::ContextBuilder::new(), ColorFormat::SELF, None)
                    .with_vsync(true);
            gfx_backend::glutin::GlWindow::new(config.window.get_builder(), builder, &events_loop).unwrap()
        };
        Window {
            events_loop,
            window: Some(window),
        }
    }
}