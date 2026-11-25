//! The reflection rung against fed sequences: what a desktop and a phone cost, and the extent
//! loop beside it.

use super::*;
use crate::drs::{Controller, Cost, Limits, Target};

const MS: f32 = 0.001;

/// A machine: the frame without the glass, and the glass at each step, seconds. The frame is
/// unpaced (wall clock and timed GPU both the sum) unless `paced` holds it to the period.
#[derive(Clone, Copy)]
struct Machine {
    base: f32,
    glass: [f32; 3],
    period: f32,
    paced: bool,
}

impl Machine {
    fn reading(self, step: ReflectionResolution) -> Reading {
        let level = ReflectionResolution::STEPS
            .iter()
            .position(|s| *s == step)
            .unwrap();
        let gpu = self.base + self.glass[level];
        Reading {
            glass: Some(self.glass[level]),
            gpu: Some(gpu),
            frame: if self.paced {
                gpu.max(self.period)
            } else {
                gpu
            },
            period: Some(self.period),
        }
    }
}

/// The RX 7800 XT's Vineta K target, 0.12 ms (`docs/rendering/water.md`), in a desktop frame
/// that misses a 144 Hz budget: a share near 2 %, under the floor.
const DESKTOP: Machine = Machine {
    base: 8.0 * MS,
    glass: [0.12 * MS, 0.07 * MS, 0.05 * MS],
    period: 1.0 / 144.0,
    paced: false,
};

/// The S21 at render scale 25 on Vineta K: 41.9 ms at `original`, 24.9 at `quarter`
/// (`docs/tools/android.md`), against its 120 Hz panel.
const MALI: Machine = Machine {
    base: 23.6 * MS,
    glass: [18.3 * MS, 5.0 * MS, 1.3 * MS],
    period: 1.0 / 120.0,
    paced: false,
};

/// Runs `machine` for `seconds` and returns the steps it changed through.
fn run(rung: &mut Rung, machine: Machine, seconds: f32) -> Vec<Change> {
    let mut changes = Vec::new();
    let mut t = 0.0;
    while t < seconds {
        let reading = machine.reading(rung.step());
        t += reading.frame;
        if let Some(change) = rung.record(reading, true) {
            changes.push(change);
        }
    }
    changes
}

#[test]
fn a_desktop_cost_never_leaves_the_originals_size() {
    let mut rung = Rung::new();
    assert!(run(&mut rung, DESKTOP, 120.0).is_empty());
    assert_eq!(rung.step(), ReflectionResolution::Original);
    assert!(!rung.holds_extent());
    // Paced at the budget too: a vsync loop that holds its rate.
    let paced = Machine {
        base: 5.0 * MS,
        paced: true,
        ..DESKTOP
    };
    assert!(run(&mut rung, paced, 120.0).is_empty());
}

#[test]
fn a_target_under_the_floor_is_kept_whatever_its_share() {
    // A frame that misses a 240 Hz limit with a 0.6 ms target, 11 % of it: above the share,
    // under FLOOR_SECONDS, so the coarser view would buy too little to give it up.
    let machine = Machine {
        base: 4.9 * MS,
        glass: [0.6 * MS, 0.2 * MS, 0.1 * MS],
        period: 1.0 / 240.0,
        paced: false,
    };
    let mut rung = Rung::new();
    assert!(run(&mut rung, machine, 60.0).is_empty());
}

#[test]
fn a_mali_cost_settles_at_quarter_and_stays() {
    let mut rung = Rung::new();
    let changes = run(&mut rung, MALI, 10.0);
    let steps: Vec<_> = changes.iter().map(|c| c.to).collect();
    assert_eq!(
        steps,
        [ReflectionResolution::Half, ReflectionResolution::Quarter]
    );
    assert!(
        run(&mut rung, MALI, 300.0).is_empty(),
        "it must not flip back"
    );
    assert_eq!(rung.step(), ReflectionResolution::Quarter);
}

#[test]
fn a_cost_that_fits_one_step_down_stays_there_without_oscillating() {
    // Misses 60 at the original's size by more than OVER; fits at half, and the learned
    // saving predicts the original would miss again, so it never restores.
    let machine = Machine {
        base: 14.5 * MS,
        glass: [4.0 * MS, 1.2 * MS, 0.5 * MS],
        period: 1.0 / 60.0,
        paced: true,
    };
    let mut rung = Rung::new();
    let changes = run(&mut rung, machine, 300.0);
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(rung.step(), ReflectionResolution::Half);
}

