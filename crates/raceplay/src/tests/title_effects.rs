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
const OWN: [Trigger; 10] = [
    Trigger::BombRays,
    Trigger::HitSpark,
    Trigger::LeachHitSpark,
    Trigger::WeaponSpark,
    Trigger::ShieldAbsorb,
    Trigger::WreckNode,
    Trigger::WreckSparks,
    Trigger::DamageMild,
    Trigger::DamageModerate,
    Trigger::DamageCritical,
];

/// The engine triggers whose effect neither 2048 nor Omega authors
/// (`docs/formats/pob.md`, "Effects Pulse names that 2048 and Omega never
/// authors"); Omega drops the two magstrip ones besides.
const UNAUTHORED: [Trigger; 3] = [
    Trigger::PlasmaBlast,
    Trigger::EngineFlare,
    Trigger::LeachbeamEnergy,
];

#[test]
fn the_vita_and_ps4_titles_drop_what_they_never_author_and_the_rest_keep_it() {
    let has = |trigger| ALL.map(|t| t.effect_on(trigger).is_some());
    for trigger in UNAUTHORED {
        // Pure has no LeachBeam at all (`PURE_ABSENT`), so it names none of the
        // beam's effects, the energy one included.
        let pure = !PURE_ABSENT.contains(&trigger);
        // HD drops the PS2 engine-flare particle too (`HD_ABSENT`).
        let hd = !HD_ABSENT.contains(&trigger);
        let pulse = !PULSE_ABSENT.contains(&trigger);
        assert_eq!(has(trigger), [pulse, pure, hd, false, false], "{trigger:?}");
    }
    for trigger in [Trigger::MagstripSparks, Trigger::MagstripZone] {
        // Pulse reads neither (`PULSE_ABSENT`), nor HD (`HD_ABSENT`), nor Pure
        // (`PURE_ABSENT`); only Omega drops them here.
        assert_eq!(
            has(trigger),
            [false, false, false, true, false],
            "{trigger:?}"
        );
    }
    for title in [oag_2048::TITLE, oag_omega::TITLE] {
        let names = title.effects.names();
        for gone in ["WO_BLUE_WELDER", "WO_RAIN", "WO_RAIN_LENS", "WO_SNOW"] {
            assert!(!names.contains(&gone), "{} still loads {gone}", title.name);
        }
        assert!(names.contains(&"WO_MODESTO_STEAM_A"), "{}", title.name);
    }
}

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
        [true, false, true, false, false]
    );
    assert_eq!(
        has(Trigger::WeaponSpark),
        [false, false, true, false, false]
    );
    assert_eq!(has(Trigger::ShieldAbsorb), [true, true, true, false, false]);
    // `ShipDamageFx_Update_q` is read on HD alone; 2048 and Omega are
    // "checked, applies, not wired" (their trigger unread).
    for damage in [
        Trigger::DamageMild,
        Trigger::DamageModerate,
        Trigger::DamageCritical,
    ] {
        assert_eq!(
            has(damage),
            [false, false, true, false, false],
            "{damage:?}"
        );
    }
    for wreck in [
        Trigger::WreckNode,
        Trigger::WreckSparks,
        Trigger::WreckExplosion,
    ] {
        assert_eq!(has(wreck), [true, false, false, false, false], "{wreck:?}");
    }
    // `NormalBombBlast_Update` (HD, `0x001503d8`) spawns it; no other title's
    // blast does.
    assert_eq!(has(Trigger::BombRays), [false, false, true, false, false]);
}

/// The engine-table triggers whose effect no Pulse archive holds and no Pulse
/// executable requests (`docs/formats/pulse-absent-effects.md`).
const PULSE_ABSENT: [Trigger; 8] = [
    Trigger::PlasmaLightningExpand,
    Trigger::PlasmaLightningCollapse,
    Trigger::TrailHitship,
    Trigger::TrailHitshipRed,
    Trigger::LeachbeamAbsorb,
    Trigger::MagstripSparks,
    Trigger::MagstripZone,
    Trigger::EngineFlare,
];

