//! The first speed blur shot of a visit to photo mode against the shots after it. The game binary
//! is handed in by the parent module, which is the one the test groups name.
//!
//! The scripted drive draws few frames while the mode ticks, so the zoom ring's history at the
//! first shot is what one history draw over the race's own tick left, where a player's preview
//! has drawn it frame after frame. A shot settles the ring over its frozen scene
//! (`oag_post::hd_zoom::SETTLE_PASSES`), so no shot depends on the frames drawn before it.

use super::shot_fidelity::meta_of;
use super::*;

/// Where the zoom ring's streak lands: a strip along each side, clear of the HUD.
const LEFT_EDGE: (f32, f32, f32, f32) = (0.0, 0.25, 0.08, 0.70);
const RIGHT_EDGE: (f32, f32, f32, f32) = (0.92, 0.25, 1.0, 0.70);

/// Two speed blur shots of one frozen scene, in one visit, differ by no more than this along the
/// sides, on average. Measured with the ring settled: 0.02 on HD, 0.00 on Pulse, and on 2048 0.23
/// and 0.33, which is also how far its second shot is from its third (a state of 2048's own post
/// chain that moves between shots, not the ring's history). Without the settle: 3.6 and 6.8 on
/// HD, 4.6 and 5.3 on Pulse, 3.8 and 7.6 on 2048.
const TOLERANCE: f32 = 0.5;

/// The menus to a Time Trial on a disc image.
const FROM_THE_MENUS: &str = "boot; select racebox; press cross; until page=race; \
     set race.mode=time_trial; select launch_race; press cross; until screen=picker; \
     press cross; until screen.id=ship-select; press cross; until stage=race within 4000;";

/// A speed blur shot at 100 taken first in a visit to `image`, and another of the same frozen
/// scene after sixty ticks of the mode: the two pictures, as their mean difference along the
/// left edge, the right edge and the whole picture.
fn two_shots_of_a_visit(
    image: &Path,
    tag: &str,
    to_the_race: &str,
    args: &[&str],
) -> (f32, f32, f32) {
    let shots = oag_testdata::scratch::dir(&format!("photo-first-speed-{tag}"));
    let run = drive_with(
        image,
        tag,
        &format!(
            "{to_the_race} ticks 300; hold cross 400; escape; until stage=menu; \
             select photo_mode; press cross; \
             until photo=free within 200; press select select; ticks 5; hold left 8; ticks 3; \
             press start; hold down 1; ticks 2; hold down 1; ticks 2; hold right 260; ticks 3; \
             press square; ticks 3; press cross; ticks 3; ticks 60; press cross; ticks 3; \
             press circle; until photo=none within 50"
        ),
        &shots,
        args,
    );
    assert!(run.status.success(), "{}", run.stderr);
    let photos = the_photos(&run);
    assert_eq!(photos.len(), 2, "{photos:?}");
    for photo in &photos {
        assert_eq!(meta_of(photo).picture.expect("controls").speed_blur, 100);
    }
    let (first, second) = (decode(&photos[0]), decode(&photos[1]));
    let (left, right, whole) = (
        difference(&first, &second, LEFT_EDGE),
        difference(&first, &second, RIGHT_EDGE),
        difference(&first, &second, ALL),
    );
    println!(
        "{tag}: the first shot against the second: left edge {left:.2}, right edge {right:.2}, whole {whole:.2}"
    );
    (left, right, whole)
}

/// On HD, whose chain draws the ring inside its bloom (`hd_bloom`), the first speed blur shot of
/// a visit matches the one after it along the sides, where the streak is.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_first_speed_blur_shot_of_a_visit_matches_the_one_after_it() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let (left, right, _) = two_shots_of_a_visit(&image, "hd", FROM_THE_MENUS, &[]);
    assert!(left < TOLERANCE, "the left edge is {left} apart");
    assert!(right < TOLERANCE, "the right edge is {right} apart");
}

/// The same on a title whose ring is HD's borrowed (`oag_post::hd_zoom::Host`), Pulse on the PSP.
#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_first_speed_blur_shot_of_a_visit_matches_the_one_after_it_on_a_borrowed_ring() {
    let Some(image) = pulse() else {
        return;
    };
    let (left, right, _) = two_shots_of_a_visit(&image, "pulse", FROM_THE_MENUS, &[]);
    assert!(left < TOLERANCE, "the left edge is {left} apart");
    assert!(right < TOLERANCE, "the right edge is {right} apart");
}

/// The same on 2048, which borrows HD's ring as Pulse does.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_first_speed_blur_shot_of_a_visit_matches_the_one_after_it_on_2048() {
    let Some(image) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    // 2048's front end goes on into its first event by itself.
    let (left, right, _) = two_shots_of_a_visit(&image, "2048", "boot;", &[]);
    assert!(left < TOLERANCE, "the left edge is {left} apart");
    assert!(right < TOLERANCE, "the right edge is {right} apart");
}
