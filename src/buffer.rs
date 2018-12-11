use gfx_hal::Backend;
use gfx_hal::{
    Device,
    MemoryType, 
    adapter::MemoryTypeId,
    buffer,
    memory::{Barrier, Dependencies, Properties},
};

/// Buffer data structure.
pub struct Buffer<B: gfx_hal::Backend> {
    buffer: Option<B::Buffer>,
    memory: Option<B::Memory>,
    size: u64,
}

impl<B: gfx_hal::Backend> Buffer<B> {
    /// Create, allocate and populate a new buffer.
    pub fn new<T: Copy>(device: &B::Device, data: &[T], memory_types: &[MemoryType], usage: buffer::Usage, properties: Properties) -> Self {
        let mut buf = Buffer::new_empty::<T>(device, data.len(), memory_types, usage, properties);
        buf.fill(device, data);
        buf
    }

    /// Create a new empty buffer to hold entities of type T.
    pub fn new_empty<T: Copy>(device: &B::Device, size: usize, memory_types: &[MemoryType], usage: buffer::Usage, properties: Properties) -> Self {
        let stride = ::std::mem::size_of::<T>() as u64;
        let buffer_len = size as u64 * stride;
        
        // TODO: return result.
        let unbound_buffer = device.create_buffer(buffer_len, usage).unwrap();
        let mem_req = device.get_buffer_requirements(&unbound_buffer);
        
        // TODO: return result.
        let upload_type = memory_types
            .iter()
            .enumerate()
            .find(|(id, ty)| {
                let type_supported = mem_req.type_mask & (1_u64 << id) != 0;
                type_supported && ty.properties.contains(properties)
            })
            .map(|(id, _ty)| MemoryTypeId(id))
            .expect("Could not find appropriate vertex buffer memory type.");
        // TODO: return result.
        let buffer_memory = device.allocate_memory(upload_type, mem_req.size).unwrap();
        // TODO: return result.
        let buffer = device
            .bind_buffer_memory(&buffer_memory, 0, unbound_buffer)
            .unwrap();

        Buffer {
            buffer: Some(buffer),
            memory: Some(buffer_memory),
            size: mem_req.size,
        }
    }

    // Fill the buffer with data.
    pub fn fill<T: Copy>(&mut self, device: &B::Device, data: &[T]) {
        let stride = ::std::mem::size_of::<T>() as u64;
        let buffer_len = data.len() as u64 * stride;

        // TODO: check that incoming data fits in buffer.
        // TODO: check that we're currently in a valid state.

        let mut dest = device.acquire_mapping_writer::<T>(self.memory.as_ref().unwrap(), 0..buffer_len)
                            .unwrap();
        dest.copy_from_slice(data);
        device.release_mapping_writer(dest);
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