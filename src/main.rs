use jadis::config::Config;
use jadis::input::InputHandler;
use jadis::backend::Backend;
use jadis::window::Window;

use log::{info, error, debug, warn};



fn run_loop(window: &mut Window) {
    let backend = Backend::new(&window);

    let mut input_handler = InputHandler::default();

    info!("starting main loop");
    'main: loop {
        window.events_loop.poll_events(|event| input_handler.handle_event(event));
        if input_handler.should_quit() {
            info!("got quit signal, breaking from 'main loop");
            break 'main;
        }
    }
}

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
    let mut window = Window::new(&config);

    run_loop(&mut window);

    info!("Done...");
}
