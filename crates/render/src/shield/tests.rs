use super::*;

/// 60 Hz, the rate everything in this engine runs at and the rate the
/// original's own substep loop is expressed in.
const DT: f32 = 1.0 / 60.0;

/// Nothing is drawn until the pickup fires. The regression this guards is a
/// shell that renders around every craft from the grid.
#[test]
fn a_fresh_shield_draws_nothing() {
    let shield = ShipShield::new();
    assert!(!shield.visible());
}

/// `ShipShield_Activate` sets the object's active flag on the frame it is
/// called, but the shell **fades up from nothing and grows from `0.7`** rather
/// than appearing whole: the activation colour is transparent black and the
/// target is white.
#[test]
fn activating_fades_the_shell_up_rather_than_popping_it_in() {
    let mut shield = ShipShield::new();
    shield.activate();
    assert!(
        shield.visible(),
        "the shell is not up on the activate frame"
    );
    assert_eq!(shield.colour()[3], 0.0, "the shell appeared at full alpha");
    assert!(
        shield.scale() < SWELL_REST,
        "the shell appeared at full size: {}",
        shield.scale()
    );

    for _ in 0..40 {
        shield.advance(DT);
    }
    assert!(
        shield.colour()[3] > 0.9 * ALPHA_FLOOR,
        "the shell never faded up: alpha {}",
        shield.colour()[3]
    );
    assert!(
        (shield.scale() - (SWELL_REST + SCALE_BASE)).abs() < 0.02,
        "the shell never grew into place: {}",
        shield.scale()
    );
}

/// An absorbed hit flashes the shell **cyan** - the red channel alone drops to
/// zero - and it lerps back to white. Against the mesh's own blue-violet that
/// reads as a hue shift rather than a brightening, which is why the alpha is
/// deliberately untouched by the flash.
#[test]
fn an_absorbed_hit_flashes_the_shell_cyan_and_settles_back_to_white() {
    let mut shield = ShipShield::new();
    shield.activate();
    for _ in 0..60 {
        shield.advance(DT);
    }
    let settled = shield.colour();
    assert!(settled[0] > 0.9, "the settled shell is not white");

    shield.hit();
    let flash = shield.colour();
    assert_eq!(flash[0], 0.0, "the flash left the red channel up");
    assert_eq!(flash[1], 1.0);
    assert_eq!(flash[2], 1.0);

    for _ in 0..40 {
        shield.advance(DT);
    }
    assert!(
        shield.colour()[0] > 0.9,
        "the flash never settled back to white: red {}",
        shield.colour()[0]
    );
}

/// `ShipShield_Hit` stores `1.1` into the swell, and the drawn scale is
/// `swell + 0.012 + flicker * 0.012`. So the frame a hit lands is measurably
/// larger than the frame before it, whatever the flicker is doing: the flicker's
/// whole range is `0.012`, and the bulge is `0.1`.
#[test]
fn an_absorbed_hit_bulges_the_shell_past_the_flickers_reach() {
    let mut shield = ShipShield::new();
    shield.activate();
    // Settled first: an activation *starts* at `SWELL_ON_ACTIVATE`, so a hit
    // measured against the activate frame would be reading the grow-in.
    for _ in 0..60 {
        shield.advance(DT);
    }
    let resting = shield.scale();
    shield.hit();
    let bulged = shield.scale();
    assert!(
        bulged - resting > SCALE_FLICKER,
        "a hit moved the scale by {} - inside the flicker's own {SCALE_FLICKER}",
        bulged - resting
    );
    assert!((bulged - resting - (SWELL_ON_HIT - SWELL_REST)).abs() < 1e-6);
}

/// The swell settles back at `0.2` per substep, which is about a fifth of a
/// second. Pinned as a duration rather than as a rate so that changing the
/// substep loop to a closed form has to keep the same feel.
#[test]
fn a_bulge_settles_in_about_a_fifth_of_a_second() {
    let mut shield = ShipShield::new();
    shield.activate();
    shield.hit();
    for _ in 0..12 {
        shield.advance(DT);
    }
    // `0.8^12` of the original tenth is under a hundredth of the shell.
    let left = shield.scale() - (SWELL_REST + SCALE_BASE + flicker(shield.time) * SCALE_FLICKER);
    assert!(left.abs() < 0.01, "{left} of the bulge left after 12 ticks");
}

