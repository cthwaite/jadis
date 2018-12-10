#[cfg(windows)]
// pub extern crate gfx_backend_dx12 as gfx_backend;
pub extern crate gfx_backend_vulkan as gfx_backend;
#[cfg(target_os = "macos")]
pub extern crate gfx_backend_metal as gfx_backend;
#[cfg(all(unix, not(target_os = "macos")))]
pub extern crate gfx_backend_vulkan as gfx_backend;

pub mod context;
pub mod config;
pub mod hal_prelude;
pub mod input;
pub mod shader;
pub mod window;