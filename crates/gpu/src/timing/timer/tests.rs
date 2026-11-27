//! Two halves, tested two ways.
//!
//! [`Ring`] is the "which frame does this reading belong to" bookkeeping and
//! has no device in it, so it is tested as a plain state machine - which is
//! where the bugs that matter live: a slot reused while its map is in flight,
//! or a reading credited to the wrong frame, are both wrong *numbers* rather
//! than crashes, and a GPU test would show neither.
//!
//! [`PassTimer`] itself needs a real device, a real pass and a real readback,
//! and gets the same treatment `super::super::tests` gives the probe: every
//! adapter that claims the feature, skipping those that do not.

use super::*;

#[test]
fn a_claim_names_the_frame_it_was_made_for() {
    let mut ring = Ring::new();
    let slot = ring.claim(7).expect("a free slot");
    ring.copied(slot);
    ring.mapping(slot);
    assert_eq!(ring.ready(|_| true), Some((slot, 7)));
}

/// The property the whole ring exists for: four frames in flight at once, each
/// coming back as itself.
#[test]
fn four_frames_in_flight_come_back_as_themselves() {
    let mut ring = Ring::new();
    for frame in 100..104 {
        let slot = ring.claim(frame).expect("a free slot");
        ring.copied(slot);
        ring.mapping(slot);
    }
    for frame in 100..104 {
        let (slot, in_flight) = ring.ready(|_| true).expect("a reading");
        assert_eq!(in_flight, frame, "a backlog drains oldest first");
        ring.free(slot);
    }
    assert_eq!(ring.ready(|_| true), None);
}

/// A slot is never handed out twice, which is what would produce a *wrong*
/// reading rather than a missing one.
#[test]
fn a_full_ring_refuses_rather_than_reusing() {
    let mut ring = Ring::new();
    for frame in 0..SLOTS as u64 {
        let slot = ring.claim(frame).expect("a free slot");
        ring.copied(slot);
        ring.mapping(slot);
    }
    assert_eq!(ring.claim(99), None);
    let (slot, _) = ring.ready(|_| true).expect("a reading");
    ring.free(slot);
    assert_eq!(ring.claim(99), Some(slot), "the freed one, and only it");
}

/// A map that never lands costs its own slot and nothing else.
///
/// The failure this guards is not a missing sample, it is a dead ring: taking
/// the oldest slot outright rather than the oldest *ready* one would have a
/// single stuck flag hold up every slot behind it for the rest of the run, and
/// four of them stop the timer measuring anything ever again.
#[test]
fn a_slot_whose_map_never_lands_does_not_hold_up_the_others() {
    let mut ring = Ring::new();
    let stuck = ring.claim(1).expect("a free slot");
    ring.copied(stuck);
    ring.mapping(stuck);
    let later = ring.claim(2).expect("a free slot");
    ring.copied(later);
    ring.mapping(later);

    assert_eq!(
        ring.ready(|slot| slot != stuck),
        Some((later, 2)),
        "frame 2 is readable while frame 1's map is outstanding"
    );
    ring.free(later);
    assert_eq!(ring.ready(|slot| slot != stuck), None);
    // Every slot but the stuck one is still in circulation: three more claims
    // land, and none of them is the one whose map never came back.
    for frame in 3..6 {
        let slot = ring.claim(frame).expect("a free slot");
        assert_ne!(slot, stuck, "the stuck slot was handed out again");
        ring.copied(slot);
        ring.mapping(slot);
        ring.free(slot);
    }
}

/// Round robin rather than lowest-free-first: a readback wants the longest
/// possible time to land in, and always reaching for slot 0 would give the one
/// most recently freed the least.
#[test]
fn slots_are_used_round_robin() {
    let mut ring = Ring::new();
    let claimed: Vec<_> = (0..SLOTS as u64)
        .map(|frame| ring.claim(frame).expect("a free slot"))
        .collect();
    assert_eq!(claimed, (0..SLOTS).collect::<Vec<_>>());
}

/// A claim that is never resolved holds its slot, and four of them stop the
/// timer measuring anything ever again - which is why `begin` and `resolve`
/// are documented as a pair.
#[test]
fn an_unresolved_claim_holds_its_slot() {
    let mut ring = Ring::new();
    for frame in 0..SLOTS as u64 {
        ring.claim(frame).expect("a free slot");
    }
    assert_eq!(ring.claim(99), None);
    assert_eq!(ring.ready(|_| true), None, "nothing to read, either");
}

