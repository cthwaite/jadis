use winit::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};

/// The InputHandler trait is used to react to, process and optionally modify or
/// cancel further propagation of incoming events.
pub trait InputHandler {
    /// Handle an event.
    /// At the discretion of the implementation, events passed to this method
    /// may be:
    /// - returned unmodified
    /// - returned in some modified state
    /// - not returned at all
    ///
    fn handle_event(&mut self, event: Event) -> Option<Event> {
        Some(event)
    }
}

/// Root handler for events. Stores `Event`s coming from winit::EventLoop in a
/// cache for propagation to `InputHandler`s via the `sync` method.
#[derive(Debug, Default)]
pub struct RootEventHandler {
    events: Vec<Event>,
}

impl RootEventHandler {
    /// Pass each Event in the cache to an InputHandler, saving the return value
    /// from each invocation of handle_event as the contents of a new cache.
    pub fn sync<R: InputHandler>(&mut self, receiver: &mut R) {
        let events = std::mem::replace(&mut self.events, vec![]);
        self.events = events
            .into_iter()
            .filter_map(|event| receiver.handle_event(event))
            .collect::<Vec<_>>();
    }

    /// Empty the Event cache.
    pub fn reset(&mut self) {
        self.events.clear();
    }

    /// Store an incoming Event in the cache.
    pub fn handle_event(&mut self, event: Event) {
        self.events.push(event);
    }
}

/// Simple blackboard for data used by the first-cut main loop.
#[derive(Clone, Debug)]
pub struct Blackboard {
    pub should_quit: bool,
    pub should_rebuild_swapchain: bool,
}

impl Default for Blackboard {
    fn default() -> Self {
        Blackboard {
            should_quit: false,
            should_rebuild_swapchain: false,
        }
    }
}

impl Blackboard {
    /// Reset the flags to false.
    pub fn reset(&mut self) {
        self.should_quit = false;
        self.should_rebuild_swapchain = false;
    }
}

impl InputHandler for Blackboard {
    /// Check for `Esc`, window resize, and window close events.
    fn handle_event(&mut self, event: Event) -> Option<Event> {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::CloseRequested => self.should_quit = true,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => self.should_quit = true,
                WindowEvent::Resized(_) => {
                    self.should_rebuild_swapchain = true;
                }
                _ => (),
            }
        }
        None
    }
}
