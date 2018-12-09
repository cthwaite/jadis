use crate::config::Config;
use winit::{self, EventsLoop};

/// Window abstraction.
pub struct Window {
    pub events_loop: EventsLoop,
    pub window: winit::Window,
}

impl Window {
    /// Create a new window from the passed Config.
    pub fn new(config: &Config) -> Self {
        let events_loop = EventsLoop::new();
        let window = config.window.build(&events_loop).expect("Failed to build window!");
        Window {
            events_loop,
            window
        }
    }
}