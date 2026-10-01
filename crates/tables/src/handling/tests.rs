//! What the `handlingstats.xml` reader in [`super`] is asserted to do: the
//! scaled tunables, the per-class blocks, and the shapes a malformed file
//! must not produce.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `handling.rs`: the tests are 672 lines, well past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

/// Every number in these fixtures is **invented**: a plain 1, 2, 3 ... in
/// document order. Per
/// `docs/architecture/adr/0006-no-copyrighted-content.md` no shipped tuning
/// value may appear anywhere in this repository, tests included, and
/// counting from one is about as obviously synthetic as data gets.
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

/// Attributes the fixture carries, which is also the count the schema lists:
/// 41 in the header plus `team`, and 32 per class plus its `name`.
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

/// Every ` name="value"` span in `doc`, so a test can remove them one at a
/// time. Crude on purpose: the fixture is the only input it ever sees.
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
    let phantom = stats.class(SpeedClass::Phantom).expect("four rungs");
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

/// The invariant that matters most. A silently absent attribute defaulting
/// to zero would be a physics bug that reads as a tuning problem, so every
/// one of them is removed in turn and every removal must be a typed error.
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
        // Attributes whose absence is a fact about a *generation* of the
        // schema rather than a defect in a file, so they are exempt here and
        // pinned separately: the three Pulse added, by
        // `the_three_pulse_era_attributes_are_absent_rather_than_missing`, and
        // `headtilt`, which Wipeout HD's team files drop and its own mode
        // ships keep.
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
        // **`FE` and `pitch` are deliberately absent from this list.** Pure
        // omits both from every file on both pressings, so their absence is
        // a schema difference rather than a defect, and `Stats::fe` and
        // `Class::pitch` are `Option` for it. Everything else here is still
        // required, which is what keeps this test meaningful rather than a
        // formality - and the two optional ones get their own test below,
        // because "may be absent" must not quietly become "may be
        // malformed".
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
/// The same pair of claims as the `<pitch>` test below, for the same reason
/// and one schema generation later. **Wipeout HD's team files omit the
/// attribute** - `<InternalCamera fov="65" height="0" length="3" pitch="0"/>`
/// on Feisar - where its own Zone and Detonator ships still carry it, so the
/// absence is a fact about a document rather than a defect in one. Nothing in
/// this project applies `headtilt` (see `oag_render::camera::internal`), so
/// what an `Option` buys here is only that a typo cannot become a silent zero.
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

/// Dropping `<pitch>` gives `None`; breaking one still fails.
///
/// The pair of claims `Class::pitch`'s `Option` makes, and they have to be
/// tested together: an `Option` that swallowed a parse error would turn a
/// typo in a Pulse file into a silent fall-through to
/// `oag_gameplay::handling::PITCH_STAND_IN` - another title's tuning, applied
/// because of a typo, with nothing on screen to say so.
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

    // And an attribute *dropped* from a present block is still an error too,
    // which is the shape a half-written element would take.
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
/// A file that names *some* of Pulse's ladder must name all of it.
///
/// Narrowed rather than relaxed: the check now applies only where the file
/// has shown it is using Pulse's ladder, so a partial Pulse file is still an
/// error while a different generation's ladder is not.
fn a_partly_present_pulse_ladder_is_an_error() {
    assert_eq!(
        parse(&document(&["VENOM", "FLASH", "RAPIER"])),
        Err(Error::MissingClass {
            class: SpeedClass::Phantom
        })
    );
    assert_eq!(
        parse(&document(&["FLASH", "RAPIER", "PHANTOM"])),
        Err(Error::MissingClass {
            class: SpeedClass::Venom
        })
    );
    assert_eq!(
        parse(&document(&["VENOM"])),
        Err(Error::MissingClass {
            class: SpeedClass::Flash
        })
    );
}

/// The three attributes Pulse added read as `None` when absent, and the
/// exemption goes no further than those three.
///
/// `None` is "this schema predates the field", which is why it is not an
/// error - and why a *present but broken* value still is. Collapsing those
/// two would let a typo pass as an older file.
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
                    .class(SpeedClass::Venom)
                    .expect("four rungs")
                    .airbrake
                    .sideshift
            }
        };
        assert_eq!(read, None, "{attribute} should read as absent");
    }

    // Present but unparseable is still an error, for the same three.
    let broken = doc.replace(r#"easyshield="35""#, r#"easyshield="oops""#);
    assert!(
        matches!(parse(&broken), Err(Error::NotANumber { .. })),
        "a broken easyshield should not pass as an older schema"
    );
}

/// A rung outside Pulse's four is **kept**, not rejected.
///
/// This used to be `Error::UnknownClass`, and that strictness was what made
/// this parser refuse Pure's `handlingstats.xml` outright: Pure ships a
/// fifth speed class below Pulse's slowest. An unrecognised rung is a
/// different ladder, not a corrupt file.
///
/// What is still guaranteed: Pulse's four keep their `SpeedClass`
/// discriminants as indices into the front of the vector, so the extra rung
/// cannot displace them.
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

/// A file naming *none* of Pulse's ladder is another generation's, and is
/// kept whole rather than reported as four missing classes.
#[test]
fn a_ladder_with_no_pulse_rung_at_all_is_kept_whole() {
    let doc = document(&["ALPHA", "BETA"]);
    let stats = parse(&doc).expect("a wholly different ladder still parses");
    assert_eq!(stats.classes.len(), 2);
    assert!(!stats.has_pulse_class_ladder());
    assert_eq!(stats.class(SpeedClass::Venom), None);
}

