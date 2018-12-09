use winit::{Event, EventsLoop, KeyboardInput, VirtualKeyCode, WindowEvent};

use jadis::config::Config;

use log::{info, error, debug, warn};

fn main() {
    let config_path = std::env::var("JADIS_CONFIG").unwrap_or("config.toml".to_owned());
    let config = Config::load_from_file(&config_path).unwrap_or_else(|err|{
        eprintln!("Unable to load config from {}, detail:", config_path);
        eprintln!("{:?}", err);
        eprintln!("Falling back on default config...");
        Default::default()
    });
    config.logging.setup_logging().expect("Failed to start logging!");
    info!("Config successfully loaded from {}", config_path);
    let mut events_loop = EventsLoop::new();
    let _window = config.window.build(&events_loop);

    info!("starting main loop");
    'main: loop {
        let mut quitting = false;
        events_loop.poll_events(|event| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => quitting = true,
                    WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                        ..
                    } => quitting = true,
                    _ => ()
                }

            }
        });

        if quitting {
            break 'main;
        }
    }
}
