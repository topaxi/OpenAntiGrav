//! What [`super::Timing`] reports, and - the part worth having - whether an
//! adapter that reports the feature can actually resolve a timestamp pair.
//!
//! The two are different claims. A feature bit says the driver advertises the
//! query; it does not say a query set resolves, that the period is non-zero,
//! or that the pair comes back in order. A controller built on the bit alone
//! would divide by a zero period or act on a negative interval, and the bit
//! would still read green. So the last test runs the whole path.

use super::*;

#[test]
fn nothing_probed_asks_for_nothing() {
    assert_eq!(Timing::NONE.features(), wgpu::Features::empty());
    assert!(!Timing::NONE.can_time_a_pass());
    assert_eq!(Timing::NONE.describe(), "no GPU timestamps");
}

/// The native bits are never asked for on their own.
///
/// wgpu requires `TIMESTAMP_QUERY` alongside either of them, so a probe that
/// somehow saw one without it must not turn that into a device request that
/// fails - which is the failure mode this whole module exists to avoid.
#[test]
fn a_native_bit_without_the_portable_one_asks_for_nothing() {
    let odd = Timing {
        passes: false,
        encoders: true,
        inside_passes: true,
    };
    assert_eq!(odd.features(), wgpu::Features::empty());
}

#[test]
fn what_is_asked_for_is_what_was_probed() {
    let passes_only = Timing {
        passes: true,
        ..Timing::NONE
    };
    assert_eq!(passes_only.features(), wgpu::Features::TIMESTAMP_QUERY);
    assert_eq!(passes_only.describe(), "timestamps around passes");

    let all = Timing {
        passes: true,
        encoders: true,
        inside_passes: true,
    };
    assert_eq!(
        all.features(),
        wgpu::Features::TIMESTAMP_QUERY
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES
    );
}

/// Reports every adapter this machine offers and what each can be timed with.
///
/// Asserts nothing about the answer - it is a property of the machine, not of
/// this code - but it is the command that produces the record
/// `docs/rendering/dynamic-resolution.md` cites, and it fails loudly if the
/// enumeration itself breaks:
///
/// ```sh
/// cargo nextest run -p oag-render -E 'test(every_adapter)' --no-capture
/// ```
#[test]
fn every_adapter_reports_what_it_can_be_timed_with() {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    if adapters.is_empty() {
        eprintln!("no GPU adapter: skipping");
        return;
    }
    for adapter in &adapters {
        let info = adapter.get_info();
        let timing = Timing::of(adapter);
        eprintln!(
            "{:?} {} ({:?}): {}",
            info.backend,
            info.name,
            info.device_type,
            timing.describe()
        );
    }
}

/// The control the feature bit alone cannot give: a real device, a real pass,
/// a resolved pair and a plausible duration - on **every** adapter that claims
/// the feature, not only the one `request_adapter` happens to pick.
///
/// Both halves matter. An adapter that advertises the query and then refuses
/// the device is a boot failure a controller must never cause, and the tick
/// *period* differs by an order of magnitude between adapters on the same
/// machine, so a record taken from one says nothing about the other.
///
/// **Skips an adapter with no timestamps**, and skips outright with none at
/// all - so a green run is not on its own evidence that this ran. Run it where
/// the hardware is:
///
/// ```sh
/// cargo nextest run -p oag-render -E 'test(a_timestamp_pair)' --no-capture
/// ```
#[test]
fn a_timestamp_pair_resolves_to_a_forward_going_duration() {
    let instance = wgpu::Instance::default();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    if adapters.is_empty() {
        eprintln!("no GPU adapter: skipping");
        return;
    }
    for adapter in &adapters {
        let info = adapter.get_info();
        let timing = Timing::of(adapter);
        if !timing.can_time_a_pass() {
            eprintln!("{}: {}, skipping", info.name, timing.describe());
            continue;
        }
        time_a_cleared_pass(adapter, &info.name, timing);
    }
}

/// One adapter's whole path: request the device with what was probed, bracket
/// a pass, resolve the pair, and check the units.
fn time_a_cleared_pass(adapter: &wgpu::Adapter, name: &str, timing: Timing) {
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("timing probe"),
        required_features: timing.features(),
        ..Default::default()
    })) else {
        panic!(
            "{name} reported {} and then refused the device",
            timing.describe()
        );
    };

    // Nanoseconds per tick. Zero would make every measured cost zero, which
    // reads as "the frame was free" rather than as "the clock is broken".
    let period = queue.get_timestamp_period();
    assert!(
        period.is_finite() && period > 0.0,
        "{name}: timestamp period is {period}"
    );

    let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("timing probe"),
        ty: wgpu::QueryType::Timestamp,
        count: 2,
    });
    let resolved = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("resolved"),
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("timing probe"),
        size: wgpu::Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());

    let mut encoder = device.create_command_encoder(&Default::default());
    // A pass that draws nothing: the clear is the work, and bracketing it is
    // exactly the shape a controller would use around the scene passes.
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("timing probe"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
            query_set: &queries,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: Some(1),
        }),
        occlusion_query_set: None,
        multiview_mask: None,
    });
    encoder.resolve_query_set(&queries, 0..2, &resolved, 0);
    encoder.copy_buffer_to_buffer(&resolved, 0, &readback, 0, 16);
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the GPU");
    let mapped = slice.get_mapped_range().expect("the readback");
    let at = |offset: usize| {
        u64::from_le_bytes(<[u8; 8]>::try_from(&mapped[offset..offset + 8]).expect("eight bytes"))
    };
    let (begin, end) = (at(0), at(8));
    drop(mapped);
    readback.unmap();

    // Forward-going, which is the property a controller subtracts on. A
    // wrapped or unordered pair would come back as an enormous unsigned
    // difference and read as a catastrophically slow frame.
    assert!(
        end >= begin,
        "{name}: timestamps went backwards, {begin} -> {end}"
    );
    let nanos = (end - begin) as f64 * f64::from(period);
    eprintln!("{name}: {period} ns per tick, a cleared 256x256 pass took {nanos:.0} ns");
    // A clear of a 256x256 target is fast, but it is not free and it is not a
    // whole second. Both bounds are loose on purpose: the claim is that the
    // clock moves and is in the right units, not that this machine is fast.
    assert!(
        nanos > 0.0 && nanos < 1_000_000_000.0,
        "{name}: a cleared 256x256 pass measured {nanos} ns"
    );
}
