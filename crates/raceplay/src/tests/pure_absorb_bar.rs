//! Pure's absorb burst draws as the PSP engine's pool bar, wide and soft, where every other
//! source's class-6 streak without a recovered draw stays the procedural 1 to 2 px line
//! (`docs/rendering/weapon-absorb-pairs.md`, "Pure: the burst's streak draw").

use oag_fx::psys::streak::{KeptBar, StreakDraw};
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
    }
    assert_eq!(oag_pure::TITLE.effects.pool_bars, [ABSORB_EFFECT]);
    assert!(oag_pure::TITLE.effects.names().contains(&ABSORB_EFFECT));
}

fn absorb_burst(title: &Title, pool_bars: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("pure-psp-usa.chd")?;
    let mut archives = oag_pure::open(&image.display().to_string()).expect("Pure opens");
    let dir = title.race.effect_dir_for("");
    let (effect, _) = if pool_bars {
        listed_particle_effect(&mut archives, dir, title.effects, ABSORB_EFFECT, false)
    } else {
        particle_effect(
            &mut archives,
            dir,
            ABSORB_EFFECT,
            false,
            false,
            false,
            KeptBar::No,
        )
    }
    .expect("the burst loads");
    Some(effect)
}

/// `WO_WEAPON_ABSORB.POB` on Pure holds the streak emitter and two template billboards, `ring1` and `glow`.
/// Listed, the streak loads as the pool bar with its own `res+0x4c8` aspect (2.0) and its embedded sprite placed;
/// dropping the list puts the thin procedural line back. The templates stay procedural either way.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pure_loads_its_absorb_burst_as_the_pool_bar() {
    let Some(listed) = absorb_burst(oag_pure::TITLE, true) else {
        return;
    };
    let names: Vec<&str> = listed.emitters.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, [ABSORB_EFFECT, "ring1", "glow"]);
    let streak = &listed.emitters[0];
    assert_eq!(streak.streak, StreakDraw::Bar { aspect: 2.0 });
    assert!(streak.sprite.is_some(), "the file's own sprite is read");
    assert!(
        listed.emitters[1..]
            .iter()
            .all(|e| e.streak == StreakDraw::Procedural)
    );
    let unlisted = absorb_burst(oag_pure::TITLE, false).expect("same image");
    assert!(
        unlisted
            .emitters
            .iter()
            .all(|e| e.streak == StreakDraw::Procedural)
    );
}