#[test]
fn a_duplicated_class_is_an_error() {
    let doc = document(&["VENOM", "FLASH", "RAPIER", "PHANTOM", "FLASH"]);
    assert_eq!(
        parse(&doc),
        Err(Error::DuplicateClass {
            class: SpeedClass::Flash
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

/// `"nan"` and `"inf"` parse fine as `f32`, and either one would poison every
/// state hash downstream of it without ever failing a parse.
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

/// `<Values>` is an attribute carrier for its parent throughout this format,
/// so a block may be written either way round.
#[test]
fn attributes_may_arrive_on_a_values_carrier() {
    let doc = all_four().replace(
        r#"<Brakes amount="6" falloff="7" gain="8"/>"#,
        r#"<Brakes><Values amount="6" falloff="7" gain="8"/></Brakes>"#,
    );
    let stats = parse(&doc).expect("a carrier is equivalent");
    assert_eq!(
        stats
            .class(SpeedClass::Venom)
            .expect("four rungs")
            .brakes
            .amount,
        6.0
    );
}

/// The files on disc are shortened, so the expander and the schema have to
/// join up. Only the outer three element names are shortened here; names
/// absent from a dictionary pass through untouched, which is what keeps the
/// fixture readable.
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

/// The PS2 release ships this file as plain text beginning `<?xml`, so a blob
/// with no `<code>` dictionary is not an error: it is the other platform.
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
        assert_eq!(SpeedClass::from_name(class.as_str()), Some(class));
        assert_eq!(class as usize, index, "{class} is not in slot {index}");
    }
    assert_eq!(SpeedClass::from_name("venom"), Some(SpeedClass::Venom));
    assert_eq!(SpeedClass::from_name("SUPERSONIC"), None);
}

#[test]
fn entry_names_are_built_the_way_the_loader_builds_them() {
    assert_eq!(entry_name("Feisar"), r"Data\Ships\Feisar\handlingstats.xml");
}

/// A `<Global>` document with `classes` as its `<GlobalClass>` names, in the
/// order given. Every number is invented and none is the game's; the two
/// `<SpeedupPads>` attributes count from the block's position so a test can
/// tell the slots apart.
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

/// [`global_document`] with a `<StartBoost>` appended to `<Global>`. Invented
/// numbers, each distinct so a swapped attribute shows.
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

/// `<Special>` as [`global_document`] authors it. All five attributes, so
/// the fixture is the shape the disc's own file is rather than only the four
/// [`Special`] reads; `turbo_jump` is authored but not asserted on.
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
    // The four `<Special>` attributes with a consumer, read off the same
    // `<Global>` as the rest and each distinct from every other number here.
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
        // Read from the same block, so a reader that took the pads from one
        // `<GlobalClass>` and the gravity from another would show up here.
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

/// Pure authors `<WeaponPad refresh_time>` and no
/// `elimination_refresh_time`, having no Eliminator - see
/// `crates/pure/tests/handling_schema_ground_truth.rs`. So the second
/// attribute is absent rather than zero, and the distinction has to survive
/// the parse.
#[test]
fn a_weapon_pad_without_an_elimination_time_parses_as_absent_not_zero() {
    let pure_shaped = global_document(&FOUR).replace(r#" elimination_refresh_time="10000""#, "");
    let global = parse_global(&pure_shaped)
        .expect("parses")
        .expect("has a <Global>");
    assert_eq!(
        global.weapon_pads(SpeedClass::Venom),
        WeaponPad {
            refresh_time: 1000.0,
            elimination_refresh_time: None,
        }
    );
    // The other three still carry theirs, so this is about the attribute
    // rather than about the element.
    assert_eq!(
        global
            .weapon_pads(SpeedClass::Flash)
            .elimination_refresh_time,
        Some(20000.0)
    );
}

/// The finding this reader is shaped around: both shipped discs author a
/// fifth `<GlobalClass name="VECTOR">` **first**, and it must neither be an
/// error nor shift the four that follow into the wrong slots. Position
/// indexing would put every class one slot out and produce a boost that is
/// wrong by a plausible-looking amount on all four.
#[test]
fn an_unrecognised_global_class_is_skipped_without_shifting_the_others() {
    let with_vector = ["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"];
    let shifted = parse_global(&global_document(&with_vector))
        .expect("parses")
        .expect("has a <Global>");
    assert_eq!(shifted.speedup_pads(SpeedClass::Venom).amount, 2.0);
    assert_eq!(shifted.speedup_pads(SpeedClass::Phantom).amount, 5.0);

    // And the same four names without it keep the same *relative* order, so
    // the assertion above is about the skip rather than about the numbering.
    let without = parse_global(&global_document(&FOUR))
        .expect("parses")
        .expect("has a <Global>");
    assert_eq!(without.speedup_pads(SpeedClass::Venom).amount, 1.0);
    assert_eq!(without.speedup_pads(SpeedClass::Phantom).amount, 4.0);
}

#[test]
fn a_global_block_missing_a_speed_class_is_an_error() {
    assert_eq!(
        parse_global(&global_document(&["VENOM", "FLASH", "RAPIER"])),
        Err(Error::MissingGlobalClass {
            class: SpeedClass::Phantom
        })
    );
    assert_eq!(
        parse_global(&global_document(&[
            "VENOM", "VENOM", "FLASH", "RAPIER", "PHANTOM"
        ])),
        Err(Error::DuplicateGlobalClass {
            class: SpeedClass::Venom
        })
    );
}

/// `<Global>` present but incomplete is an error rather than `Ok(None)`,
/// which is the tightening this reader makes over the `<Zone>`-only one it
/// replaced. Half a configuration is worse than none: the boost would be
/// silently absent on whichever class lost its block.
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
