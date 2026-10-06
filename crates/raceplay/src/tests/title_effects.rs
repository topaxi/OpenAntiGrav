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

fn names(title: &Title, trigger: Trigger) -> Option<&'static [&'static str]> {
    title.effect_on(trigger).map(|spec| spec.effects)
}

#[test]
fn each_trigger_is_answered_by_exactly_the_titles_that_read_it() {
    let has = |trigger| ALL.map(|t| t.effect_on(trigger).is_some());
    // Pulse, Pure, HD, 2048, Omega.
    assert_eq!(has(Trigger::HitSpark), [true, false, false, false, false]);
    assert_eq!(
        has(Trigger::WeaponSpark),
        [false, false, true, false, false]
    );
    assert_eq!(has(Trigger::ShieldAbsorb), [true, true, true, false, false]);
    assert_eq!(has(Trigger::Wreck), [true, false, false, false, false]);
}

#[test]
fn the_tables_name_the_effects_the_loader_plays() {
    use crate::{absorb, hit_sparks, wreck_fx};
    assert_eq!(
        names(oag_pulse::TITLE, Trigger::HitSpark),
        Some(
            &[
                hit_sparks::HIT_SPARK_EFFECT,
                hit_sparks::LEACHBEAM_HIT_SPARK_EFFECT
            ][..]
        )
    );
    assert_eq!(
        names(oag_hd::TITLE, Trigger::WeaponSpark),
        Some(&[crate::effect_names::WEAPON_SPARK_EFFECT][..])
    );
    for title in [oag_pulse::TITLE, oag_pure::TITLE, oag_hd::TITLE] {
        assert_eq!(
            names(title, Trigger::ShieldAbsorb),
            Some(&[absorb::ABSORB_EFFECT][..]),
            "{}",
            title.name
        );
    }
    assert_eq!(
        names(oag_pulse::TITLE, Trigger::Wreck),
        Some(
            &[
                wreck_fx::FXNODE_EXPLO_EFFECT,
                wreck_fx::DEATH_SPARKS_EFFECT,
                wreck_fx::EXPLOSION_EFFECT
            ][..]
        )
    );
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
        for trigger in [
            Trigger::HitSpark,
            Trigger::WeaponSpark,
            Trigger::ShieldAbsorb,
            Trigger::Wreck,
        ] {
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
