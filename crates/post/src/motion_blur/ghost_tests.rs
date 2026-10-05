//! Ghosting: smear that lands where no surface's own motion put it.
//!
//! Each scene is dark with bright content whose swept path is known exactly,
//! so the energy the chain deposits outside that path is a number. Method and
//! findings: `docs/rendering/motion-blur.md`, "Ghosting".
//!
//! These scenes have uniform velocity per region, so the tile lookup never
//! differs between the pre-grain and the bilinear shader: they read the same
//! numbers on both (checked 2026-10-05). They guard the chain against leaking
//! energy outside a surface's swept path; they do not isolate the lookup.

use super::lattice_tests::{H, Pixel, W, frame, gpu};

/// A shutter's worth of travel at 0.04 uv a tick and strength 0.75, in
/// pixels: the gather spreads a surface half of this to each side.
const REACH_PX: f32 = 0.04 * 0.75 * W as f32;
const FAST: f32 = 0.04;

/// Energy (sum of red) a frame holds where `outside` says the scene's own
/// motion cannot have put any, over the energy the input held.
fn leak(rgba: &[u8], input: &dyn Fn(u32, u32) -> Pixel, outside: &dyn Fn(u32, u32) -> bool) -> f32 {
    let (mut leaked, mut total) = (0.0f32, 0.0f32);
    for y in 300..H - 300 {
        for x in 0..W {
            let i = ((y * W + x) * 4) as usize;
            let was = f32::from(input(x, y).0[0]);
            total += was;
            if outside(x, y) {
                leaked += (f32::from(rgba[i]) - was).abs();
            }
        }
    }
    leaked / total.max(1.0)
}

/// A still bright block in front of a fast dark background: nothing about it
/// moves, so nothing about it may change.
fn still_block(x: u32, y: u32) -> Pixel {
    if (900..1000).contains(&x) && (400..700).contains(&y) {
        ([220; 3], [0.0, 0.0], 0.4)
    } else {
        ([30; 3], [FAST, 0.0], 0.6)
    }
}

/// Two fast regions moving opposite ways, a thin bright line riding in each:
/// the forward-moving camera's two sides of the vanishing point.
fn opposing(x: u32, _y: u32) -> Pixel {
    let left = x < 960;
    let line = if left { 900 } else { 1020 };
    let colour = if x.abs_diff(line) < 2 { 255 } else { 0 };
    ([colour; 3], [if left { FAST } else { -FAST }, 0.0], 0.5)
}

/// A horizontal mover beside a vertical one: each line's smear has a direction
/// the other region's pixels do not share.
fn perpendicular(x: u32, _y: u32) -> Pixel {
    let left = x < 960;
    let colour = if left && x.abs_diff(900) < 2 { 255 } else { 0 };
    (
        [colour; 3],
        if left { [FAST, 0.0] } else { [0.0, FAST] },
        0.5,
    )
}

fn run(strength: f32) -> Option<[(&'static str, f32); 3]> {
    let (device, queue) = gpu()?;
    let swept = |line: u32| move |x: u32, _y: u32| x.abs_diff(line) as f32 > REACH_PX / 2.0 + 3.0;
    let off_block = |x: u32, y: u32| !((900..1000).contains(&x) && (400..700).contains(&y));
    let out_still = frame(&device, &queue, strength, &still_block);
    let out_opp = frame(&device, &queue, strength, &opposing);
    let out_perp = frame(&device, &queue, strength, &perpendicular);
    let both = |x: u32, y: u32| swept(900)(x, y) && swept(1020)(x, y);
    Some([
        ("still block", leak(&out_still, &still_block, &off_block)),
        ("opposing", leak(&out_opp, &opposing, &both)),
        (
            "perpendicular",
            leak(&out_perp, &perpendicular, &swept(900)),
        ),
    ])
}

#[test]
fn line_profile_is_reported() {
    let Some((device, queue)) = gpu() else {
        return;
    };
    for v in [0.02f32, 0.04, 0.1] {
        let line = |x: u32, _y: u32| -> Pixel {
            let c = if x.abs_diff(960) < 2 { 255 } else { 0 };
            ([c; 3], [v, 0.0], 0.5)
        };
        let out = frame(&device, &queue, 0.75, &line);
        for y in [400u32, 401, 402] {
            let row: Vec<u8> = (900..1020)
                .map(|x| out[((y * W + x) * 4) as usize])
                .collect();
            eprintln!("v {v} y {y}: {row:?}");
        }
    }
}

#[test]
fn no_energy_lands_off_a_surfaces_swept_path() {
    let Some(found) = run(0.75) else {
        return;
    };
    for (name, value) in found {
        eprintln!("ghost {name}: {value:.4}");
        assert!(
            value < 0.01,
            "{name} leaks {value} of its energy off its swept path"
        );
    }
}