/// A hit on a shell that is not up does nothing. Both of the original's callers
/// test the active flag first, so this is its guard rather than a convenience.
#[test]
fn a_hit_with_no_shield_up_is_a_no_op() {
    let mut shield = ShipShield::new();
    shield.hit();
    assert!(!shield.visible());
    assert!(
        (shield.scale() - (SWELL_REST + SCALE_BASE + flicker(0.0) * SCALE_FLICKER)).abs() < 1e-6
    );
}

/// The alpha flicker never dims the shell below half its colour and never
/// brightens it past it. Both bounds fall out of `sin`'s own `-1..=1` against
/// the recovered `0.25`/`0.75` split, and they are what say the shell breathes
/// rather than blinks.
#[test]
fn the_alpha_flicker_stays_inside_its_recovered_quarter() {
    let mut shield = ShipShield::new();
    shield.activate();
    // Past the fade-up, so the bound is about the flicker rather than about the
    // colour still ramping toward `TARGET_COLOUR`.
    for _ in 0..120 {
        shield.advance(DT);
    }
    for tick in 0..600 {
        shield.advance(DT);
        let alpha = shield.colour()[3];
        assert!(
            (ALPHA_FLOOR..=1.0).contains(&alpha),
            "tick {tick}: alpha {alpha} left [{ALPHA_FLOOR}, 1]"
        );
    }
}

/// The flicker is `sin` of the shell's clock in seconds, so it is a pure
/// function - a capture at a given time is reproducible - and it completes one
/// cycle in `2*pi` seconds rather than several a second.
///
/// **The period is the assertion that matters.** The first version of this
/// module invented a ~2.7 Hz shimmer for the same slot, which is a visibly
/// different effect; pinning the period is what makes that regression fail a
/// test rather than pass review.
#[test]
fn the_flicker_is_sin_of_the_clock_in_seconds() {
    for step in 0..200 {
        let t = step as f32 * 0.03;
        assert_eq!(flicker(t), flicker(t));
        assert!(
            (-1.0..=1.0).contains(&flicker(t)),
            "flicker({t}) = {} left [-1, 1]",
            flicker(t)
        );
    }
    let period = std::f32::consts::TAU;
    assert!(flicker(period / 4.0) > 0.99, "the peak is not a quarter in");
    assert!(
        flicker(period * 3.0 / 4.0) < -0.99,
        "the trough is not three quarters in"
    );
    assert!(
        (flicker(period) - flicker(0.0)).abs() < 1e-5,
        "the cycle is not 2*pi seconds long"
    );
}

/// Two shells activated on the same frame stay in step. Same argument as above,
/// stated against the type rather than the function.
#[test]
fn two_shells_raised_together_flicker_together() {
    let mut a = ShipShield::new();
    let mut b = ShipShield::new();
    a.activate();
    b.activate();
    for _ in 0..90 {
        a.advance(DT);
        b.advance(DT);
    }
    assert_eq!(a.colour(), b.colour());
    assert_eq!(a.scale(), b.scale());
}

/// `advance` on a shell that is not up must not move its clock: a shield raised
/// later would otherwise start mid-waveform, which is the same class of bug the
/// boost plume's free-running clock was.
#[test]
fn an_inactive_shell_does_not_age() {
    let mut shield = ShipShield::new();
    for _ in 0..120 {
        shield.advance(DT);
    }
    shield.activate();
    let mut fresh = ShipShield::new();
    fresh.activate();
    assert_eq!(shield.colour(), fresh.colour());
}

/// A `dt` shorter than the original's own `1/60` substep advances neither lerp.
/// That is the original's `(int)(dt / 0.016666668)` and it is kept rather than
/// smoothed, so a caller running faster than 60 Hz gets the original's
/// behaviour rather than a different settle time.
#[test]
fn a_substep_shorter_than_the_originals_moves_no_lerp() {
    let mut shield = ShipShield::new();
    shield.activate();
    shield.hit();
    let bulged = shield.swell;
    shield.advance(SUBSTEP * 0.5);
    assert_eq!(shield.swell, bulged);
}

