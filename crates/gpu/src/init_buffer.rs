//! Buffers born holding their contents.
//!
//! A constructor that has only a `Device` fills a small constant buffer by
//! creating it mapped. That is the whole story on native, and on a browser's
//! WebGPU it is not: Chrome's software adapter refuses `mappedAtCreation`
//! after the first few (`RangeError: createBuffer failed, size (32) is too
//! large for the implementation when mappedAtCreation == true`, on a 32-byte
//! buffer, at the first window resize), and wgpu unwraps that error into a
//! panic. So on `wasm32` the contents go through the queue instead, which maps
//! nothing. The queue is registered once by the web entry point
//! ([`set_web_queue`]); changing every device-only constructor to take one
//! would touch most of `oag-post` and `oag-present` for a path only the
//! browser needs.
//!
//! Native code never takes the queue branch, so a native buffer is created
//! exactly as it was before this helper existed.

/// Creates a buffer holding `contents`. `COPY_DST` is added on `wasm32`, where
/// the queue writes it; native usage is left as given.
pub fn init(
    device: &wgpu::Device,
    label: &str,
    usage: wgpu::BufferUsages,
    contents: &[u8],
) -> wgpu::Buffer {
    #[cfg(target_arch = "wasm32")]
    if let Some(queue) = web::queue() {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: contents.len() as u64,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buffer, 0, contents);
        return buffer;
    }
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: contents.len() as u64,
        usage,
        mapped_at_creation: true,
    });
    buffer
        .slice(..)
        .get_mapped_range_mut()
        .expect("a freshly mapped buffer maps")
        .copy_from_slice(contents);
    buffer.unmap();
    buffer
}

/// Registers the queue [`init`] writes through on the web.
#[cfg(target_arch = "wasm32")]
pub fn set_web_queue(queue: wgpu::Queue) {
    web::QUEUE.with(|cell| *cell.borrow_mut() = Some(queue));
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::cell::RefCell;

    thread_local! {
        pub(super) static QUEUE: RefCell<Option<wgpu::Queue>> = const { RefCell::new(None) };
    }

    pub(super) fn queue() -> Option<wgpu::Queue> {
        QUEUE.with(|cell| cell.borrow().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
        pollster::block_on(adapter.request_device(&Default::default())).ok()
    }

    #[test]
    fn a_buffer_reads_back_what_it_was_created_with() {
        let Some((device, queue)) = device() else {
            eprintln!("no adapter: skipped");
            return;
        };
        let contents: Vec<u8> = (0u8..32).collect();
        let source = init(
            &device,
            "init test",
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC,
            &contents,
        );
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("init test readback"),
            size: contents.len() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&source, 0, &readback, 0, contents.len() as u64);
        queue.submit([encoder.finish()]);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.expect("maps"));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("polls");
        let seen = readback
            .slice(..)
            .get_mapped_range()
            .expect("range")
            .to_vec();
        assert_eq!(seen, contents);
    }
}
