use winit::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};

/// Handler for input events coming from winit.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputHandler {
    should_quit: bool
}

impl InputHandler {
    pub fn handle_event(&mut self, event: Event) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::CloseRequested => self.should_quit = true,
                WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                } => self.should_quit = true,
                _ => ()
            }

        }
    }
    
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
}