//! Pure's absorb burst draws as the PSP engine's pool bar, wide and soft, where every other
//! source's class-6 streak without a recovered draw stays the procedural 1 to 2 px line, and its
//! two sprite templates, `ring1` and `glow`, play where every other source mutes them
//! (`docs/rendering/weapon-absorb-pairs.md`, "Pure: the burst's streak draw").

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::psys::streak::{Kept, StreakDraw};
use oag_fx::psys::{System, TICK_HZ};
use oag_title::Title;
use oag_title::engine_effects::ABSORB_EFFECT;

use crate::assets::{listed_particle_effect, particle_effect};

#[test]
fn only_pure_lists_effects_for_the_pool_bar_and_only_the_absorb_burst() {
    for title in [
        oag_pulse::TITLE,
        oag_hd::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ] {
        assert!(title.effects.pool_bars.is_empty(), "{}", title.name);
        assert!(title.effects.templates.is_empty(), "{}", title.name);
    }
    assert_eq!(oag_pure::TITLE.effects.pool_bars, [ABSORB_EFFECT]);
    assert_eq!(oag_pure::TITLE.effects.templates, [ABSORB_EFFECT]);
    assert!(oag_pure::TITLE.effects.names().contains(&ABSORB_EFFECT));
}

fn absorb_burst(title: &Title, listed: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("pure-psp-usa.chd")?;
    let mut archives = oag_pure::open(&image.display().to_string()).expect("Pure opens");
    let dir = title.race.effect_dir_for("");
    let (effect, _) = if listed {
        listed_particle_effect(&mut archives, dir, title.effects, ABSORB_EFFECT, false)
    } else {
        particle_effect(
            &mut archives,
            dir,
            ABSORB_EFFECT,
            false,
            false,
            false,
            Kept::NONE,
        )
    }
    .expect("the burst loads");
    Some(effect)
}

/// `WO_WEAPON_ABSORB.POB` on Pure holds the streak emitter and two template billboards, `ring1` and `glow`.
/// Listed, the streak loads as the pool bar with its own `res+0x4c8` aspect (2.0) and its embedded sprite placed,
/// and the templates keep their emission and their roll; dropping the list puts the thin procedural line back
/// and mutes the templates.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pure_loads_its_absorb_burst_as_the_pool_bar_and_plays_its_templates() {
    let Some(listed) = absorb_burst(oag_pure::TITLE, true) else {
        return;
    };
    let names: Vec<&str> = listed.emitters.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, [ABSORB_EFFECT, "ring1", "glow"]);
    let streak = &listed.emitters[0];
    assert_eq!(streak.streak, StreakDraw::Bar { aspect: 2.0 });
    assert!(streak.sprite.is_some(), "the file's own sprite is read");
    for template in &listed.emitters[1..] {
        assert!(template.template, "{}", template.name);
        assert_ne!(template.per_emission, (0, 0), "{} emits", template.name);
        assert!(template.rotation.is_some(), "{} rolls", template.name);
        assert!(
            template.sprite.is_some(),
            "{} has its own sprite",
            template.name
        );
        assert_eq!(template.streak, StreakDraw::Procedural, "{}", template.name);
    }

    let unlisted = absorb_burst(oag_pure::TITLE, false).expect("same image");
    assert!(
        unlisted
            .emitters
            .iter()
            .all(|e| e.streak == StreakDraw::Procedural)
    );
    for template in &unlisted.emitters[1..] {
        assert_eq!(template.per_emission, (0, 0), "{} is muted", template.name);
    }

    // Played, the templates are what is drawn on the first tick besides the streak's own spawn.
    let alive = |effect: &oag_fx::psys::Effect| {
        let mut system = System::new();
        system.ignite(effect, Vec3::ZERO, 1.0);
        system.advance(effect, 1.0 / TICK_HZ, Vec3::ZERO, Vec3::Y, &mut Rng::new(1));
        system.alive_count()
    };
    assert_eq!(alive(&listed), alive(&unlisted) + 2);
}
