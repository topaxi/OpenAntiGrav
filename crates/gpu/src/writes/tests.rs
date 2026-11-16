use super::*;

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = crate::test_adapter::request(&instance)?;
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

fn buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("encoded"),
        size,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// A copy of `source` taken inside a submission, as a command buffer.
fn snapshot(
    device: &wgpu::Device,
    source: &wgpu::Buffer,
    into: &wgpu::Buffer,
) -> wgpu::CommandBuffer {
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(source, 0, into, 0, source.size());
    encoder.finish()
}

/// Two logs of one submission land at their places in it: what runs between them reads the
/// first log's write, what runs after reads the second's, and a queue write made after both
/// still lands ahead of them all. That is the order one submission per log gave, and what a
/// queue write alone cannot give, since all of a submission's queue writes land before it.
#[test]
fn encoded_logs_land_at_their_places_in_one_submission() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let shared = buffer(&device, 4);
    let (seen_first, seen_second) = (buffer(&device, 4), buffer(&device, 4));
    let seen_queue = buffer(&device, 4);
    let mut upload = UploadBuffer::default();
    let log = RefCell::new(Log::default());
    let writes: &dyn Writes = &log;

    writes.write_buffer(&shared, 0, &[1, 1, 1, 1]);
    let first = log.borrow_mut().encode(&device, &queue, &mut upload);
    writes.write_buffer(&shared, 0, &[2, 2, 2, 2]);
    let second = log.borrow_mut().encode(&device, &queue, &mut upload);
    queue.write_buffer(&shared, 0, &[3, 3, 3, 3]);

    queue.submit(
        [
            Some(snapshot(&device, &shared, &seen_queue)),
            first.map(wgpu::CommandEncoder::finish),
            Some(snapshot(&device, &shared, &seen_first)),
            second.map(wgpu::CommandEncoder::finish),
            Some(snapshot(&device, &shared, &seen_second)),
        ]
        .into_iter()
        .flatten(),
    );
    assert_eq!(read(&device, &queue, &seen_queue), [3; 4]);
    assert_eq!(read(&device, &queue, &seen_first), [1; 4]);
    assert_eq!(read(&device, &queue, &seen_second), [2; 4]);
    assert_eq!(read(&device, &queue, &shared), [2; 4]);
}

/// A log with nothing in it encodes to nothing, and so copies nothing: a frame whose later view
/// wrote nothing leaves that buffer as the frame before it left it, not with bytes the upload
/// buffer still holds from then. Frames apart, each write reaches its target once and in full.
#[test]
fn a_frame_without_writes_leaves_the_last_frames_values() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let target = buffer(&device, 8);
    let other = buffer(&device, 8);
    let mut upload = UploadBuffer::default();
    let log = RefCell::new(Log::default());
    let writes: &dyn Writes = &log;
    for (frame, value) in [(0u8, Some(10u8)), (1, None), (2, Some(30)), (3, None)] {
        upload.rewind();
        // The upload buffer's first bytes change every frame, so a stale copy would show.
        writes.write_buffer(&other, 0, &[frame; 8]);
        let buffers: Vec<_> = log
            .borrow_mut()
            .encode(&device, &queue, &mut upload)
            .map(wgpu::CommandEncoder::finish)
            .into_iter()
            .collect();
        queue.submit(buffers);
        if let Some(value) = value {
            writes.write_buffer(&target, 4, &[value; 4]);
        }
        let encoded = log.borrow_mut().encode(&device, &queue, &mut upload);
        assert_eq!(encoded.is_some(), value.is_some(), "frame {frame}");
        queue.submit(encoded.map(wgpu::CommandEncoder::finish));
        let held = if frame < 2 { 10 } else { 30 };
        let mut want = [0; 8];
        want[4..].fill(held);
        assert_eq!(read(&device, &queue, &target), want, "frame {frame}");
    }
}