#[test]
fn headroom_restores_one_step_at_a_time_after_a_patient_run() {
    let mut rung = Rung::new();
    run(&mut rung, MALI, 10.0);
    assert_eq!(rung.step(), ReflectionResolution::Quarter);
    // The scene gets light enough for the original's glass: base 3 ms against 8.3.
    let light = Machine {
        base: 3.0 * MS,
        glass: [2.0 * MS, 0.6 * MS, 0.2 * MS],
        ..MALI
    };
    // The run is counted in frames of the budget's period (3 s of 120 Hz), as the blur rung's:
    // the first restore comes no sooner than that many frames into the light scene.
    let patience = crate::drs::restore_frames(MALI.period);
    let mut changes = Vec::new();
    let mut frames = 0u32;
    while frames < 20 * patience {
        let reading = light.reading(rung.step());
        frames += 1;
        if let Some(change) = rung.record(reading, true) {
            if changes.is_empty() {
                assert!(
                    frames >= patience,
                    "restored after {frames} of {patience} frames"
                );
            }
            changes.push(change);
        }
    }
    let steps: Vec<_> = changes.iter().map(|c| c.to).collect();
    assert_eq!(
        steps,
        [ReflectionResolution::Half, ReflectionResolution::Original]
    );
}

#[test]
fn a_restore_waits_for_the_extent_at_its_ceiling() {
    let mut rung = Rung::new();
    run(&mut rung, MALI, 10.0);
    let light = Machine {
        base: 3.0 * MS,
        glass: [2.0 * MS, 0.6 * MS, 0.2 * MS],
        ..MALI
    };
    let mut t = 0.0;
    while t < 60.0 {
        let reading = light.reading(rung.step());
        t += reading.frame;
        assert_eq!(rung.record(reading, false), None);
    }
}

#[test]
fn with_nothing_pacing_the_frame_it_is_inert() {
    let mut rung = Rung::new();
    let mut t = 0.0;
    while t < 30.0 {
        let reading = Reading {
            period: None,
            ..MALI.reading(rung.step())
        };
        t += reading.frame;
        assert_eq!(rung.record(reading, true), None);
    }
    assert!(!rung.holds_extent());
}

/// A cost the extent loop reads as far over `target`.
fn overrun(target: Target) -> Cost {
    let budget = target
        .scalable_budget(0.0, crate::drs::Residual::new())
        .unwrap();
    Cost {
        scalable: budget * 1.6,
        fixed: 0.0,
        blur: 0.0,
    }
}

#[test]
fn reflections_reach_quarter_before_the_render_scale_moves() {
    // 60: the Mali frame misses it at half as well, so the rung goes on to quarter.
    let target = Target::OFFERED[2];
    let limits = Limits::new((1440, 816), (360, 204), target);
    let machine = Machine {
        period: target.period().unwrap(),
        ..MALI
    };
    let mut drs = Controller::new();
    let mut t = 0.0;
    let mut quarter_at = None;
    let mut fell_at = None;
    while t < 30.0 {
        let reading = machine.reading(drs.reflection_step());
        t += reading.frame;
        drs.record_reflection(reading, limits);
        if drs.reflection_step() == ReflectionResolution::Quarter && quarter_at.is_none() {
            quarter_at = Some(t);
        }
        if drs.record(overrun(target), None, limits).is_some() && fell_at.is_none() {
            fell_at = Some(t);
        }
    }
    let (quarter_at, fell_at) = (quarter_at.unwrap(), fell_at.unwrap());
    assert!(
        quarter_at < fell_at,
        "quarter at {quarter_at} s, fell at {fell_at} s"
    );
}

#[test]
fn a_small_share_leaves_the_extent_loop_exactly_as_it_was() {
    let target = Target::OFFERED[2];
    let limits = Limits::new((1440, 816), (720, 408), target);
    let budget = target
        .scalable_budget(0.0, crate::drs::Residual::new())
        .unwrap();
    let mut plain = Controller::new();
    let mut fed = Controller::new();
    let mut lowest = 1.0f32;
    // Over, then under, then over again: falls and rises both.
    let ratios = (0..900).map(|i| match i / 300 {
        0 => 1.4,
        1 => 0.5,
        _ => 1.2,
    });
    for (frame, ratio) in ratios.enumerate() {
        let cost = Cost {
            scalable: budget * ratio,
            fixed: 0.0,
            blur: 0.0,
        };
        let reading = Reading {
            glass: Some(0.12 * MS),
            gpu: Some(budget * ratio),
            frame: (budget * ratio).max(1.0 / 60.0) * 1.3,
            period: target.period(),
        };
        fed.record_reflection(reading, limits);
        assert_eq!(
            plain.record(cost, None, limits),
            fed.record(cost, None, limits),
            "frame {frame}"
        );
        assert_eq!(plain.scale(), fed.scale());
        lowest = lowest.min(plain.scale());
    }
    assert!(lowest < 1.0, "the sequence moves the extent");
    assert_eq!(fed.reflection_step(), ReflectionResolution::Original);
}