/// The transitions are one-way. `copied` on a slot that was never claimed, or
/// `mapping` on one that was never copied, leaves it alone rather than
/// inventing a frame index for it.
#[test]
fn a_transition_out_of_order_changes_nothing() {
    let mut ring = Ring::new();
    ring.copied(2);
    ring.mapping(2);
    assert_eq!(ring.ready(|_| true), None);
    assert_eq!(ring.claim(1), Some(0), "and slot 2 is still free");
}

/// The whole path on real hardware: a device requested with what
/// [`Timing`](super::super::Timing) probed, a pass bracketed through
/// [`PassTimer::writes`], and a reading that comes back naming the frame it
/// was asked for.
///
/// **Skips an adapter with no timestamps**, and skips outright with none at
/// all, so a green run is not on its own evidence that this ran:
///
/// ```sh
/// cargo nextest run -p oag-render -E 'test(a_bracketed_pass)' --no-capture
/// ```
#[test]
fn a_bracketed_pass_reads_back_naming_its_own_frame() {
    let instance = wgpu::Instance::default();
    let adapters = crate::test_adapter::ordered(&instance);
    if adapters.is_empty() {
        eprintln!("no GPU adapter: skipping");
        return;
    }
    for adapter in &adapters {
        let info = adapter.get_info();
        let timing = super::super::Timing::of(adapter);
        if !timing.can_time_a_pass() {
            eprintln!("{}: {}, skipping", info.name, timing.describe());
            continue;
        }
        time_a_pass(adapter, &info.name);
    }
}

/// One adapter's whole round trip, twice, so the second frame proves a slot is
/// reusable rather than only that the first one worked.
fn time_a_pass(adapter: &wgpu::Adapter, name: &str) {
    let features = super::super::Timing::of(adapter).features();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("pass timer test"),
        required_features: features,
        ..Default::default()
    }))
    .expect("a device with what was probed");
    let mut timer = PassTimer::new(&device, &queue).expect("a timer on a device that has the bit");
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pass timer test"),
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
        texture_binding_view_dimension: None,
    });
    let view = target.create_view(&Default::default());

    for frame in [11u64, 12] {
        timer.begin(frame);
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pass timer test"),
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
            timestamp_writes: timer.writes(),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        timer.resolve(&mut encoder);
        queue.submit(Some(encoder.finish()));

        // The frame loop calls this once a frame and takes what has arrived;
        // a test has nothing to pace, so it asks until the reading is there.
        // The bound is what makes a never-arriving reading a failure rather
        // than a hang.
        let mut reading = None;
        for _ in 0..1_000 {
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("the GPU");
            if let Some(got) = timer.read(&device) {
                reading = Some(got);
                break;
            }
        }
        let reading = reading.unwrap_or_else(|| panic!("{name}: frame {frame} never came back"));
        assert_eq!(
            reading.frame, frame,
            "{name}: the reading named the wrong frame"
        );
        // Loose on purpose, exactly as the probe's bounds are: the claim is
        // that the clock moves and is in seconds, not that this machine is
        // fast.
        assert!(
            reading.seconds > 0.0 && reading.seconds < 60.0,
            "{name}: a cleared 256x256 pass measured {} s",
            reading.seconds
        );
        eprintln!(
            "{name}: frame {frame} took {:.6} ms",
            reading.seconds * 1000.0
        );
    }

    // **Two frames submitted with nothing read between them come back in one
    // `drain`, oldest first.** `read` would hand back one per call and leave
    // the other waiting - which is the behaviour that lets a ring fall a frame
    // behind its neighbours for good; see `PassTimer::drain`.
    for frame in [13u64, 14] {
        timer.begin(frame);
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pass timer test"),
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
            timestamp_writes: timer.writes(),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        timer.resolve(&mut encoder);
        queue.submit(Some(encoder.finish()));
    }
    let mut drained = [None; PassTimer::SLOTS];
    let mut frames = Vec::new();
    for _ in 0..1_000 {
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the GPU");
        timer.drain(&device, &mut drained);
        frames.extend(drained.iter().flatten().map(|reading| reading.frame));
        if frames.len() >= 2 {
            break;
        }
    }
    assert_eq!(
        frames,
        [13, 14],
        "{name}: a drain hands back every ready reading, oldest first"
    );
    timer.drain(&device, &mut drained);
    assert!(
        drained.iter().all(Option::is_none),
        "{name}: a drain on an empty ring leaves the array empty"
    );
}

/// A device without the feature gets no timer rather than a validation error
/// the first time a pass is encoded.
///
/// The one path the development machine cannot otherwise exercise - both its
/// adapters have the feature - reached by simply not asking for it.
#[test]
fn a_device_without_the_feature_gets_no_timer() {
    let instance = wgpu::Instance::default();
    let adapters = crate::test_adapter::ordered(&instance);
    let Some(adapter) = adapters.first() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("no timestamps"),
        required_features: wgpu::Features::empty(),
        ..Default::default()
    }))
    .expect("a device with nothing asked of it");
    assert!(PassTimer::new(&device, &queue).is_none());
}