/// Deactivating leaves the shell up. It disappears when its alpha crosses
/// `0.1`, which is what makes an expiring shield dissolve **while expanding** -
/// the deactivate retargets the swell to `1.2`, above rest, so the shell blows
/// outward as it thins.
#[test]
fn deactivating_fades_rather_than_hiding() {
    let mut shield = ShipShield::new();
    shield.activate();
    for _ in 0..60 {
        shield.advance(DT);
    }
    let settled = shield.scale();
    shield.deactivate();
    for _ in 0..6 {
        shield.advance(DT);
    }
    assert!(
        shield.scale() > settled,
        "an expiring shell shrank instead of expanding: {settled} -> {}",
        shield.scale()
    );
    assert!(
        shield.visible(),
        "the shell vanished on the deactivate frame"
    );
    for _ in 0..120 {
        shield.advance(DT);
    }
    assert!(!shield.visible(), "the shell never finished fading");
}

/// Deactivating a shell that is not up does nothing, so a caller that drives
/// this off a timer crossing zero can call it unconditionally.
#[test]
fn deactivating_nothing_is_a_no_op() {
    let mut shield = ShipShield::new();
    shield.deactivate();
    assert!(!shield.visible());
    shield.advance(DT);
    assert!(!shield.visible());
}

/// HD's own hit flash is **amber**, not Pulse's cyan - the one confirmed
/// colour difference `docs/ghidra/functions/ps3-hdfury-eu/shield.md` found.
/// Same law, different palette: everything else about the bulge is untouched.
#[test]
fn hds_palette_flashes_amber_where_pulses_flashes_cyan() {
    let mut pulse = ShipShield::new();
    let mut hd = ShipShield::with_palette(HD_PALETTE);
    pulse.activate();
    hd.activate();
    for _ in 0..60 {
        pulse.advance(DT);
        hd.advance(DT);
    }
    pulse.hit();
    hd.hit();
    assert_eq!(pulse.colour()[..3], HIT_COLOUR[..3]);
    assert_eq!(hd.colour()[..3], HD_HIT_COLOUR[..3]);
    assert_ne!(pulse.colour()[..3], hd.colour()[..3]);
}

/// HD's activation colour is a dim red rather than Pulse's transparent black,
/// but both start at alpha `0.0` - the fade-up itself is unchanged, only the
/// hue it starts from moves.
#[test]
fn hds_palette_starts_the_fade_up_from_a_different_hue_at_the_same_alpha() {
    let pulse = ShipShield::new();
    let hd = ShipShield::with_palette(HD_PALETTE);
    assert_eq!(pulse.colour()[3], 0.0);
    assert_eq!(hd.colour()[3], 0.0);
    assert_eq!(pulse.rgba, ACTIVATION_COLOUR);
    assert_eq!(hd.rgba, HD_ACTIVATION_COLOUR);
    assert_ne!(pulse.rgba, hd.rgba);
}

/// Both palettes settle to the same target: HD's own steady-state colour was
/// not recovered (see [`HD_PALETTE`]'s doc comment), so it is Pulse's white,
/// chosen rather than measured. This pins that choice rather than letting a
/// future palette silently start differing here too.
#[test]
fn both_palettes_settle_to_the_same_unmeasured_target() {
    assert_eq!(PULSE_PALETTE.target, HD_PALETTE.target);
    assert_eq!(HD_PALETTE.target, TARGET_COLOUR);
}

