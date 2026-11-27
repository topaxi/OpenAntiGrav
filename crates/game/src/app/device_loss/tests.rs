use super::*;

fn validation(text: &str) -> wgpu::Error {
    wgpu::Error::Validation {
        source: Box::new(std::fmt::Error),
        description: text.into(),
    }
}

/// wgpu 30.0.2 never hands the uncaptured-error handler a lost device's own error, so an error
/// that says "device is lost" is a validation bug like any other and must stay fatal.
#[test]
fn a_validation_error_that_says_the_device_is_lost_is_still_fatal() {
    let loss = DeviceLoss::default();
    assert_eq!(
        loss.judge(&validation("Parent device is lost")),
        Verdict::Fatal
    );
    assert!(!loss.is_lost(), "an error is never the loss");
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
    let require = std::env::var_os("OAG_REQUIRE_GPU").is_some();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        assert!(!require, "OAG_REQUIRE_GPU is set and there is no adapter");
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        assert!(
            !require,
            "OAG_REQUIRE_GPU is set and the device was refused"
        );
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
fn the_note_stays_until_it_is_cleared() {
    let dir = std::env::temp_dir().join(format!("oag-device-loss-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!marker_present_in(&dir), "no loss yet");
    leave_marker_in(
        &dir,
        &Lost {
            reason: wgpu::DeviceLostReason::Unknown,
            message: "GPU hang\nsecond line".into(),
        },
    );
    let text = std::fs::read_to_string(dir.join(MARKER)).expect("the note");
    assert_eq!(text, "Unknown\nGPU hang second line\n");
    assert!(marker_present_in(&dir));
    assert!(
        marker_present_in(&dir),
        "reading it does not use it up: a run that never reaches the menu leaves it"
    );
    clear_marker_in(&dir);
    assert!(!marker_present_in(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

fn args(list: &[&str]) -> Vec<std::ffi::OsString> {
    list.iter().map(Into::into).collect()
}

#[test]
fn a_game_started_the_way_a_player_starts_it_may_restart() {
    for list in [
        &[][..],
        &["pulse.chd"],
        &["--source", "pulse.chd", "--no-audio"],
        &["pulse.chd", "--no-soundtrack", "--no-effects"],
        &["--source=pulse.chd", "--dlc", "pack", "--no-intro"],
        &["pulse.chd", "--launcher", "--pick-language"],
        &["--launcher", "--launcher-list"],
        &["pulse.chd", "--script", "45:lose-device"],
    ] {
        assert!(restartable_args(&args(list)), "{list:?}");
    }
}

#[test]
fn a_game_started_with_anything_that_would_run_twice_does_not_restart() {
    for list in [
        &["--race"][..],
        &["pulse.chd", "--connect", "host:1234"],
        &["pulse.chd", "--online"],
        &["pulse.chd", "--dump-audio", "out.wav"],
        &["pulse.chd", "--tap-audio", "out.wav"],
        &["pulse.chd", "--screenshot", "out.png"],
        &["pulse.chd", "--no-audio", "--trace-out", "t.csv"],
        &["--source"],
    ] {
        assert!(!restartable_args(&args(list)), "{list:?}");
    }
}
