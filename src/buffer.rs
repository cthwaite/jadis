use gfx_hal::Backend;

/// Buffer data structure.
struct Buffer<B: gfx_hal::Backend> {
    buffer: Option<B::Buffer>,
    memory: Option<B::Memory>,
    size: usize,
}

impl<B: gfx_hal::Backend> Buffer<B> {
    /// Create, allocate and populate a new buffer.
    pub fn new<T: Copy>(device: &B::Device, data: &[T], memory_types: &[MemoryType], usage: buffer::Usage, properties: Properties) -> Self {
        let mut buf = Buffer::new_empty(device, data, memory_types, usage, properties);
        buf.fill(device, data);
        buf
    }

    /// Create a new empty buffer to hold entities of type T.
    pub fn new_empty<T: Copy>(device: &B::Device, memory_types: &[MemoryType], usage: buffer::Usage, properties: Properties) -> Self {
        let stride = ::std::mem::size_of::<Item>() as u64;
        let buffer_len = item_count as u64 * stride;
        
        /// TODO: return result.
        let unbound_buffer = device.create_buffer(buffer_len, usage).unwrap();
        let mem_req = device.get_buffer_requirements(&unbound);

        
        let upload_type = memory_types
            .iter()
            .enumerate()
            .find(|(id, ty)| {
                let type_supported = req.type_mask & (1_u64 << id) != 0;
                type_supported && ty.properties.contains(properties)
            })
            .map(|(id, _ty)| MemoryTypeId(id))
        /// TODO: return result.
            .expect("Could not find appropriate vertex buffer memory type.");
        /// TODO: return result.
        let buffer_memory = device.allocate_memory(upload_type, req.size).unwrap();
        let buffer = device
            .bind_buffer_memory(&buffer_memory, 0, unbound_buffer)
        /// TODO: return result.
            .unwrap();

        Buffer {
            buffer: Some(buffer),
            memory: Some(memory),
            size: mem_req.size,
        }
    }

    // Fill the buffer with data.
    pub fn fill<T: Copy>(&mut self, device: &B::Device, data: &[T]) {
        let stride = ::std::mem::size_of::<T>() as u64;
        let buffer_len = data.len() as u64 * stride;

        // TODO: check that incoming data fits in buffer.

        let mut dest = device.acquire_mapping_writer::<T>(&self.memory, 0..buffer_len)
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
