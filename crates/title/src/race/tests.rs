use super::*;

const PULSE: ZoneCircuit = ZoneCircuit::Prefixed("zone_");
const PURE: ZoneCircuit = ZoneCircuit::Separate(
    &[
        r"Data\Zone\01_Zone\track.vex",
        r"Data\Zone\02_Zone\track.vex",
        r"Data\Zone\03_Zone\track.vex",
        r"Data\Zone\04_Zone\track.vex",
    ],
    false,
);
const HD: ZoneCircuit = ZoneCircuit::Separate(
    &[
        "/data/environments/zone_1/track.vex",
        "/data/environments/zone_2/track.vex",
        "/data/environments/zone_3/track.vex",
        "/data/environments/zone_4/track.vex",
    ],
    true,
);

/// The prefix goes on the *file*, not the front of the whole entry name,
/// and both separators the corpus uses are directory separators.
#[test]
fn the_prefix_lands_on_the_file_name() {
    assert_eq!(
        PULSE.variant_of(r"Data\Environments\16_Track\track.vex"),
        r"Data\Environments\16_Track\zone_track.vex"
    );
    assert_eq!(
        PULSE.variant_of("/data/environments/talons_junction/track.vex"),
        "/data/environments/talons_junction/zone_track.vex"
    );
}

/// The `_reversed` suffix is part of the file name, so prefixing composes
/// with it and produces the fourth of the four names a circuit ships.
#[test]
fn the_prefix_composes_with_the_reversed_suffix() {
    assert_eq!(
        PULSE.variant_of(r"Data\Environments\16_Track\track_reversed.vex"),
        r"Data\Environments\16_Track\zone_track_reversed.vex"
    );
}

/// `--track` naming a Zone circuit outright must not be prefixed twice.
#[test]
fn prefixing_an_already_prefixed_name_changes_nothing() {
    let zone = r"Data\Environments\16_Track\zone_track.vex";
    assert_eq!(PULSE.variant_of(zone), zone);
}

/// A title whose Zone circuits are their own has nothing to derive from a
/// race circuit: naming one of its *own* Zone circuits stands, and the
/// default is the title's own first Zone circuit rather than a rewrite of
/// its race one.
#[test]
fn separate_circuits_are_never_rewritten() {
    let named = r"Data\Zone\03_Zone\track.vex";
    assert_eq!(PURE.variant_of(named), named);
    assert_eq!(
        PURE.default_track(r"Data\Environments\01_Vineta_K\track.vex"),
        r"Data\Zone\01_Zone\track.vex"
    );
    assert_eq!(
        PULSE.default_track(r"Data\Environments\16_Track\track.vex"),
        r"Data\Environments\16_Track\zone_track.vex"
    );
}

/// **The regression this type exists to close.** A menu that switches to
/// Zone mode without touching the Circuit row hands `variant_of` a *race*
/// circuit, not one of the title's own Zone ones - and until this test
/// existed, `Separate` passed it straight through, so the "zone race"
/// loaded whichever race environment happened to be selected.
#[test]
fn separate_substitutes_its_own_default_for_a_race_circuit() {
    let race_circuit = r"Data\Environments\01_Vineta_K\track.vex";
    assert_eq!(
        PURE.variant_of(race_circuit),
        r"Data\Zone\01_Zone\track.vex"
    );
}

/// **`also_race_circuits` skips the substitution entirely.** A menu that
/// offers ordinary race circuits in Zone mode (HD, unverified) must not then
/// silently substitute the title's own default underneath whichever one gets
/// picked - that would reproduce the exact bug the test above exists to
/// catch, one layer further down.
#[test]
fn also_race_circuits_never_substitutes() {
    let race_circuit = "/data/environments/15_anulpha_pass/track.vex";
    assert_eq!(HD.variant_of(race_circuit), race_circuit);
}

/// The membership check folds case and separators the same way a PSARC
/// container does before it looks a path up, so a caller that spells its
/// own Zone circuit differently from the list still gets it honoured
/// rather than silently substituted.
#[test]
fn separate_recognises_its_own_circuit_however_it_is_spelled() {
    let differently_spelled = "data/zone/03_zone/track.vex";
    assert_eq!(
        PURE.variant_of(differently_spelled),
        differently_spelled,
        "a name that matches one of PURE's own Zone circuits case- and \
         separator-insensitively should come back exactly as given"
    );
}

/// **2048's shape.** Unlike [`ZoneCraft::ModelsInTeam`] and
/// [`ZoneCraft::OwnShip`], nothing changes at all: the directory stays the
/// player's team and both model stems fall back to whatever a race would
/// have used, which is the point - see [`ZoneCraft::PlayerShip`]'s own
/// docs for why.
#[test]
fn player_ship_resolves_to_nothing_but_the_players_own_team() {
    let craft = ZoneCraft::PlayerShip;
    assert_eq!(craft.directory("feisar2048\\3"), "feisar2048\\3");
    assert_eq!(craft.hull(), None);
    assert_eq!(craft.boost(), None);
}