/// The engine-table effects Pure does not ask for: the LeachBeam's three (it
/// ships no LeachBeam, `docs/ghidra/functions/psp-pure-usa/weapons.md`, "Pure
/// ships no LeachBeam") and the fifteen names its executable's `WO_*` list does
/// not carry (`oag_pure::effects::EFFECTS`), so its table leaves them unread
/// rather than inheriting names for entries no Pure disc carries.
const PURE_ABSENT: [Trigger; 18] = [
    Trigger::LeachbeamEnergy,
    Trigger::LeachbeamCharging,
    Trigger::LeachbeamAbsorb,
    Trigger::PlasmaLightningExpand,
    Trigger::PlasmaLightningCollapse,
    Trigger::ShurikenFlare,
    Trigger::ShurikenTrail,
    Trigger::ShurikenBounce,
    Trigger::ShurikenExpire,
    Trigger::TrackBlast,
    Trigger::CannonSparks,
    Trigger::EngineFlare,
    Trigger::TrailHitship,
    Trigger::TrailHitshipRed,
    Trigger::RepulserBlast,
    Trigger::Repulser,
    Trigger::MagstripSparks,
    Trigger::MagstripZone,
];

/// The 2048-lineage magstrip pair, which no archive of HD's disc carries and
/// which HD replaces with the arc wake (`docs/formats/pob.md`, "Which names HD
/// does not author"), and the PS2 engine-flare particle, which the disc ships
/// but nothing plays (`engine-trail.md`, "The flare's draw state, read live").
const HD_ABSENT: [Trigger; 3] = [
    Trigger::MagstripSparks,
    Trigger::MagstripZone,
    Trigger::EngineFlare,
];

#[test]
fn hd_asks_for_none_of_the_effects_its_disc_does_not_author() {
    for trigger in HD_ABSENT {
        assert!(oag_hd::TITLE.effect_on(trigger).is_none(), "{trigger:?}");
        for title in [oag_pure::TITLE, oag_2048::TITLE] {
            if trigger == Trigger::EngineFlare && title.name == oag_2048::TITLE.name {
                continue;
            }
            if std::ptr::eq(title, oag_pure::TITLE) && PURE_ABSENT.contains(&trigger) {
                continue;
            }
            assert!(
                title.effect_on(trigger).is_some(),
                "{} {trigger:?}",
                title.name
            );
        }
    }
}

#[test]
fn pulse_asks_for_none_of_the_effects_its_discs_do_not_author() {
    for trigger in PULSE_ABSENT {
        assert!(
            oag_pulse::TITLE.effect_on(trigger).is_none(),
            "Pulse names {trigger:?}"
        );
        for title in [oag_pure::TITLE, oag_hd::TITLE, oag_2048::TITLE] {
            if title.name == oag_hd::TITLE.name && HD_ABSENT.contains(&trigger) {
                continue;
            }
            if title.name == oag_2048::TITLE.name && trigger == Trigger::EngineFlare {
                continue;
            }
            if title.name == oag_pure::TITLE.name && PURE_ABSENT.contains(&trigger) {
                assert!(title.effect_on(trigger).is_none(), "Pure names {trigger:?}");
                continue;
            }
            assert!(
                title.effect_on(trigger).is_some(),
                "{} {trigger:?}",
                title.name
            );
        }
    }
    // The PS2 disc ships `WO_SHIP_ENGINEFLARE.POB` and its executable never loads it
    // (`engine-trail.md`, "The engine flare is one quad"): no pressing names it.
    for platform in [Platform::Psp, Platform::Ps2] {
        assert!(
            !oag_pulse::TITLE
                .effects
                .names_on(platform)
                .contains(&"WO_SHIP_ENGINEFLARE")
        );
    }
}