/// Pairs summed by the label they were stamped with, in first-seen order.
#[test]
fn passes_sharing_a_label_are_summed_in_first_seen_order() {
    // Period 1e9 makes a tick one second, so the sums read off directly.
    let sums = sum_by_label(
        &["shadow", "glass", "shadow"],
        &[],
        &[(0, 2), (10, 13), (20, 21)],
        1e9,
    );
    assert_eq!(sums, [("shadow", 3.0), ("glass", 3.0)]);
}

/// A label that lost a pass to a full slot, or has a wrapped pair, is a part of its cost, and a
/// part is worse than no row.
#[test]
fn a_label_with_a_missing_or_wrapped_pass_reports_nothing() {
    let dropped = sum_by_label(&["a", "b"], &["a"], &[(0, 1), (0, 1)], 1e9);
    assert_eq!(dropped, [("b", 1.0)], "a dropped label is left out whole");
    let wrapped = sum_by_label(&["a", "a", "b"], &[], &[(0, 1), (5, 4), (0, 1)], 1e9);
    assert_eq!(wrapped, [("b", 1.0)], "one wrapped pair takes its label");
}

/// Nothing is armed until a ring claims a frame, so a pass site outside a measured frame gets
/// no stamp and pays only the lookup.
#[test]
fn a_stamp_outside_a_claimed_frame_is_none() {
    assert!(stamp("shadow").is_none());
}

fn stamped_pass(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    label: &'static str,
) {
    let stamp = stamp(label);
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: stamp.as_ref().map(Stamp::render_writes),
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let _ = device;
}

/// The whole labelled path on every adapter that can be timed: two frames in flight with
/// different passes each, read back after both are submitted, each reading naming its own
/// frame and its own labels. A ring-wide label table would label frame 1's reading with frame
/// 2's passes.
#[test]
fn labelled_passes_come_back_under_their_own_frames_labels() {
    let instance = wgpu::Instance::default();
    for adapter in &crate::test_adapter::ordered(&instance) {
        let info = adapter.get_info();
        let timing = super::super::Timing::of(adapter);
        if !timing.can_time_a_pass() {
            eprintln!("{}: {}, skipping", info.name, timing.describe());
            continue;
        }
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("labelled pass timer test"),
            required_features: timing.features(),
            ..Default::default()
        }))
        .expect("a device with what was probed");
        let mut timer = PassTimer::with_passes(&device, &queue, 3).expect("a timer");
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("labelled pass timer test"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
            texture_binding_view_dimension: None,
        });
        let view = target.create_view(&Default::default());
        // Frame 21: shadow twice, glass. Frame 22: hud alone. Frame 23: four passes of two
        // labels into three pairs, so `shadow` loses a pass. Frame 24: nothing stamped.
        let frames: [(u64, &[&'static str]); 4] = [
            (21, &["shadow", "glass", "shadow"]),
            (22, &["hud"]),
            (23, &["glass", "shadow", "shadow", "shadow"]),
            (24, &[]),
        ];
        for (frame, passes) in frames {
            timer.begin(frame);
            let mut encoder = device.create_command_encoder(&Default::default());
            for label in passes {
                stamped_pass(&device, &mut encoder, &view, label);
            }
            timer.resolve(&mut encoder);
            queue.submit(Some(encoder.finish()));
        }
        let mut seen: Vec<(u64, &'static str)> = Vec::new();
        let mut out = Vec::new();
        for _ in 0..1_000 {
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("the GPU");
            timer.drain_labelled(&device, &mut out);
            seen.extend(out.iter().map(|reading| (reading.frame, reading.label)));
            for reading in &out {
                assert!(
                    reading.seconds > 0.0 && reading.seconds < 60.0,
                    "{}: {} took {} s",
                    info.name,
                    reading.label,
                    reading.seconds
                );
            }
            if seen.len() >= 5 {
                break;
            }
        }
        seen.sort_unstable();
        assert_eq!(
            seen,
            [(21, "glass"), (21, "shadow"), (22, "hud"), (23, "glass"),],
            "{}: frame 23's `shadow` lost a pass to the full slot and must report nothing, and \
             frame 24 stamped nothing and must report nothing",
            info.name
        );
        // The empty frame gave its slot back, so a fifth claim is not refused.
        for frame in 30..34 {
            timer.begin(frame);
            timer.abandon();
        }
        timer.begin(40);
        assert!(
            timer.writing.is_some(),
            "{}: a frame that stamped nothing or was abandoned held its slot",
            info.name
        );
        timer.abandon();
    }
}
