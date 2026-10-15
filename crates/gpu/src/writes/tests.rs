use super::*;

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

fn read(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &wgpu::Buffer) -> Vec<u8> {
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("log readback"),
        size: buffer.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &out, 0, buffer.size());
    queue.submit([encoder.finish()]);
    out.slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("maps"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("polls");
    out.slice(..).get_mapped_range().expect("range").to_vec()
}

/// A logged write lands only when replayed, and two writes to one place land in the order
/// they were made: the second wins, as it would have on the queue.
#[test]
fn a_logged_write_waits_for_its_replay_and_keeps_its_order() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("logged"),
        size: 8,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let log = RefCell::new(Log::default());
    let writes: &dyn Writes = &log;
    writes.write_buffer(&buffer, 0, &[1, 2, 3, 4]);
    writes.write_buffer(&buffer, 4, &[5, 6, 7, 8]);
    writes.write_buffer(&buffer, 0, &[9, 9, 9, 9]);
    assert_eq!(log.borrow().len(), 3);
    assert_eq!(
        read(&device, &queue, &buffer),
        [0; 8],
        "nothing lands before the replay"
    );
    log.borrow_mut().replay(&queue);
    assert!(log.borrow().is_empty());
    assert_eq!(read(&device, &queue, &buffer), [9, 9, 9, 9, 5, 6, 7, 8]);
}
