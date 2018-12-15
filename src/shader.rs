use crate::hal_prelude::*;
use glsl_to_spirv::ShaderType;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

type ShaderModule<B> = <B as gfx_hal::Backend>::ShaderModule;

#[derive(Debug)]
pub enum ShaderHandleError {
    LoadFail(std::io::Error),
    ShaderFail(gfx_hal::device::ShaderError),
    Other(String),
}
impl From<String> for ShaderHandleError {
    fn from(err: String) -> Self {
        ShaderHandleError::Other(err)
    }
}

impl From<std::io::Error> for ShaderHandleError {
    fn from(err: std::io::Error) -> Self {
        ShaderHandleError::LoadFail(err)
    }
}

impl From<gfx_hal::device::ShaderError> for ShaderHandleError {
    fn from(err: gfx_hal::device::ShaderError) -> Self {
        ShaderHandleError::ShaderFail(err)
    }
}
impl fmt::Display for ShaderHandleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for ShaderHandleError {}

pub fn compile_to_spirv(
    source: &str,
    shader_type: &ShaderType,
) -> Result<Vec<u8>, ShaderHandleError> {
    let mut compiled_file = glsl_to_spirv::compile(source, shader_type.clone())?;
    let mut compiled_bytes = Vec::new();
    compiled_file.read_to_end(&mut compiled_bytes)?;
    Ok(compiled_bytes)
}

///
#[derive(Debug)]
pub enum ShaderSource {
    GLSLFile(ShaderType, String),
    GLSLRaw(ShaderType, String),
    SpirVFile(String),
    SpirVRaw(Vec<u8>),
}

impl ShaderSource {
    pub fn from_glsl_path(path: &str) -> Option<ShaderSource> {
        let sys_path = Path::new(path);
        let shader_type =
            sys_path
                .extension()
                .and_then(|ext| match ext.to_string_lossy().as_ref() {
                    "vert" => Some(ShaderType::Vertex),
                    "vs" => Some(ShaderType::Vertex),
                    "frag" => Some(ShaderType::Fragment),
                    "fs" => Some(ShaderType::Fragment),
                    "geom" => Some(ShaderType::Geometry),
                    "gs" => Some(ShaderType::Geometry),
                    _ => None,
                });
        if shader_type.is_none() {
            return None;
        }
        Some(ShaderSource::GLSLFile(
            shader_type.unwrap(),
            path.to_owned(),
        ))
    }
}

#[derive(Debug)]
pub struct ShaderHandle<B: gfx_hal::Backend> {
    source: ShaderSource,
    module: Option<ShaderModule<B>>,
}

impl<B: gfx_hal::Backend> ShaderHandle<B> {
    /// Create a new shader handle using the passed device.
    pub fn new(device: &B::Device, source: ShaderSource) -> Result<Self, ShaderHandleError> {
        let module = ShaderHandle::<B>::build_module(device, &source)?;
        Ok(ShaderHandle {
            source,
            module: Some(module),
        })
    }

    pub fn build_module(
        device: &B::Device,
        source: &ShaderSource,
    ) -> Result<ShaderModule<B>, ShaderHandleError> {
        let result = match source {
            ShaderSource::GLSLFile(shader_type, path) => {
                let source = fs::read_to_string(path)?;
                let compiled_spirv = compile_to_spirv(&source, shader_type)?;
                device.create_shader_module(&compiled_spirv)
            }
            ShaderSource::GLSLRaw(shader_type, source) => {
                let compiled_spirv = compile_to_spirv(&source, shader_type)?;
                device.create_shader_module(&compiled_spirv)
            }
            ShaderSource::SpirVFile(path) => {
                let mut file = File::open(path)?;
                let mut buf = Vec::new();
                let read_size = file.read_to_end(&mut buf)?;
                device.create_shader_module(&buf)
            }
            ShaderSource::SpirVRaw(bytes) => device.create_shader_module(&bytes),
        };
        result.map_err(|err| err.into())
    }

    pub fn rebuild(&mut self, device: &B::Device) -> Result<(), Box<Error>> {
        self.destroy(device);
        let module = ShaderHandle::<B>::build_module(device, &self.source)?;
        self.module = Some(module);
        Ok(())
    }

    pub fn destroy(&mut self, device: &B::Device) {
        let module = std::mem::replace(&mut self.module, None);
        if let Some(module) = module {
            device.destroy_shader_module(module);
        }
    }

    pub fn entry_point<'e, 's: 'e>(&'s self, entry: &'e str) -> Option<EntryPoint<'e, B>> {
        if let Some(module) = &self.module {
            return Some(EntryPoint {
                entry,
                module: &module,
                specialization: Default::default(),
            });
        }
        None
    }
}
