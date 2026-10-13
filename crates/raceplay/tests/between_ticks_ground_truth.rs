//! Drawing between ticks on a real circuit, the way the window does it: a frame
//! clock fed a fixed 240 Hz or 144 Hz period, [`between_ticks::frame_alpha`] for
//! the alpha, and the player's craft and the chase camera read through the
//! accessors the scene draws with. Disc-backed, so the motion is the real
//! physics on Pulse's own circuit rather than a fixture's straight line.

use oag_core::{TickClock, TickRate};
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_raceplay::{self as race, between_ticks};

/// What one run of frames measured.
struct Run {
    frames: usize,
    /// Frames whose drawn craft did not move from the frame before.
    still_frames: usize,
    /// The shortest and longest per-frame step, along the craft's travel.
    shortest: f32,
    longest: f32,
    /// Frames whose drawn eye did not move.
    still_eye_frames: usize,
}

/// One second of frames at `hz`, with the alpha `alpha` picks per frame:
/// [`between_ticks::frame_alpha`] for the window as it now is, `1.0` for the
/// window as it was.
fn run(hz: u64, alpha: impl Fn(&TickClock) -> f32) -> Option<Run> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    let mut held = race::HeldButtons::new(Button::Cross.bit());
    // Past the countdown and two seconds up to speed.
    for _ in 0..oag_race::COUNTDOWN_TICKS + 120 {
        race.tick(&PlayerInputs::single(held.snapshot()));
    }

    let mut clock = TickClock::new(TickRate::from_hz(60));
    let period = 1_000_000_000 / hz;
    let frames = usize::try_from(hz).expect("a frame rate");
    let mut previous: Option<(oag_core::math::Vec3, oag_core::math::Vec3)> = None;
    let mut measured = Run {
        frames,
        still_frames: 0,
        shortest: f32::INFINITY,
        longest: 0.0,
        still_eye_frames: 0,
    };
    for frame in 0..frames {
        // The window's frame: the ticks the clock owes, then the draw.
        for _ in 0..clock.advance(period) {
            race.tick(&PlayerInputs::single(held.snapshot()));
        }
        race.set_draw_alpha(alpha(&clock));
        let drawn = race.ship_model_matrix_of(0).w_axis.truncate();
        let eye = race.camera_position();
        race.set_draw_alpha(1.0);
        // At alpha 1 the accessor is the tick itself, bit for bit.
        let body = race.sim.world.ships[0].physics.body;
        assert_eq!(
            race.ship_model_matrix_of(0).w_axis.truncate(),
            body.position,
            "frame {frame}: alpha 1 drew something other than the tick"
        );
        if let Some((last, last_eye)) = previous {
            let along = (drawn - last).dot(body.linear_velocity.normalize());
            assert!(
                along >= 0.0,
                "frame {frame}: the drawn craft went backwards by {along}"
            );
            if along <= 1e-4 {
                measured.still_frames += 1;
            } else {
                measured.shortest = measured.shortest.min(along);
            }
            measured.longest = measured.longest.max(along);
            if eye.distance(last_eye) <= 1e-4 {
                measured.still_eye_frames += 1;
            }
        }
        previous = Some((drawn, eye));
    }
    Some(measured)
}

fn report(label: &str, run: &Run) {
    println!(
        "{label}: {} frames, craft still on {}, eye still on {}, per-frame step {:.3}..{:.3} units",
        run.frames, run.still_frames, run.still_eye_frames, run.shortest, run.longest
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn above_60_hz_the_craft_and_camera_move_on_every_frame() {
    for hz in [240u64, 144] {
        let Some(before) = run(hz, |_| 1.0) else {
            return;
        };
        let after = run(hz, |clock| between_ticks::frame_alpha(clock, true))
            .expect("the image was there a moment ago");
        report(
            &format!("{hz} Hz, every frame at the tick (before)"),
            &before,
        );
        report(&format!("{hz} Hz, between ticks (after)"), &after);
        assert!(
            before.still_frames > 0,
            "{hz} Hz: the old window never repeated a frame, so this measures nothing"
        );
        assert_eq!(
            after.still_frames, 0,
            "{hz} Hz: the drawn craft held still on some frames"
        );
        assert_eq!(
            after.still_eye_frames, 0,
            "{hz} Hz: the drawn camera held still on some frames"
        );
    }
}
