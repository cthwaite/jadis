use std::path::Path;
use std::fs::{self, File};
use std::error::Error;
use std::io::Read;
use crate::hal_prelude::*;
use glsl_to_spirv::ShaderType;


type EntryPointType<'a> = EntryPoint<'a, gfx_backend::Backend>;
type ShaderModuleType = <gfx_backend::Backend as gfx_hal::Backend>::ShaderModule;


pub fn compile_to_spirv(source: &str, shader_type: &ShaderType) -> Result<Vec<u8>, Box<Error>> {
    let mut compiled_file = glsl_to_spirv::compile(source, shader_type.clone())?;
    let mut compiled_bytes = Vec::new();
    compiled_file.read_to_end(&mut compiled_bytes)?;
    Ok(compiled_bytes)
}

/// 
pub enum ShaderSource {
    GLSLFile(ShaderType, String),
    GLSLRaw(ShaderType, String),
    SpirVFile(String),
    SpirVRaw(Vec<u8>),
}

impl ShaderSource {
    pub fn from_glsl_path(path: &str) -> Option<ShaderSource> {
        let sys_path = Path::new(path);
        let shader_type = sys_path.extension().and_then(|ext| {
            match ext.to_string_lossy().as_ref() {
                "vert" => Some(ShaderType::Vertex),
                "vs" => Some(ShaderType::Vertex),
                "frag" => Some(ShaderType::Fragment),
                "fs" => Some(ShaderType::Fragment),
                "geom" => Some(ShaderType::Geometry),
                "gs" => Some(ShaderType::Geometry),
                _ => None,
            }
        });
        if shader_type.is_none() {
            return None;
        }
        Some(ShaderSource::GLSLFile(shader_type.unwrap(), path.to_owned()))
    }
}

pub struct ShaderHandle {
    source: ShaderSource,
    module: Option<ShaderModuleType>,
}



impl ShaderHandle {
    /// Create a new shader handle using the passed device.
    pub fn new(device: &gfx_backend::Device, source: ShaderSource) -> Result<Self, Box<Error>> {
        let module = match &source {
            ShaderSource::GLSLFile(shader_type, path) => {
                let source = fs::read_to_string(path)?;
                let compiled_bytes = compile_to_spirv(&source, shader_type)?;
                device.create_shader_module(&compiled_bytes)?
            },
            ShaderSource::GLSLRaw(shader_type, source) => {
                let compiled_bytes = compile_to_spirv(&source, shader_type)?;
                device.create_shader_module(&compiled_bytes)?
            },
            ShaderSource::SpirVFile(path) => {
                let mut file = File::open(path)?;
                let mut buf = Vec::new();
                let read_size = file.read_to_end(&mut buf)?;
                device.create_shader_module(&buf)?
            },
            ShaderSource::SpirVRaw(bytes) => {
                device.create_shader_module(&bytes)?
            }
        };
        Ok(ShaderHandle {
            source,
            module: Some(module),
        })
    }

    pub fn destroy(&mut self, device: &gfx_backend::Device) {
        let module = std::mem::replace(&mut self.module, None);
        if let Some(module) = module {
            device.destroy_shader_module(module);
        }
    }

    pub fn entry_point<'e, 's: 'e>(&'s self, entry: &'e str) -> Option<EntryPointType<'e>> {
        if let Some(module) = &self.module {
            return Some(EntryPointType {
                entry,
                module: &module,
                specialization: Default::default(),
            });
        }
        None
    }
}