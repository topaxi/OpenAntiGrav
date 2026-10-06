//! Each title's effect and look tables (ADR-0058), against the loader's own
//! names. Dropping an entry from a title package, or wiring a consumer past
//! the table, fails here.

use oag_title::{Burst, Origin, Platform, ShieldPalette, Title, Trigger};

const ALL: [&Title; 5] = [
    oag_pulse::TITLE,
    oag_pure::TITLE,
    oag_hd::TITLE,
    oag_2048::TITLE,
    oag_omega::TITLE,
];

/// The triggers a title answers for itself; the rest are the engine's.
const OWN: [Trigger; 6] = [
    Trigger::HitSpark,
    Trigger::LeachHitSpark,
    Trigger::WeaponSpark,
    Trigger::ShieldAbsorb,
    Trigger::WreckNode,
    Trigger::WreckSparks,
];

#[test]
fn the_trigger_list_is_in_index_order_and_counts_itself() {
    assert_eq!(Trigger::COUNT, Trigger::ALL.len());
    for (i, trigger) in Trigger::ALL.iter().enumerate() {
        assert_eq!(trigger.index(), i, "{trigger:?}");
    }
}

#[test]
fn each_trigger_is_answered_by_exactly_the_titles_that_read_it() {
    let has = |trigger| ALL.map(|t| t.effect_on(trigger).is_some());
    // Pulse, Pure, HD, 2048, Omega.
    assert_eq!(has(Trigger::HitSpark), [true, false, false, false, false]);
    assert_eq!(
        has(Trigger::LeachHitSpark),
        [true, false, false, false, false]
    );
    assert_eq!(
        has(Trigger::WeaponSpark),
        [false, false, true, false, false]
    );
    assert_eq!(has(Trigger::ShieldAbsorb), [true, true, true, false, false]);
    for wreck in [
        Trigger::WreckNode,
        Trigger::WreckSparks,
        Trigger::WreckExplosion,
    ] {
        assert_eq!(has(wreck), [true, false, false, false, false], "{wreck:?}");
    }
}

#[test]
fn every_title_tries_the_engines_own_effects_as_pulses() {
    for title in ALL {
        for trigger in Trigger::ALL
            .into_iter()
            .filter(|t| !OWN.contains(t) && *t != Trigger::WreckExplosion)
        {
            // HD authors neither magstrip `.pob` (it builds the arc wake) and its
            // table takes the inherited pair back - `docs/formats/pob.md`, "Which
            // names HD does not author".
            if std::ptr::eq(title, oag_hd::TITLE)
                && matches!(trigger, Trigger::MagstripSparks | Trigger::MagstripZone)
            {
                assert!(title.effect_on(trigger).is_none(), "{trigger:?}");
                continue;
            }
            let spec = title
                .effect_on(trigger)
                .unwrap_or_else(|| panic!("{} has no {trigger:?}", title.name));
            let want = if title.name == oag_pulse::TITLE.name {
                Origin::Measured
            } else {
                Origin::InheritedFrom("Wipeout Pulse")
            };
            assert_eq!(spec.origin, want, "{} {trigger:?}", title.name);
        }
    }
}

#[test]
fn the_tables_name_the_effects_the_loader_plays() {
    use oag_title::engine_effects as n;
    let name = |title: &Title, trigger| title.effect_on(trigger).map(|spec| spec.effect);
    assert_eq!(
        name(oag_pulse::TITLE, Trigger::HitSpark),
        Some(oag_fx::sparks::DAMAGE_EFFECT)
    );
    assert_eq!(n::COLLISION_SPARK_EFFECT, oag_fx::sparks::DAMAGE_EFFECT);
    assert_eq!(
        n::TRAIL_HITSHIP_EFFECT,
        oag_fx::exhaust::hd::TRAIL_HITSHIP_EFFECT
    );
    assert_eq!(
        n::TRAIL_HITSHIP_RED_EFFECT,
        oag_fx::exhaust::hd::TRAIL_HITSHIP_RED_EFFECT
    );
    assert_eq!(
        name(oag_pulse::TITLE, Trigger::LeachHitSpark),
        Some("WO_SHIP_SPARK_DAMAGE_LEACHBEAM")
    );
    assert_eq!(
        name(oag_hd::TITLE, Trigger::WeaponSpark),
        Some("WO_SHIP_SPARK_DAMAGE_WEAPON")
    );
    for title in [oag_pulse::TITLE, oag_pure::TITLE, oag_hd::TITLE] {
        assert_eq!(
            name(title, Trigger::ShieldAbsorb),
            Some("WO_WEAPON_ABSORB"),
            "{}",
            title.name
        );
    }
    assert_eq!(
        [
            Trigger::WreckNode,
            Trigger::WreckSparks,
            Trigger::WreckExplosion
        ]
        .map(|t| name(oag_pulse::TITLE, t)),
        [
            Some("WO_SHIP_FXNODE_EXPLO"),
            Some("WO_SHIP_DEATH_SPARKS"),
            Some("WO_SHIP_EXPLOSION")
        ]
    );
}

#[test]
fn a_race_loads_each_name_of_its_own_table_once_and_the_superset_covers_every_title() {
    for title in ALL {
        let names = title.effects.names();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "{} repeats a name", title.name);
        for name in &names {
            assert!(
                crate::RACE_EFFECTS.contains(name),
                "{name} is not in RACE_EFFECTS"
            );
        }
    }
    // Pulse's hit spark and the collision spark are one file, loaded once.
    let pulse = oag_pulse::TITLE.effects.names();
    assert_eq!(
        pulse
            .iter()
            .filter(|n| **n == oag_fx::sparks::DAMAGE_EFFECT)
            .count(),
        1
    );
    assert_eq!(crate::RACE_EFFECTS.len(), 39);
}