/// The live object, frame by frame: `ShipShield_Update`'s own `dt` argument and
/// the colour, swell and clock it found on entry, read off PPSSPP (Time Trial,
/// Talon's Junction, Venom, fire bit `0x20` at speed 106; `psp-weapon-pair.py
/// shield --probe shield`, 2026-10-01). 30 of the 200 frames logged.
///
/// Fed the same `dt` values, [`ShipShield::advance`] reproduces every one: the
/// activation, the `0.15` fade, the `0.2` swell and the clock are the original's
/// own, and **nothing between `Shield_Fire` and the first drawn frame delays the
/// shell**. The shell's apparent onset in the original is later than a 60 Hz
/// tick-for-tick run's because the original's `dt` is a measured frame time that
/// jitters about `1/59.94` s: `(int)(dt / SUBSTEP)` is `0` on 47 of the 200 frames
/// logged (`dt` under `1/60`), so those frames advance neither lerp. That is the
/// emulator's frame-timer jitter acting on the original's variable timestep, not a
/// rule of the game, and a fixed 60 Hz engine does not reproduce it.
const LIVE_DT: [f32; 30] = [
    0.0166790001,
    0.0167859998,
    0.0165890008,
    0.0166720003,
    0.0166829992,
    0.016694,
    0.0166779999,
    0.0166829992,
    0.0166800003,
    0.0172600001,
    0.0160610005,
    0.0166209992,
    0.0169319995,
    0.0164410006,
    0.0166779999,
    0.0167720001,
    0.0165890008,
    0.0166829992,
    0.0166880004,
    0.0166890007,
    0.0166730005,
    0.0166779999,
    0.0166810006,
    0.0169600006,
    0.0164119992,
    0.0166779999,
    0.0168650001,
    0.0165020004,
    0.0166820008,
    0.0166859999,
];
const LIVE_COLOUR: [f32; 30] = [
    0.0,
    0.150000006,
    0.277500033,
    0.277500033,
    0.385875046,
    0.477993786,
    0.556294739,
    0.622850537,
    0.679422975,
    0.727509558,
    0.768383145,
    0.768383145,
    0.768383145,
    0.803125679,
    0.803125679,
    0.832656801,
    0.857758284,
    0.857758284,
    0.879094541,
    0.897230387,
    0.912645817,
    0.925748944,
    0.936886609,
    0.946353614,
    0.954400599,
    0.954400599,
    0.96124053,
    0.967054427,
    0.967054427,
    0.971996248,
];
const LIVE_SWELL: [f32; 30] = [
    0.699999988,
    0.75999999,
    0.807999969,
    0.807999969,
    0.846399963,
    0.877119958,
    0.901695967,
    0.921356797,
    0.93708545,
    0.949668348,
    0.959734678,
    0.959734678,
    0.959734678,
    0.967787743,
    0.967787743,
    0.97423017,
    0.979384124,
    0.979384124,
    0.983507276,
    0.986805797,
    0.989444613,
    0.991555691,
    0.993244529,
    0.994595647,
    0.995676517,
    0.995676517,
    0.996541202,
    0.997232974,
    0.997232974,
    0.997786403,
];
const LIVE_CLOCK: [f32; 30] = [
    0.0,
    0.0166790001,
    0.033464998,
    0.0500539988,
    0.0667259991,
    0.0834089965,
    0.100102998,
    0.116780996,
    0.133463994,
    0.150143996,
    0.167403996,
    0.183465004,
    0.200085998,
    0.217017993,
    0.233458996,
    0.250137001,
    0.266909003,
    0.283497989,
    0.300181001,
    0.316868991,
    0.333557993,
    0.350230992,
    0.366908997,
    0.383589983,
    0.400549978,
    0.416961968,
    0.433639973,
    0.450504959,
    0.467006952,
    0.483688951,
];

#[test]
fn the_live_objects_fade_swell_and_clock_are_reproduced_from_its_own_dts() {
    let mut shield = ShipShield::new();
    shield.activate();
    for i in 0..30 {
        assert!(
            (shield.rgba[0] - LIVE_COLOUR[i]).abs() < 1e-5,
            "frame {i}: colour {} against the live {}",
            shield.rgba[0],
            LIVE_COLOUR[i]
        );
        assert!(
            (shield.swell - LIVE_SWELL[i]).abs() < 1e-5,
            "frame {i}: swell {} against the live {}",
            shield.swell,
            LIVE_SWELL[i]
        );
        assert!(
            (shield.time - LIVE_CLOCK[i]).abs() < 1e-5,
            "frame {i}: clock {} against the live {}",
            shield.time,
            LIVE_CLOCK[i]
        );
        shield.advance(LIVE_DT[i]);
    }
}

/// The same 30 frames at a fixed 60 Hz are ahead of the live object: this engine
/// takes a substep every tick where the live frame timer dropped a quarter of
/// them. Pinned so the difference is a stated number and not a surprise.
#[test]
fn a_fixed_60_hz_shield_runs_ahead_of_the_jittered_live_one() {
    let mut fixed = ShipShield::new();
    fixed.activate();
    for _ in 0..29 {
        fixed.advance(DT);
    }
    assert!(
        fixed.rgba[0] > LIVE_COLOUR[29] + 0.015,
        "fixed {} against live {}",
        fixed.rgba[0],
        LIVE_COLOUR[29]
    );
}