/// A log too large for the upload buffer gets a larger one mid-frame, and the copies of the
/// log before it still read the old buffer: nothing a frame encoded is lost when it grows.
#[test]
fn the_upload_buffer_grows_inside_a_frame() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    let size = MIN_UPLOAD_SIZE as usize;
    let (small, large) = (buffer(&device, 16), buffer(&device, size as u64 + 16));
    let mut upload = UploadBuffer::default();
    let log = RefCell::new(Log::default());
    let writes: &dyn Writes = &log;
    writes.write_buffer(&small, 0, &[7; 16]);
    let first = log.borrow_mut().encode(&device, &queue, &mut upload);
    let held = upload.buffer.as_ref().map(wgpu::Buffer::size);
    assert_eq!(held, Some(MIN_UPLOAD_SIZE));
    let bytes: Vec<u8> = (0..size + 16).map(|at| (at % 251) as u8).collect();
    writes.write_buffer(&large, 0, &bytes);
    let second = log.borrow_mut().encode(&device, &queue, &mut upload);
    assert!(upload.buffer.as_ref().map(wgpu::Buffer::size) > held);
    queue.submit(
        first
            .into_iter()
            .chain(second)
            .map(wgpu::CommandEncoder::finish),
    );
    assert_eq!(read(&device, &queue, &small), [7; 16]);
    assert_eq!(read(&device, &queue, &large), bytes);
}

/// Reads an `Rgba8Unorm` or `R8Unorm` texture of `layers` back, tightly packed.
fn texels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    texel: u32,
) -> Vec<u8> {
    let size = texture.size();
    let row = (size.width * texel).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("log texture readback"),
        size: u64::from(row * size.height * size.depth_or_array_layers),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &out,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(size.height),
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    out.slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("maps"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("polls");
    let mapped = out.slice(..).get_mapped_range().expect("range");
    let line = (size.width * texel) as usize;
    mapped
        .chunks(row as usize)
        .flat_map(|chunk| chunk[..line].iter().copied())
        .collect()
}

/// A texture write made with a queue write's tight rows, an offset and a stride of its own
/// reaches the texture the same through [`Log::encode`] and through [`Log::replay`], for one
/// byte and four byte texels, over two array layers: a copy out of a buffer asks for row
/// spacing a queue write does not, and the log has to make that up.
#[test]
fn an_encoded_texture_write_lands_as_a_queue_write_does() {
    let Some((device, queue)) = device() else {
        eprintln!("no adapter: skipped");
        return;
    };
    for (format, texel) in [
        (wgpu::TextureFormat::R8Unorm, 1u32),
        (wgpu::TextureFormat::Rgba8Unorm, 4),
    ] {
        let texture = || {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("logged texture"),
                size: wgpu::Extent3d {
                    width: 5,
                    height: 3,
                    depth_or_array_layers: 2,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let (replayed, encoded) = (texture(), texture());
        let stride = 5 * texel + 3;
        let rows_per_image = 4u32;
        let offset = 7u64;
        let data: Vec<u8> = (0..offset as u32 + 2 * rows_per_image * stride)
            .map(|at| (at * 7 % 253) as u8 + 1)
            .collect();
        let layout = wgpu::TexelCopyBufferLayout {
            offset,
            bytes_per_row: Some(stride),
            rows_per_image: Some(rows_per_image),
        };
        let mut upload = UploadBuffer::default();
        for target in [&replayed, &encoded] {
            let log = RefCell::new(Log::default());
            let writes: &dyn Writes = &log;
            // A word ahead of it, so the texture's bytes do not start at the log's start.
            let word = buffer(&device, 4);
            writes.write_buffer(&word, 0, &[1, 2, 3, 4]);
            writes.write_texture(target.as_image_copy(), &data, layout, target.size());
            if std::ptr::eq(target, &replayed) {
                log.borrow_mut().replay(&queue);
                queue.submit([]);
            } else {
                let encoded = log.borrow_mut().encode(&device, &queue, &mut upload);
                queue.submit(encoded.map(wgpu::CommandEncoder::finish));
            }
        }
        let want = texels(&device, &queue, &replayed, texel);
        let mut direct = Vec::new();
        for layer in 0..2usize {
            for line in 0..3usize {
                let from =
                    offset as usize + (layer * rows_per_image as usize + line) * stride as usize;
                direct.extend_from_slice(&data[from..from + (5 * texel) as usize]);
            }
        }
        assert_eq!(want, direct, "{format:?}: the replay is the queue write");
        assert_eq!(
            texels(&device, &queue, &encoded, texel),
            direct,
            "{format:?}"
        );
    }
}
