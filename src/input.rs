use std::sync::{Arc, Mutex};
use winit::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};

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
    pub fn reset(&mut self) {
        self.should_quit = false;
        self.should_rebuild_swapchain = false;
    }
}


/// Handler for input events coming from winit.
#[derive(Clone, Debug)]
pub struct InputHandler {
    blackboard: Arc<Mutex<Blackboard>>
}

impl InputHandler {
    pub fn new(blackboard: Arc<Mutex<Blackboard>>) -> Self {
        InputHandler {
            blackboard
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::CloseRequested => {
                    let blackboard = &mut self.blackboard.lock().unwrap();
                    blackboard.should_quit = true;
                },
                WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                } => {
                    let blackboard = &mut self.blackboard.lock().unwrap();
                    blackboard.should_quit = true;
                },
                WindowEvent::Resized(_) => {
                    let blackboard = &mut self.blackboard.lock().unwrap();
                    blackboard.should_rebuild_swapchain = true;
                }
                _ => ()
            }

        }
    }

}