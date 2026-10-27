//! What the `handlingstats.xml` reader in [`super`] is asserted to do: scaled
//! tunables, per-class blocks, and the shapes a malformed file must not produce.
//! Its own file because the tests exceed the 200-line inline limit
//! (`scripts/check-file-size.py`).

use super::*;

/// Every number is **invented** (1, 2, 3 ... in document order): no shipped
/// tuning value may appear in this repository
/// (`docs/architecture/adr/0006-no-copyrighted-content.md`).
const HEADER: &str = concat!(
    r#"<InternalCamera fov="1" headtilt="2" height="3" length="4" pitch="5"/>"#,
    r#"<BackwardCamera fov="6" headtilt="7" height="8" length="9" pitch="10"/>"#,
    r#"<BonnetCamera fov="11" height="12" length="13" pitch="14"/>"#,
    r#"<ExternalCameraFar fov="15" lookat_height="16" lookat_length="17""#,
    r#" pos_height="18" pos_length="19" spring_horiz="20" spring_vert="21"/>"#,
    r#"<ExternalCameraClose fov="22" lookat_height="23" lookat_length="24""#,
    r#" pos_height="25" pos_length="26" spring_horiz="27" spring_vert="28"/>"#,
    r#"<AirbrakeGraphics amount="29" down_speed="30" up_speed="31"/>"#,
    r#"<Misc height="32" length="33" shield="34" easyshield="35" width="36""#,
    r#" weight_distribution="37"/>"#,
    r#"<FE speed="38" thrust="39" handling="40" shield="41"/>"#,
);

/// The seven per-class blocks, again with invented values.
const CLASS_BODY: &str = concat!(
    r#"<Engine accelcap="1" amount="2" falloff="3" gain="4" turbo="5"/>"#,
    r#"<Brakes amount="6" falloff="7" gain="8"/>"#,
    r#"<Turning amount="9" falloff="10" gain="11"/>"#,
    r#"<Airbrake amount="12" drag="13" falloff="14" gain="15" turn="16""#,
    r#" slidegrip="17" sideshift="18"/>"#,
    r#"<Antigrav grip_air="19" grip_ground="20" landing_rebound="21""#,
    r#" rebound="22" rebound_jump_time="23" ride_height="24"/>"#,
    r#"<Physical flight_gravity="25" mass="26" normal_gravity="27" track_gravity="28"/>"#,
    r#"<pitch pitch_air="29" pitch_ground="30" pitch_damping="31""#,
    r#" antigrav_height_adjust="32"/>"#,
);

/// The attribute count the schema lists: 41 in the header plus `team`, 32 per
/// class plus its `name`.
const FIXTURE_ATTRIBUTES: usize = 42 + 4 * 33;