#[test]
fn every_title_tries_the_engines_own_effects_as_pulses() {
    for title in ALL {
        // `ALL`'s order: 2048 is index 3 and Omega index 4.
        let at = ALL.iter().position(|t| std::ptr::eq(*t, title)).unwrap();
        for trigger in Trigger::ALL.into_iter().filter(|t| {
            !OWN.contains(t)
                && *t != Trigger::WreckExplosion
                && !(title.name == oag_pulse::TITLE.name && PULSE_ABSENT.contains(t))
                && !(title.name == oag_hd::TITLE.name && HD_ABSENT.contains(t))
                && !(title.name == oag_pure::TITLE.name && PURE_ABSENT.contains(t))
                && !(at >= 3 && UNAUTHORED.contains(t))
                && !(at == 4 && matches!(t, Trigger::MagstripSparks | Trigger::MagstripZone))
        }) {
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
    // 43 since Pure, the last title that named it, dropped `WO_SHIP_ENGINEFLARE`.
    assert_eq!(crate::RACE_EFFECTS.len(), 43);
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
            t.looks.hull_shine.applies(Platform::Psp),
            t.looks.laid_pose.applies_everywhere(),
            t.looks.absorb_shell.applies_everywhere(),
        ]
    };
    assert_eq!(every(oag_pulse::TITLE), [true, true, true, false]);
    assert_eq!(every(oag_hd::TITLE), [false, false, false, true]);
    assert_eq!(every(oag_pure::TITLE), [false, true, false, false]);
    for other in [oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(every(other), [false; 4], "{}", other.name);
    }

    let psp = |t: &Title| t.looks.measured_draws.applies(Platform::Psp);
    let ps2 = |t: &Title| t.looks.ps2_glow_mask.applies(Platform::Ps2);
    let wreck = |t: &Title, p| t.looks.hull_wreck.applies(p);
    assert!(psp(oag_pulse::TITLE) && ps2(oag_pulse::TITLE));
    assert!(!oag_pulse::TITLE.looks.measured_draws.applies(Platform::Ps2));
    assert!(!oag_pulse::TITLE.looks.ps2_glow_mask.applies(Platform::Psp));
    assert!(oag_pulse::TITLE.looks.ps2_trail.applies(Platform::Ps2));
    assert!(!oag_pulse::TITLE.looks.ps2_trail.applies(Platform::Psp));
    for other in [
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ] {
        assert!(
            !other.looks.ps2_trail.applies(Platform::Ps2),
            "{}",
            other.name
        );
    }
    assert!(wreck(oag_pulse::TITLE, Platform::Psp));
    assert!(wreck(oag_pulse::TITLE, Platform::Ps2));
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
fn only_2048_ends_its_race_in_the_wo_composite_and_it_is_measured() {
    let wo = oag_2048::TITLE.looks.wo_composite;
    assert!(wo.applies_everywhere());
    assert_eq!(wo.origin, Origin::Measured);
    for other in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
    ] {
        for platform in [Platform::Psp, Platform::Ps2] {
            assert!(
                !other.looks.wo_composite.applies(platform),
                "{}",
                other.name
            );
        }
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

/// Both PSP titles draw the circuit's chrome-map pass; only Pure weights it by
/// the batch texture's alpha (measured on its GE lists), Pulse's weighting is
/// unmeasured and stays off. No other title draws it.
#[test]
fn the_circuit_chrome_pass_is_pulses_and_pures_and_only_pures_is_weighted() {
    let rules = |t: &Title| {
        (
            t.looks.track_shine.applies(Platform::Psp),
            t.looks.track_shine_weighted.applies(Platform::Psp),
        )
    };
    assert_eq!(rules(oag_pulse::TITLE), (true, false));
    assert_eq!(rules(oag_pure::TITLE), (true, true));
    for other in [oag_hd::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(rules(other), (false, false), "{}", other.name);
    }
}
