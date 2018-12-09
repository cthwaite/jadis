#[cfg(windows)]
// pub extern crate gfx_backend_dx12 as gfx_backend;
pub extern crate gfx_backend_vulkan as gfx_backend;
#[cfg(target_os = "macos")]
pub extern crate gfx_backend_metal as gfx_backend;
#[cfg(all(unix, not(target_os = "macos")))]
pub extern crate gfx_backend_vulkan as gfx_backend;

pub mod backend;
pub mod config;
pub mod input;
pub mod prelude;
pub mod shader;
pub mod window;