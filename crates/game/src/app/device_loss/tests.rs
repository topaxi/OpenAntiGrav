use super::*;

fn validation(text: &str) -> wgpu::Error {
    wgpu::Error::Validation {
        source: Box::new(std::fmt::Error),
        description: text.into(),
    }
}

#[test]
fn an_error_that_names_a_lost_device_is_the_loss() {
    let loss = DeviceLoss::default();
    assert_eq!(
        loss.judge(&validation("Parent device is lost")),
        Verdict::Lost
    );
    assert!(!loss.is_lost(), "judging records nothing");
}

#[test]
fn an_error_on_a_live_device_stays_fatal_until_a_loss_is_recorded() {
    let loss = DeviceLoss::default();
    let other = validation("Buffer is invalid");
    assert_eq!(loss.judge(&other), Verdict::Fatal);
    loss.record(wgpu::DeviceLostReason::Unknown, "reset".into());
    assert_eq!(loss.judge(&other), Verdict::Swallow);
}

#[test]
fn only_a_validation_error_can_name_a_lost_device() {
    let error = wgpu::Error::Internal {
        source: Box::new(std::fmt::Error),
        description: "device is lost".into(),
    };
    assert_eq!(DeviceLoss::default().judge(&error), Verdict::Fatal);
}

#[test]
fn the_loss_is_taken_once_and_stays_a_loss() {
    let loss = DeviceLoss::default();
    assert_eq!(loss.take(), None);
    loss.record(wgpu::DeviceLostReason::Unknown, "first".into());
    loss.record(wgpu::DeviceLostReason::Destroyed, "second".into());
    assert_eq!(
        loss.take(),
        Some(Lost {
            reason: wgpu::DeviceLostReason::Unknown,
            message: "first".into()
        }),
        "the first reason stands"
    );
    assert_eq!(loss.take(), None, "the frame loop reacts once");
    assert!(
        loss.is_lost(),
        "a taken loss still swallows what the dead device reports"
    );
}

/// A destroyed device reaches the flag, and what it reports afterwards does not panic. Without
/// the handler installed, wgpu's own rule panics on the first call into the dead device.
#[test]
fn a_destroyed_device_is_a_loss_and_its_later_errors_are_swallowed() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        return;
    };
    let loss = DeviceLoss::install(&device);
    assert_eq!(loss.take(), None);

    device.destroy();
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let lost = loss.take().expect("the callback ran");
    assert_eq!(lost.reason, wgpu::DeviceLostReason::Destroyed);

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("after the loss"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, &[0; 16]);
    queue.submit([]);
    assert!(loss.is_lost());
}

#[test]
fn the_note_for_the_next_run_is_left_once_and_read_once() {
    let dir = std::env::temp_dir().join(format!("oag-device-loss-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!take_marker_in(&dir), "no loss yet");
    leave_marker_in(
        &dir,
        &Lost {
            reason: wgpu::DeviceLostReason::Unknown,
            message: "GPU hang\nsecond line".into(),
        },
    );
    let text = std::fs::read_to_string(dir.join(MARKER)).expect("the note");
    assert_eq!(text, "Unknown\nGPU hang second line\n");
    assert!(take_marker_in(&dir), "the next run reads it");
    assert!(!take_marker_in(&dir), "and only once");
    let _ = std::fs::remove_dir_all(&dir);
}