#[test]
fn a_trigger_resolves_to_the_effect_its_titles_table_names() {
    let mut library = oag_fx::psys::Library::new();
    let name = oag_hd::TITLE
        .effect_on(Trigger::WeaponSpark)
        .map(|spec| spec.effect)
        .expect("HD throws a weapon spark");
    let blob = super::respawn::one_emitter_pob(name, 0);
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    library.insert(name, effect);
    let hd = crate::EffectHandles::resolve(oag_hd::TITLE, &library);
    assert!(hd.get(Trigger::WeaponSpark).is_some());
    assert!(
        Trigger::ALL
            .iter()
            .filter(|t| **t != Trigger::WeaponSpark)
            .all(|t| hd.get(*t).is_none()),
        "only the loaded one resolves"
    );
    // Pulse's table does not name it, so it stays empty there.
    let pulse = crate::EffectHandles::resolve(oag_pulse::TITLE, &library);
    assert!(pulse.get(Trigger::WeaponSpark).is_none());
}

#[test]
fn the_absorb_burst_is_each_titles_own() {
    let burst = |t: &Title| absorb::absorb_burst_for(t);
    use crate::absorb;
    assert_eq!(
        burst(oag_pulse::TITLE),
        Some(Burst::Sequential {
            cap: 10,
            stagger: 0.1
        })
    );
    assert_eq!(
        burst(oag_pure::TITLE),
        Some(Burst::Sequential {
            cap: 8,
            stagger: 0.1
        })
    );
    assert_eq!(
        burst(oag_hd::TITLE),
        Some(Burst::MirroredPairs { stagger: 0.2 })
    );
    assert_eq!(burst(oag_2048::TITLE), None);
    assert_eq!(burst(oag_omega::TITLE), None);
}

#[test]
fn a_burst_staggers_its_locators_the_way_the_original_spawns_them() {
    let pulse = oag_pulse::effects::ABSORB_BURST.schedule(12);
    assert_eq!(pulse.len(), 10);
    assert_eq!(pulse[9], (9, 9.0 * 0.1));
    assert_eq!(oag_pure::effects::ABSORB_BURST.schedule(10).len(), 8);
    let hd = oag_hd::effects::ABSORB_BURST;
    assert_eq!(
        hd.schedule(6),
        vec![(0, 0.0), (5, 0.0), (1, 0.2), (4, 0.2), (2, 0.4), (3, 0.4)]
    );
    assert!(hd.schedule(5).is_empty());
    assert!(oag_pulse::effects::ABSORB_BURST.schedule(0).is_empty());
}

#[test]
fn no_title_labels_a_present_effect_chosen_and_the_others_are_measured() {
    for title in ALL {
        for trigger in OWN {
            if let Some(spec) = title.effect_on(trigger) {
                assert_eq!(spec.origin, Origin::Measured, "{} {trigger:?}", title.name);
            }
        }
    }
}

#[test]
fn the_flags_the_loader_derived_from_a_name_are_the_titles() {
    let every = |t: &Title| {
        [
            t.looks.hull_overlay.applies_everywhere(),
            t.looks.hull_shine.applies_everywhere(),
            t.looks.laid_pose.applies_everywhere(),
            t.looks.absorb_shell.applies_everywhere(),
        ]
    };
    assert_eq!(every(oag_pulse::TITLE), [true, true, true, false]);
    assert_eq!(every(oag_hd::TITLE), [false, false, false, true]);
    for other in [oag_pure::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(every(other), [false; 4], "{}", other.name);
    }

    let psp = |t: &Title| t.looks.measured_draws.applies(Platform::Psp);
    let ps2 = |t: &Title| t.looks.ps2_glow_mask.applies(Platform::Ps2);
    let wreck = |t: &Title, p| t.looks.hull_wreck.applies(p);
    assert!(psp(oag_pulse::TITLE) && ps2(oag_pulse::TITLE));
    assert!(!oag_pulse::TITLE.looks.measured_draws.applies(Platform::Ps2));
    assert!(!oag_pulse::TITLE.looks.ps2_glow_mask.applies(Platform::Psp));
    assert!(wreck(oag_pulse::TITLE, Platform::Psp));
    assert!(!wreck(oag_pulse::TITLE, Platform::Ps2));
    for other in [
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ] {
        assert!(!psp(other) && !ps2(other), "{}", other.name);
        assert!(!wreck(other, Platform::Psp), "{}", other.name);
    }
}

#[test]
fn the_shield_tint_is_hds_own_and_everyone_elses_is_pulses_by_inheritance() {
    let tint = |t: &Title, p| t.looks.shield_palette.on(p);
    assert_eq!(tint(oag_hd::TITLE, Platform::Psp), ShieldPalette::Hd);
    assert_eq!(tint(oag_hd::TITLE, Platform::Ps2), ShieldPalette::Hd);
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ] {
        assert_eq!(tint(title, Platform::Psp), ShieldPalette::Pulse);
        assert_eq!(tint(title, Platform::Ps2), ShieldPalette::Ps2Pulse);
    }
    assert_eq!(
        oag_pure::TITLE.looks.shield_palette.origin,
        Origin::InheritedFrom("Wipeout Pulse")
    );
}
