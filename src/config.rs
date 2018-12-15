use std::fs::File;
use std::io::{BufReader, Read};

use chrono;
use fern;
use log;
use serde_derive::{Deserialize, Serialize};
use toml;
use winit::{EventsLoop, WindowBuilder};

#[serde(default)]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub decorations: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            width: 800,
            height: 600,
            decorations: true,
        }
    }
}

impl WindowConfig {
    pub fn get_builder(&self) -> WindowBuilder {
        WindowBuilder::new()
            .with_title("jadis")
            .with_dimensions((self.width, self.height).into())
            .with_decorations(self.decorations)
    }
    pub fn build(&self, events_loop: &EventsLoop) -> Result<winit::Window, winit::CreationError> {
        self.get_builder().build(&events_loop)
    }
}

#[serde(default)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub log_file: Option<String>,
    pub log_stdout: bool,
    pub level_filter: log::LevelFilter,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            log_file: None,
            log_stdout: true,
            level_filter: log::LevelFilter::Debug,
        }
    }
}

impl LoggingConfig {
    pub fn setup_logging(&self) -> Result<(), fern::InitError> {
        let mut dispatch = fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{}[{}][{}] {}",
                    chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                    record.target(),
                    record.level(),
                    message
                ))
            })
            .level(self.level_filter);
        if self.log_stdout {
            dispatch = dispatch.chain(std::io::stdout());
        }

        if let Some(path) = &self.log_file {
            dispatch = dispatch.chain(fern::log_file(&path)?);
        }
        dispatch.apply()?;
        Ok(())
    }
}

#[serde(default)]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub window: WindowConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Config, toml::de::Error> {
        let config = match File::open(path) {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let mut config = String::new();
                reader.read_to_string(&mut config).unwrap();
                config
            }
            Err(err) => {
                eprintln!("{}", err);
                return Ok(Default::default());
            }
        };
        toml::from_str(&config)
    }
}
