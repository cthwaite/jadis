use crate::config::Config;
#[cfg(not(feature = "gl"))]
use gfx_backend::winit;
#[cfg(feature = "gl")]
use gfx_backend::glutin;

/// Window abstraction.
pub struct Window {
    #[cfg(not(feature = "gl"))]
    pub events_loop: winit::EventsLoop,
    #[cfg(feature = "gl")]
    pub events_loop: glutin::EventsLoop,
    #[cfg(not(feature = "gl"))]
    pub window: winit::Window,
    #[cfg(feature = "gl")]
    pub window: Option<glutin::GlWindow>,
}

impl Window {
    /// Create a new window from the passed Config.
    #[cfg(not(feature = "gl"))]
    pub fn new(config: &Config) -> Self {
        let events_loop = EventsLoop::new();
        let window = config
            .window
            .build(&events_loop)
            .expect("Failed to build window!");
        Window {
            events_loop,
            window,
        }
    }

    #[cfg(feature = "gl")]
    pub fn new(config: &Config) -> Self {
        use gfx_hal::format::{AsFormat, Rgba8Srgb as ColorFormat};
        let events_loop = glutin::EventsLoop::new();

        let window = {
            let builder = gfx_backend::config_context(
                glutin::ContextBuilder::new(),
                ColorFormat::SELF,
                None,
            )
            .with_vsync(true);
            glutin::GlWindow::new(config.window.get_builder(), builder, &events_loop)
                .unwrap()
        };
        Window {
            events_loop,
            window: Some(window),
        }
    }
}