/// **2048's circuit shape, the companion of the craft one above.** Neither
/// method derives or substitutes anything - see
/// [`ZoneCircuit::SameCircuit`]'s own docs for why.
#[test]
fn same_circuit_resolves_to_exactly_what_it_was_given() {
    let zone = ZoneCircuit::SameCircuit;
    let track = r"Data\art\published\environments\altima\track.vex";
    assert_eq!(zone.variant_of(track), track);
    assert_eq!(zone.default_track(track), track);
}

/// A stand-in for `oag_game::catalogue::Track`, carrying only the two
/// facts [`ZoneCircuit::menu_tracks`] asks about: an id to tell the
/// results apart by, and whether a race circuit is zone-available. This
/// module cannot depend on `oag-game` to use the real type - see
/// [`ZoneCircuit::menu_tracks`]'s own docs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StubTrack {
    id: &'static str,
    available_in_zone: bool,
}

/// [`ZoneCircuit::Prefixed`] filters the race list down to the circuits
/// flagged `availableInZone`, in place, and never calls `by_own_names` - a
/// title on this shape has no Zone circuits of its own to ask for by name.
#[test]
fn prefixed_filters_the_race_list_by_available_in_zone() {
    let race_tracks = [
        StubTrack {
            id: "16_Track",
            available_in_zone: true,
        },
        StubTrack {
            id: "01_Track",
            available_in_zone: false,
        },
    ];
    let offered = PULSE.menu_tracks(
        &race_tracks,
        |track| track.available_in_zone,
        |names| panic!("Prefixed should never ask by_own_names({names:?})"),
    );
    assert_eq!(offered, [race_tracks[0].clone()]);
}

/// [`ZoneCircuit::Separate`] ignores the race list entirely and asks
/// `by_own_names` for its own four names instead - the title's own Zone
/// circuits are declared apart from the race ones, not marked among them,
/// and querying by name rather than by `type` is what makes this work on
/// HD/Fury too, whose four are `type="Race"` with a `zone="true"` flag
/// rather than `type="Zone"`.
#[test]
fn separate_asks_by_own_names_and_ignores_the_race_list() {
    let race_tracks = [StubTrack {
        id: "01_Vineta_K",
        available_in_zone: false,
    }];
    let zone_tracks = vec![StubTrack {
        id: "01_Zone",
        available_in_zone: false,
    }];
    let offered = PURE.menu_tracks(
        &race_tracks,
        |_| panic!("Separate should never ask available_in_zone"),
        |names| {
            assert_eq!(
                names,
                [
                    r"Data\Zone\01_Zone\track.vex",
                    r"Data\Zone\02_Zone\track.vex",
                    r"Data\Zone\03_Zone\track.vex",
                    r"Data\Zone\04_Zone\track.vex",
                ]
            );
            zone_tracks.clone()
        },
    );
    assert_eq!(offered, zone_tracks);
}

/// **`also_race_circuits` appends the race list after the title's own four,
/// own circuits first, without duplicating one already returned by
/// `by_own_names`** - HD's own four are `type="Race"` entries too, so a
/// caller's `race_tracks` can legitimately already contain them.
#[test]
fn also_race_circuits_appends_the_race_list_without_duplicating() {
    let zone_1 = StubTrack {
        id: "25_Track",
        available_in_zone: false,
    };
    let anulpha_pass = StubTrack {
        id: "08_Track",
        available_in_zone: false,
    };
    let race_tracks = [zone_1.clone(), anulpha_pass.clone()];
    let offered = HD.menu_tracks(
        &race_tracks,
        |_| panic!("Separate should never ask available_in_zone"),
        |_names| vec![zone_1.clone()],
    );
    assert_eq!(
        offered,
        [zone_1, anulpha_pass],
        "the own circuit stays first and is not repeated"
    );
}

/// [`ZoneCircuit::SameCircuit`] hands the race list back unfiltered - 2048
/// runs Zone on whichever circuit is already picked, so the menu offers
/// exactly the same circuits it would for any other mode.
#[test]
fn same_circuit_offers_the_race_list_unfiltered() {
    let race_tracks = [
        StubTrack {
            id: "altima",
            available_in_zone: false,
        },
        StubTrack {
            id: "sebenco",
            available_in_zone: false,
        },
    ];
    let offered = ZoneCircuit::SameCircuit.menu_tracks(
        &race_tracks,
        |_| panic!("SameCircuit should never ask available_in_zone"),
        |names| panic!("SameCircuit should never ask by_own_names({names:?})"),
    );
    assert_eq!(offered, race_tracks);
}
