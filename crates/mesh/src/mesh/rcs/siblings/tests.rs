use super::*;

fn model(vex: &str, spelling: Spelling) -> Option<String> {
    Siblings::spelled(vex, spelling).map(|s| s.model)
}

#[test]
fn a_plain_set_is_the_same_stem_with_each_extension() {
    let set = Siblings::plain("/data/ships/assegai/ship.vex").unwrap();
    assert_eq!(set.model, "/data/ships/assegai/ship.rcsmodel");
    assert_eq!(set.skeleton, "/data/ships/assegai/ship.rcsskeleton");
    assert_eq!(set.clip, "/data/ships/assegai/ship.rcsanimclip");
    assert_eq!(set.pvs, "/data/ships/assegai/ship.pvs");
    assert_eq!(
        Siblings::plain(r"Data\Ships\Assegai\Ship.VEX")
            .unwrap()
            .model,
        r"Data\Ships\Assegai\Ship.rcsmodel",
        "both containers match a name case-insensitively"
    );
    assert_eq!(Siblings::plain("Data\\Tex\\thing.mip"), None);
    assert_eq!(model(".vex", Spelling::Plain).as_deref(), Some(".rcsmodel"));
}

/// The Omega Collection's circuits spell the pairing with a `.final` infix, on
/// every companion, and the plain spelling is untouched.
#[test]
fn a_cooked_set_carries_the_final_infix_on_every_companion() {
    let track = r"Data\environments\tech_de_ra\track.vex";
    let set = Siblings::spelled(track, Spelling::Cooked).unwrap();
    let dir = r"Data\environments\tech_de_ra\";
    assert_eq!(set.model, format!("{dir}track.final.rcsmodel"));
    assert_eq!(set.skeleton, format!("{dir}track.final.rcsskeleton"));
    assert_eq!(set.clip, format!("{dir}track.final.rcsanimclip"));
    assert_eq!(set.pvs, format!("{dir}track.final.pvs"));
    assert_eq!(
        model(
            r"Data\environments\tech_de_ra\track_reversed.vex",
            Spelling::Cooked
        )
        .as_deref(),
        Some(format!("{dir}track_reversed.final.rcsmodel").as_str())
    );
    assert_eq!(
        Siblings::spelled("Data\\Tex\\thing.mip", Spelling::Cooked),
        None
    );
    assert_eq!(
        model(track, Spelling::Plain).as_deref(),
        Some(format!("{dir}track.rcsmodel").as_str())
    );
}

/// 2048's Zone model is one name for both directions of a circuit, in the
/// `.vex`'s own directory, with its own companions.
#[test]
fn the_zone_set_is_track_zone_beside_the_vex() {
    let set = Siblings::spelled("Data/environments/altima/track.vex", Spelling::Zone).unwrap();
    assert_eq!(set.model, "Data/environments/altima/trackZone.rcsmodel");
    assert_eq!(
        set.skeleton,
        "Data/environments/altima/trackZone.rcsskeleton"
    );
    assert_eq!(set.clip, "Data/environments/altima/trackZone.rcsanimclip");
    assert_eq!(set.pvs, "Data/environments/altima/trackZone.pvs");
    assert_eq!(
        model(r"D\altima\track_reversed.VEX", Spelling::Zone).as_deref(),
        Some(r"D\altima\trackZone.rcsmodel")
    );
    assert_eq!(
        model("track.vex", Spelling::Zone).as_deref(),
        Some("trackZone.rcsmodel")
    );
    assert_eq!(
        Siblings::spelled("Data/altima/track.bin", Spelling::Zone),
        None
    );
}

#[test]
fn a_probe_lists_zone_first_only_when_asked_then_plain_then_cooked() {
    let spellings = |zone| -> Vec<Spelling> {
        Siblings::probe("d/track.vex", zone)
            .into_iter()
            .map(|s| s.spelling)
            .collect()
    };
    assert_eq!(spellings(false), [Spelling::Plain, Spelling::Cooked]);
    assert_eq!(
        spellings(true),
        [Spelling::Zone, Spelling::Plain, Spelling::Cooked]
    );
    assert!(Siblings::probe("d/track.mip", true).is_empty());
}

/// A model already found gives its own companions, whatever its spelling.
#[test]
fn the_set_beside_a_found_model_follows_its_spelling() {
    let dir = r"Data\environments\tech_de_ra\";
    let found = Siblings::beside(&format!("{dir}track.final.rcsmodel")).unwrap();
    assert_eq!(found.spelling, Spelling::Cooked);
    assert_eq!(found.skeleton, format!("{dir}track.final.rcsskeleton"));
    assert_eq!(found.clip, format!("{dir}track.final.rcsanimclip"));
    assert_eq!(found.pvs, format!("{dir}track.final.pvs"));
    let reversed = Siblings::beside(&format!("{dir}track_reversed.final.rcsmodel")).unwrap();
    assert_eq!(reversed.pvs, format!("{dir}track_reversed.final.pvs"));
    assert_eq!(
        Siblings::beside("a/track.rcsmodel").unwrap().spelling,
        Spelling::Plain
    );
    assert_eq!(
        Siblings::beside("a/trackZone.rcsmodel").unwrap().spelling,
        Spelling::Zone
    );
    // Each probed model reads back as its own set.
    for set in Siblings::probe("a/b/track.vex", true) {
        assert_eq!(Siblings::beside(&set.model).as_ref(), Some(&set));
    }
    // Not an `.rcsmodel`: no set, and the caller falls back to the `.vex`'s.
    assert_eq!(Siblings::beside("track.vex"), None);
    assert_eq!(Siblings::beside("a/track.bin"), None);
    assert_eq!(Siblings::beside("l"), None);
}
