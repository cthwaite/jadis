use std::error::Error;
use std::fmt::{self, Display};

use gfx_hal::{
    adapter::MemoryTypeId,
    buffer,
    memory::{Properties},
    Backend, Device, MemoryType,
};


#[derive(Debug)]
pub enum BufferError {
    AllocationError(gfx_hal::device::AllocationError),
    BindError(gfx_hal::device::BindError),
    CreationError(gfx_hal::buffer::CreationError),
    MappingError(gfx_hal::mapping::Error),
    NoSuitableMemoryType,
}

impl Error for BufferError { }
impl Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BufferError::NoSuitableMemoryType => {
                write!(f, "Could not find appropriate vertex buffer memory type.")
            },
            _ => write!(f, "{:?}", self)
        }
    }
}

macro_rules! wrap_buf_error {
    ($src: ty, $dst: ident) => {
        impl From<$src> for BufferError {
            fn from(err: $src) -> Self {
                BufferError::$dst(err)
            }
        }
    }
}

wrap_buf_error!(gfx_hal::buffer::CreationError, CreationError);
wrap_buf_error!(gfx_hal::device::BindError, BindError);
wrap_buf_error!(gfx_hal::mapping::Error, MappingError);
wrap_buf_error!(gfx_hal::device::AllocationError, AllocationError);


/// Buffer data structure.
pub struct Buffer<B: gfx_hal::Backend> {
    pub buffer: Option<B::Buffer>,
    memory: Option<B::Memory>,
    size: u64,
}

impl<B: gfx_hal::Backend> Buffer<B> {
    /// Create, allocate and populate a new buffer.
    pub fn new<T: Copy>(device: &B::Device, data: &[T], memory_types: &[MemoryType], properties: Properties, usage: buffer::Usage) -> Result<Self, BufferError> {
        let mut buf = Buffer::new_empty::<T>(device, data.len(), memory_types, properties, usage)?;
        buf.fill(device, data)?;
        Ok(buf)
    }

    /// Create a new empty buffer to hold `size` objects of type T.
    pub fn new_empty<T: Copy>(device: &B::Device, size: usize, memory_types: &[MemoryType], properties: Properties, usage: buffer::Usage) -> Result<Self, BufferError> {
        let stride = ::std::mem::size_of::<T>() as u64;
        let buffer_len = size as u64 * stride;

        let unbound_buffer = device.create_buffer(buffer_len, usage)?;
        let mem_req = device.get_buffer_requirements(&unbound_buffer);

        let upload_type = memory_types
            .iter()
            .enumerate()
            .find(|(id, ty)| {
                let type_supported = mem_req.type_mask & (1_u64 << id) != 0;
                type_supported && ty.properties.contains(properties)
            })
            .map(|(id, _ty)| MemoryTypeId(id))
            .ok_or(BufferError::NoSuitableMemoryType)?;

        let buffer_memory = device.allocate_memory(upload_type, mem_req.size)?;
        let buffer = device.bind_buffer_memory(&buffer_memory, 0, unbound_buffer)?;

        Ok(Buffer {
            buffer: Some(buffer),
            memory: Some(buffer_memory),
            size: mem_req.size,
        })
    }

    /// Create, allocate and populate a new uniform buffer.
    pub fn new_uniform<T: Copy>(device: &B::Device, data: &[T], memory_types: &[MemoryType], properties: Properties) -> Result<Self, BufferError> {
        let mut buf = Buffer::new_empty::<T>(device, data.len(), memory_types, properties, buffer::Usage::UNIFORM)?;
        buf.fill(device, data);
        Ok(buf)
    }

    /// Get the size of the buffer.
    pub fn len(&self) -> u64 {
        self.size
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.memory.is_none() || self.size == 0
    }

    /// Check if the buffer is large enough to store the passed array.
    pub fn can_hold<T: Copy>(&self, data: &[T]) -> bool {
        let stride = ::std::mem::size_of::<T>() as u64;
        let buffer_len = data.len() as u64 * stride;
        buffer_len <= self.size
    }

    // Fill the buffer with data.
    pub fn fill<T: Copy>(&mut self, device: &B::Device, data: &[T]) -> Result<(), BufferError> {
        assert!(self.memory.is_some());
        let stride = ::std::mem::size_of::<T>() as u64;
        let buffer_len = data.len() as u64 * stride;

        assert!(buffer_len as u64 <= self.size);

        let memory = self.memory.as_ref().unwrap();

        // TODO: return result
        let mut dest = device.acquire_mapping_writer::<T>(memory, 0..buffer_len)?;
        dest.copy_from_slice(data);
        device.release_mapping_writer(dest);
        Ok(())
    }

    // Destroy the buffer.
    pub fn destroy(&mut self, device: &B::Device) {
        if let Some(buffer) = self.buffer.take() {
            device.destroy_buffer(buffer);
        }
        if let Some(memory) = self.memory.take() {
            device.free_memory(memory);
        }
    }
}


pub fn empty_buffer<B: Backend, Item>(
        device: &B::Device,
        memory_types: &[MemoryType],
        properties: Properties,
        usage: buffer::Usage,
        item_count: usize) -> (B::Buffer, B::Memory) {
    let stride = ::std::mem::size_of::<Item>() as u64;
    let buffer_len = item_count as u64 * stride;
    let unbound_buffer = device.create_buffer(buffer_len, usage).unwrap();
    let req = device.get_buffer_requirements(&unbound_buffer);

    let upload_type = memory_types.iter()
                                  .enumerate()
                                  .find(|(id, ty)| {
                                      let type_supported = req.type_mask & (1_u64 << id) != 0;
                                      type_supported && ty.properties.contains(properties)
                                  })
                                  .map(|(id, _ty)| MemoryTypeId(id))
                                  .expect("Could not find appropriate vertex buffer memory type.");
    let buffer_memory = device.allocate_memory(upload_type, req.size).unwrap();
    let buffer = device
        .bind_buffer_memory(&buffer_memory, 0, unbound_buffer)
        .unwrap();

    (buffer, buffer_memory)
}

pub fn fill_buffer<B: gfx_hal::Backend, T: Copy>(device: &B::Device, buffer_memory: &B::Memory, data: &[T]) {
    let stride = ::std::mem::size_of::<T>() as u64;
    let buffer_len = data.len() as u64 * stride;

    let mut dest = device.acquire_mapping_writer::<T>(&buffer_memory, 0..buffer_len)
                        .unwrap();
    dest.copy_from_slice(data);
    device.release_mapping_writer(dest);
}


pub fn create_buffer<B: Backend, Item: Copy>(
        device: &B::Device,
        memory_types: &[MemoryType],
        properties: Properties,
        usage: buffer::Usage,
        items: &[Item]) -> (B::Buffer, B::Memory) {
    let (empty_buffer, mut empty_buffer_memory) =
        empty_buffer::<B, Item>(device, memory_types, properties, usage, items.len());

    fill_buffer::<B, Item>(device, &mut empty_buffer_memory, items);

    (empty_buffer, empty_buffer_memory)
}