fn class_block(name: &str) -> String {
    format!(r#"<Class name="{name}">{CLASS_BODY}</Class>"#)
}

fn document(classes: &[&str]) -> String {
    let blocks: String = classes.iter().map(|c| class_block(c)).collect();
    format!(r#"<Handling><Stats team="Testers">{HEADER}{blocks}</Stats></Handling>"#)
}

fn all_four() -> String {
    document(&["VENOM", "FLASH", "RAPIER", "PHANTOM"])
}

/// Every ` name="value"` span in `doc`, so a test can remove them one at a time.
fn attribute_spans(doc: &str) -> Vec<std::ops::Range<usize>> {
    let bytes = doc.as_bytes();
    let mut out = Vec::new();
    let mut at = 0usize;

    while let Some(eq) = doc[at..].find('=').map(|i| i + at) {
        let Some(open) = doc[eq + 1..].find('"').map(|i| i + eq + 1) else {
            break;
        };
        let Some(close) = doc[open + 1..].find('"').map(|i| i + open + 1) else {
            break;
        };
        let mut start = eq;
        while start > 0 && !bytes[start - 1].is_ascii_whitespace() && bytes[start - 1] != b'<' {
            start -= 1;
        }
        out.push(start..close + 1);
        at = close + 1;
    }

    out
}

#[test]
fn parses_a_whole_document() {
    let stats = parse(&all_four()).expect("well-formed fixture");
    assert_eq!(stats.team, "Testers");
    assert_eq!(stats.internal_camera.fov, 1.0);
    assert_eq!(stats.backward_camera.headtilt, Some(7.0));
    assert_eq!(stats.bonnet_camera.pitch, 14.0);
    assert_eq!(stats.external_camera_far.spring_vert, 21.0);
    assert_eq!(stats.external_camera_close.lookat_height, 23.0);
    assert_eq!(stats.airbrake_graphics.up_speed, 31.0);
    assert_eq!(stats.misc.weight_distribution, Some(37.0));
    assert_eq!(stats.fe.expect("the fixture carries <FE>").shield, 41.0);
}

#[test]
fn every_class_block_is_read_and_indexed_by_its_own_name() {
    let stats = parse(&all_four()).expect("well-formed fixture");
    for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
        assert_eq!(stats.classes[index].name, Some(class), "slot {index}");
        assert_eq!(stats.class(class).expect("four rungs").name, Some(class));
    }
    let phantom = stats.class(SpeedClass::PHANTOM).expect("four rungs");
    assert_eq!(phantom.engine.turbo, 5.0);
    assert_eq!(phantom.brakes.gain, 8.0);
    assert_eq!(phantom.turning.falloff, 10.0);
    assert_eq!(phantom.airbrake.sideshift, Some(18.0));
    assert_eq!(phantom.antigrav.ride_height, 24.0);
    assert_eq!(phantom.physical.mass, 26.0);
    assert_eq!(
        phantom
            .pitch
            .expect("this document authors <pitch>")
            .antigrav_height_adjust,
        32.0
    );
}

/// The key invariant: a silently defaulted attribute would be a physics bug that
/// reads as tuning, so each is removed in turn and every removal must be a typed
/// error.
#[test]
fn a_missing_attribute_is_an_error_not_a_default() {
    let doc = all_four();
    let spans = attribute_spans(&doc);
    assert_eq!(
        spans.len(),
        FIXTURE_ATTRIBUTES,
        "the fixture should carry every attribute the schema lists"
    );

    for span in spans {
        // Exempt: absent by *schema generation*, pinned separately: the three
        // Pulse added (`the_three_pulse_era_attributes_are_absent_rather_than_missing`)
        // and `headtilt`, which HD's team files drop and its mode ships keep.
        let text = &doc[span.clone()];
        if ["easyshield", "weight_distribution", "sideshift", "headtilt"]
            .iter()
            .any(|optional| text.trim_start().starts_with(optional))
        {
            continue;
        }

        let mut broken = doc.clone();
        broken.replace_range(span.clone(), "");
        let result = parse(&broken);
        assert!(
            matches!(result, Err(Error::MissingAttribute { .. })),
            "removing {} was tolerated: {result:?}",
            &doc[span]
        );
    }
}

#[test]
fn a_missing_element_is_an_error() {
    let doc = all_four();
    for element in [
        "InternalCamera",
        "BackwardCamera",
        "BonnetCamera",
        "ExternalCameraFar",
        "ExternalCameraClose",
        "AirbrakeGraphics",
        "Misc",
        // **`FE` and `pitch` are deliberately absent.** Pure omits both on both
        // pressings (`Stats::fe` and `Class::pitch` are `Option`), and they get
        // their own test so "may be absent" does not become "may be malformed".
        "Engine",
        "Brakes",
        "Turning",
        "Airbrake",
        "Antigrav",
        "Physical",
    ] {
        let open = format!("<{element} ");
        let at = doc.find(&open).expect("element in the fixture");
        let end = doc[at..].find("/>").expect("self-closing") + at + 2;
        let mut broken = doc.clone();
        broken.replace_range(at..end, "");
        assert_eq!(
            parse(&broken),
            Err(Error::MissingElement { element }),
            "dropping <{element}> was tolerated"
        );
    }
}

/// Dropping `headtilt` gives `None`; breaking one still fails.
///
/// **HD's team files omit the attribute** (`<InternalCamera fov="65" height="0"
/// length="3" pitch="0"/>` on Feisar) where its Zone and Detonator ships keep
/// it. Only the cockpit view applies it (`oag_render::camera::internal`); an
/// `Option` keeps a typo from becoming a silent zero.
#[test]
fn an_absent_headtilt_is_none_and_a_broken_one_is_still_an_error() {
    let doc = all_four();
    assert!(
        doc.contains(r#"headtilt="2""#),
        "the fixture has to author one for either half of this to mean anything"
    );

    let dropped = doc.replace(r#" headtilt="2""#, "");
    let stats = parse(&dropped).expect("a file with no headtilt is HD-shaped, not broken");
    assert_eq!(stats.internal_camera.headtilt, None);
    assert_eq!(
        stats.backward_camera.headtilt,
        Some(7.0),
        "and the other camera's own value is untouched"
    );

    let broken = doc.replace(r#"headtilt="2""#, r#"headtilt="tilt""#);
    assert!(
        matches!(parse(&broken), Err(Error::NotANumber { .. })),
        "an unparseable value must not collapse into the absent case"
    );
}

/// Dropping `<pitch>` gives `None`; breaking one still fails. An `Option` that
/// swallowed a parse error would turn a Pulse typo into a silent fall-through to
/// `oag_gameplay::handling::PITCH_STAND_IN`, another title's tuning.
#[test]
fn an_absent_pitch_is_none_and_a_broken_one_is_still_an_error() {
    let doc = all_four();
    assert!(
        doc.contains("<pitch "),
        "the fixture has to author one for either half of this to mean anything"
    );

    let mut dropped = doc.clone();
    while let Some(at) = dropped.find("<pitch ") {
        let end = dropped[at..].find("/>").expect("self-closing") + at + 2;
        dropped.replace_range(at..end, "");
    }
    let stats = parse(&dropped).expect("a file with no <pitch> is Pure-shaped, not broken");
    for class in &stats.classes {
        assert_eq!(
            class.pitch, None,
            "{}: an unauthored block is None, not a zeroed one",
            class.raw_name
        );
    }

    let broken = doc.replace(r#"pitch_air="29""#, r#"pitch_air="oops""#);
    assert!(
        broken != doc,
        "the fixture's pitch_air is 29; update this if it changes"
    );
    assert_eq!(
        parse(&broken),
        Err(Error::NotANumber {
            element: "pitch",
            attribute: "pitch_air",
            value: "oops".to_string(),
        }),
        "a malformed <pitch> must not pass as an absent one"
    );

    // An attribute *dropped* from a present block still errors (a half-written element).
    let short = doc.replace(r#"pitch_air="29" "#, "");
    assert_eq!(
        parse(&short),
        Err(Error::MissingAttribute {
            element: "pitch",
            attribute: "pitch_air",
        }),
        "a <pitch> missing an attribute is not an absent <pitch>"
    );
}

#[test]
/// A file naming *some* of Pulse's ladder must name all of it: the check applies
/// only where the file uses Pulse's ladder, so a partial Pulse file errors and
/// another generation's ladder does not.
fn a_partly_present_pulse_ladder_is_an_error() {
    assert_eq!(
        parse(&document(&["VENOM", "FLASH", "RAPIER"])),
        Err(Error::MissingClass {
            class: SpeedClass::PHANTOM
        })
    );
    assert_eq!(
        parse(&document(&["FLASH", "RAPIER", "PHANTOM"])),
        Err(Error::MissingClass {
            class: SpeedClass::VENOM
        })
    );
    assert_eq!(
        parse(&document(&["VENOM"])),
        Err(Error::MissingClass {
            class: SpeedClass::FLASH
        })
    );
}

/// The three attributes Pulse added read `None` when absent ("this schema
/// predates the field") and the exemption goes no further; a present but broken
/// value still errors, or a typo would pass as an older file.
#[test]
fn the_three_pulse_era_attributes_are_absent_rather_than_missing() {
    let doc = all_four();
    for (attribute, value) in [
        ("easyshield", "35"),
        ("weight_distribution", "37"),
        ("sideshift", "18"),
    ] {
        let stripped = doc.replace(&format!(r#" {attribute}="{value}""#), "");
        assert_ne!(stripped, doc, "the fixture should carry {attribute}");
        let stats = parse(&stripped)
            .unwrap_or_else(|e| panic!("removing {attribute} should not be an error: {e:?}"));
        let read = match attribute {
            "easyshield" => stats.misc.easyshield,
            "weight_distribution" => stats.misc.weight_distribution,
            _ => {
                stats
                    .class(SpeedClass::VENOM)
                    .expect("four rungs")
                    .airbrake
                    .sideshift
            }
        };
        assert_eq!(read, None, "{attribute} should read as absent");
    }

    // Present but unparseable is still an error.
    let broken = doc.replace(r#"easyshield="35""#, r#"easyshield="oops""#);
    assert!(
        matches!(parse(&broken), Err(Error::NotANumber { .. })),
        "a broken easyshield should not pass as an older schema"
    );
}

/// A rung outside Pulse's four is **kept**, not rejected (this was
/// `Error::UnknownClass`, which made the parser refuse Pure's file: a fifth
/// class below Pulse's slowest). Pulse's four keep their discriminants as
/// indices into the front of the vector.
#[test]
fn a_class_name_outside_pulses_ladder_is_kept_after_the_four() {
    let doc = document(&["VENOM", "FLASH", "RAPIER", "PHANTOM", "SUPERSONIC"]);
    let stats = parse(&doc).expect("a fifth rung is a different ladder, not an error");

    assert_eq!(stats.classes.len(), 5);
    assert!(!stats.has_pulse_class_ladder());
    for class in SpeedClass::ALL {
        assert_eq!(
            stats
                .class(class)
                .expect("Pulse's four are still here")
                .name,
            Some(class),
            "{class} moved when the fifth rung was added"
        );
    }
    assert_eq!(stats.classes[4].name, None);
    assert_eq!(stats.classes[4].raw_name, "SUPERSONIC");
}

/// A file naming *none* of Pulse's ladder is another generation's and is kept
/// whole.
#[test]
fn a_ladder_with_no_pulse_rung_at_all_is_kept_whole() {
    let doc = document(&["ALPHA", "BETA"]);
    let stats = parse(&doc).expect("a wholly different ladder still parses");
    assert_eq!(stats.classes.len(), 2);
    assert!(!stats.has_pulse_class_ladder());
    assert_eq!(stats.class(SpeedClass::VENOM), None);
}

#[test]
fn a_duplicated_class_is_an_error() {
    let doc = document(&["VENOM", "FLASH", "RAPIER", "PHANTOM", "FLASH"]);
    assert_eq!(
        parse(&doc),
        Err(Error::DuplicateClass {
            class: SpeedClass::FLASH
        })
    );
}

#[test]
fn a_non_numeric_attribute_is_an_error() {
    let doc = all_four().replace(r#"mass="26""#, r#"mass="quite heavy""#);
    assert_eq!(
        parse(&doc),
        Err(Error::NotANumber {
            element: "Physical",
            attribute: "mass",
            value: "quite heavy".to_string(),
        })
    );
}

/// `"nan"` and `"inf"` parse as `f32` and would poison every downstream state
/// hash without failing a parse.
#[test]
fn a_non_finite_attribute_is_an_error() {
    for text in ["nan", "NaN", "inf", "-inf", "infinity"] {
        let doc = all_four().replace(r#"mass="26""#, &format!(r#"mass="{text}""#));
        assert!(
            matches!(parse(&doc), Err(Error::NotANumber { .. })),
            "{text} was accepted as a mass"
        );
    }
}

/// `<Values>` carries attributes for its parent, so a block may be written
/// either way round.
#[test]
fn attributes_may_arrive_on_a_values_carrier() {
    let doc = all_four().replace(
        r#"<Brakes amount="6" falloff="7" gain="8"/>"#,
        r#"<Brakes><Values amount="6" falloff="7" gain="8"/></Brakes>"#,
    );
    let stats = parse(&doc).expect("a carrier is equivalent");
    assert_eq!(
        stats
            .class(SpeedClass::VENOM)
            .expect("four rungs")
            .brakes
            .amount,
        6.0
    );
}

/// The files on disc are shortened, so expander and schema must join up. Only
/// the outer three names are shortened here; names absent from a dictionary pass
/// through.
#[test]
fn reads_a_shortened_blob() {
    let doc = all_four()
        .replace("<Handling>", "<h>")
        .replace("</Handling>", "</h>")
        .replace("<Stats ", "<s ")
        .replace("</Stats>", "</s>")
        .replace("<Class ", "<c ")
        .replace("</Class>", "</c>");
    let blob = format!(r#"<code hs="Handling" ss="Stats" cs="Class"></code>{doc}"#);

    assert_eq!(
        from_blob(blob.as_bytes()).expect("expands and parses"),
        parse(&all_four()).expect("well-formed fixture")
    );
}

/// PS2 ships plain text beginning `<?xml`, so a blob with no `<code>` dictionary
/// is the other platform, not an error.
#[test]
fn reads_a_plain_unshortened_document() {
    let plain = format!("<?xml version=\"1.0\"?>{}", all_four());
    assert_eq!(
        from_blob(plain.as_bytes()).expect("plain XML needs no expansion"),
        parse(&all_four()).expect("well-formed fixture")
    );
}

#[test]
fn rejects_a_blob_that_is_not_text() {
    assert_eq!(
        from_blob(&[0xff, 0xfe, 0xff]),
        Err(Error::Expand(fexml::Error::NotText))
    );
}

#[test]
fn a_document_with_no_handling_element_is_an_error() {
    assert_eq!(
        parse("<Screen name=\"Top\"></Screen>"),
        Err(Error::MissingElement {
            element: "Handling"
        })
    );
    assert_eq!(
        parse("<Handling></Handling>"),
        Err(Error::MissingElement { element: "Stats" })
    );
}

#[test]
fn speed_class_names_round_trip_and_index_their_own_slot() {
    for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
        assert_eq!(SpeedClass::census(class.as_str()), Some(class));
        assert_eq!(class.index(), index, "{class} is not in slot {index}");
    }
    assert_eq!(SpeedClass::census("venom"), Some(SpeedClass::VENOM));
    assert_eq!(SpeedClass::census("SUPERSONIC"), None);
}

#[test]
fn entry_names_are_built_the_way_the_loader_builds_them() {
    assert_eq!(entry_name("Feisar"), r"Data\Ships\Feisar\handlingstats.xml");
}

/// A `<Global>` document with `classes` as its `<GlobalClass>` names, in order.
/// Invented numbers; the `<SpeedupPads>` attributes count from the block's
/// position so slots can be told apart.
fn global_document(classes: &[&str]) -> String {
    let blocks: String = classes
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let n = i + 1;
            format!(
                r#"<GlobalClass name="{name}"><SpeedupPads amount="{n}" time="{}"/>"#,
                n * 10
            ) + &format!(r#"<GravityMul airborne="{}"/>"#, n * 100)
                + &format!(
                    r#"<WeaponPad refresh_time="{}" elimination_refresh_time="{}"/></GlobalClass>"#,
                    n * 1000,
                    n * 10000
                )
        })
        .collect();
    format!(
        r#"<Handling><Global><Zone start="1" increment="2" recharge="3"/>{SPECIAL}{blocks}</Global></Handling>"#
    )
}

/// [`global_document`] with a `<StartBoost>` appended; distinct numbers so a
/// swapped attribute shows.
fn global_with_start_boost(element: &str) -> String {
    global_document(&FOUR).replace("</Global>", &format!("{element}</Global>"))
}

#[test]
fn start_boost_reads_its_seven_attributes_into_their_own_fields() {
    let global = parse_global(&global_with_start_boost(
        r#"<StartBoost windowStart="1" windowEnd="2" stallEnd="3" overallDuration="4" stallMul="5" normalMul="6" boostMul="7"/>"#,
    ))
    .expect("parses")
    .expect("has a <Global>");
    assert_eq!(
        global.start_boost,
        Some(StartBoost {
            window_start: 1.0,
            window_end: 2.0,
            stall_end: 3.0,
            overall_duration: 4.0,
            stall_mul: 5.0,
            normal_mul: 6.0,
            boost_mul: 7.0,
        })
    );
}

#[test]
fn a_global_without_start_boost_has_none_and_is_not_an_error() {
    let global = parse_global(&global_document(&FOUR))
        .expect("parses")
        .expect("has a <Global>");
    assert_eq!(global.start_boost, None);
}

#[test]
fn a_start_boost_missing_an_attribute_is_an_error_not_a_zero() {
    assert!(
        parse_global(&global_with_start_boost(
            r#"<StartBoost windowStart="1" windowEnd="2" stallEnd="3" overallDuration="4" stallMul="5" normalMul="6"/>"#
        ))
        .is_err()
    );
}

/// `<Special>` as [`global_document`] authors it: all five attributes, as the
/// disc's file has them; `turbo_jump` is authored, not asserted.
const SPECIAL: &str = r#"<Special roll_cost="4" roll_speed="5" roll_turbotime="6" speedpad_jump="7" turbo_jump="8"/>"#;

const FOUR: [&str; 4] = ["VENOM", "FLASH", "RAPIER", "PHANTOM"];

#[test]
fn a_per_team_file_carries_no_global_block_and_that_is_not_an_error() {
    assert_eq!(parse_global(&all_four()), Ok(None));
}

#[test]
fn each_global_class_lands_in_its_own_slot() {
    let global = parse_global(&global_document(&FOUR))
        .expect("parses")
        .expect("has a <Global>");
    assert_eq!(global.zone.start, 1.0);
    // The four `<Special>` attributes with a consumer, each distinct.
    assert_eq!(global.special.roll_cost, 4.0);
    assert_eq!(global.special.roll_speed, 5.0);
    assert_eq!(global.special.roll_turbotime, 6.0);
    assert_eq!(global.special.speedpad_jump, 7.0);
    for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
        let n = (index + 1) as f32;
        assert_eq!(
            global.speedup_pads(class),
            SpeedupPads {
                amount: n,
                time: n * 10.0
            }
        );
        // Same block, so pads from one `<GlobalClass>` and gravity from another would show.
        assert_eq!(
            global.gravity_mul(class),
            GravityMul {
                airborne: n * 100.0
            }
        );
        assert_eq!(
            global.weapon_pads(class),
            WeaponPad {
                refresh_time: n * 1000.0,
                elimination_refresh_time: Some(n * 10000.0),
            }
        );
    }
}

/// Pure authors `<WeaponPad refresh_time>` and no `elimination_refresh_time`
/// (no Eliminator; `crates/pure/tests/handling_schema_ground_truth.rs`), so the
/// second attribute is absent, not zero.
#[test]
fn a_weapon_pad_without_an_elimination_time_parses_as_absent_not_zero() {
    let pure_shaped = global_document(&FOUR).replace(r#" elimination_refresh_time="10000""#, "");
    let global = parse_global(&pure_shaped)
        .expect("parses")
        .expect("has a <Global>");
    assert_eq!(
        global.weapon_pads(SpeedClass::VENOM),
        WeaponPad {
            refresh_time: 1000.0,
            elimination_refresh_time: None,
        }
    );
    // The other three still carry theirs: this is about the attribute, not the element.
    assert_eq!(
        global
            .weapon_pads(SpeedClass::FLASH)
            .elimination_refresh_time,
        Some(20000.0)
    );
}

/// Both shipped discs author a fifth `<GlobalClass name="VECTOR">` **first**; it
/// must neither error nor shift the four after it (position indexing would put
/// every class one slot out, wrong by a plausible amount).
#[test]
fn an_unrecognised_global_class_is_skipped_without_shifting_the_others() {
    let with_vector = ["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"];
    let shifted = parse_global(&global_document(&with_vector))
        .expect("parses")
        .expect("has a <Global>");
    assert_eq!(shifted.speedup_pads(SpeedClass::VENOM).amount, 2.0);
    assert_eq!(shifted.speedup_pads(SpeedClass::PHANTOM).amount, 5.0);

    // The same four names without it keep the same *relative* order.
    let without = parse_global(&global_document(&FOUR))
        .expect("parses")
        .expect("has a <Global>");
    assert_eq!(without.speedup_pads(SpeedClass::VENOM).amount, 1.0);
    assert_eq!(without.speedup_pads(SpeedClass::PHANTOM).amount, 4.0);
}

#[test]
fn a_global_block_missing_a_speed_class_is_an_error() {
    assert_eq!(
        parse_global(&global_document(&["VENOM", "FLASH", "RAPIER"])),
        Err(Error::MissingGlobalClass {
            class: SpeedClass::PHANTOM
        })
    );
    assert_eq!(
        parse_global(&global_document(&[
            "VENOM", "VENOM", "FLASH", "RAPIER", "PHANTOM"
        ])),
        Err(Error::DuplicateGlobalClass {
            class: SpeedClass::VENOM
        })
    );
}

/// `<Global>` present but incomplete is an error, not `Ok(None)`: half a
/// configuration would silently lose the boost on whichever class lost its block.
#[test]
fn a_global_block_missing_its_zone_or_its_pads_is_an_error() {
    let no_zone =
        global_document(&FOUR).replace(r#"<Zone start="1" increment="2" recharge="3"/>"#, "");
    assert_eq!(
        parse_global(&no_zone),
        Err(Error::MissingElement { element: "Zone" })
    );

    let no_special = global_document(&FOUR).replace(SPECIAL, "");
    assert_eq!(
        parse_global(&no_special),
        Err(Error::MissingElement { element: "Special" })
    );

    let no_pads = global_document(&FOUR).replace(r#"<SpeedupPads amount="1" time="10"/>"#, "");
    assert_eq!(
        parse_global(&no_pads),
        Err(Error::MissingElement {
            element: "SpeedupPads"
        })
    );

    let no_gravity = global_document(&FOUR).replace(r#"<GravityMul airborne="100"/>"#, "");
    assert_eq!(
        parse_global(&no_gravity),
        Err(Error::MissingElement {
            element: "GravityMul"
        })
    );

    let no_weapon_pad = global_document(&FOUR).replace(
        r#"<WeaponPad refresh_time="1000" elimination_refresh_time="10000"/>"#,
        "",
    );
    assert_eq!(
        parse_global(&no_weapon_pad),
        Err(Error::MissingElement {
            element: "WeaponPad"
        })
    );
}

#[test]
fn a_speed_class_is_its_authored_name_and_orders_slowest_first() {
    for class in SpeedClass::KNOWN {
        assert_eq!(SpeedClass::named(class.as_str()), Some(class));
        assert_eq!(
            SpeedClass::named(&class.as_str().to_lowercase()),
            Some(class)
        );
    }
    assert!(SpeedClass::KNOWN.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(SpeedClass::named("nonsense"), None);
}

#[test]
fn the_census_four_keep_their_slots_and_the_rungs_around_them_have_none() {
    for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
        assert_eq!(class.slot(), Some(index));
        assert_eq!(SpeedClass::census(class.as_str()), Some(class));
    }
    for outside in [SpeedClass::VECTOR, SpeedClass::SUPER_PHANTOM] {
        assert_eq!(outside.slot(), None);
        assert_eq!(SpeedClass::census(outside.as_str()), None);
        assert!(SpeedClass::named(outside.as_str()).is_some());
    }
}

/// HD's file writes the restitution as a C literal, `"0.80f"`, and the original reads `0.8`
/// (`docs/ghidra/functions/ps3-hdfury-eu/wall-response.md`): a strict parse would refuse
/// the whole `<Global>` block over the suffix.
#[test]
fn collision_reads_the_wall_restitution_through_its_trailing_f() {
    let global = parse_global(&global_with_start_boost(
        r#"<Collision weaponHitMassFactor="0.9" shipShipCofR="1.0f" shipWallCofR="0.80f"/>"#,
    ))
    .expect("parses")
    .expect("has a <Global>");
    assert_eq!(
        global.collision,
        Some(Collision {
            ship_wall_cof_r: 0.8
        })
    );
}

#[test]
fn a_global_without_collision_has_none() {
    let global = parse_global(&global_document(&FOUR))
        .expect("parses")
        .expect("has a <Global>");
    assert_eq!(global.collision, None);
}

#[test]
fn a_collision_restitution_that_is_not_a_number_is_an_error() {
    assert!(
        parse_global(&global_with_start_boost(
            r#"<Collision shipWallCofR="0.8ff"/>"#
        ))
        .is_err()
    );
}

/// A classless file (the Zone files' shape) reads as one class under the asked name, with the
/// caller's numbers for the `<Antigrav>` attributes it leaves out, and is refused by the
/// per-class reader's strictness nowhere else.
#[test]
fn a_classless_file_reads_as_the_one_class_with_the_callers_unauthored_antigrav() {
    let doc = r#"<?xml version="1.0"?>
<Handling><Stats team="ZoneMode">
<InternalCamera fov="65" height="0" length="3" pitch="0"/>
<BackwardCamera fov="65" height="0" length="3" pitch="0"/>
<BonnetCamera fov="70" height="2.0" length="0" pitch="0"/>
<ExternalCameraFar fov="60" lookat_height="0" lookat_length="25" pos_height="4.0" pos_length="-13.5" spring_horiz="10" spring_vert="10"/>
<ExternalCameraClose fov="60" lookat_height="-4" lookat_length="25" pos_height="3.6" pos_length="-10.5" spring_horiz="12" spring_vert="12"/>
<Physical flight_gravity="120" mass="1" normal_gravity="5" track_gravity="100"/>
<Antigrav grip_air="3.0" grip_ground="16.0" rebound="0.4" ride_height="5.5"/>
<AirbrakeGraphics amount="30" down_speed="50" up_speed="500"/>
<Misc height="3.5" length="13" shield="100" width="5.0"/>
<Engine accelcap="20" amount="300" falloff="500" gain="900" turbo="150"/>
<Brakes amount="100" falloff="100" gain="100"/>
<Turning amount="1.8" falloff="1000" gain="500"/>
<Airbrake amount="9" drag="2" falloff="500" gain="800" turn="10" slidegrip="100" sideshift="550"/>
<Pitch pitch_air="0.2" pitch_ground="1.0" pitch_damping="5" antigrav_height_adjust="1.0"/>
</Stats></Handling>"#;
    let unauthored = Unauthored {
        landing_rebound: 0.0,
        rebound_jump_time: 2.0,
    };
    let stats = parse_classless(doc, "VENOM", unauthored).expect("a classless file");
    assert_eq!(stats.classes.len(), 1);
    let class = stats.class_named("VENOM").expect("the one class");
    assert_eq!(class.engine.accelcap, 20.0);
    assert_eq!(class.physical.track_gravity, 100.0);
    assert_eq!(class.antigrav.grip_ground, 16.0);
    assert_eq!(class.antigrav.landing_rebound, 0.0);
    assert_eq!(class.antigrav.rebound_jump_time, 2.0);
    assert_eq!(stats.misc.weight_distribution, None);
    // The per-class reader still finds no class in it, and still requires both attributes in a
    // `<Class>`.
    assert!(parse(doc).expect("the plain reader").classes.is_empty());
}

/// HD's `<Tilt>` lands field by field; a file without one (Pulse, Pure) reads `None`.
#[test]
fn a_tilt_is_read_by_name_and_its_absence_is_none() {
    let doc = all_four();
    assert_eq!(parse(&doc).expect("fixture").tilt, None);
    let with = doc.replacen(
        "<Misc ",
        r#"<Tilt internalSpeed="7" internalAmount="1.4" shipSpeed="6" shipAmount="1.3"/><Misc "#,
        1,
    );
    assert_ne!(with, doc, "the fixture has a <Misc> to put it beside");
    assert_eq!(
        parse(&with).expect("a file with a Tilt").tilt,
        Some(Tilt {
            internal_speed: 7.0,
            internal_amount: 1.4,
            ship_speed: 6.0,
            ship_amount: 1.3,
        })
    );
}
