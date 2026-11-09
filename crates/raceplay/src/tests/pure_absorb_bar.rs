//! Pure's absorb burst draws as the PSP engine's pool bar, wide and soft, where every other
//! source's class-6 streak without a recovered draw stays the procedural 1 to 2 px line. Its two
//! sprite templates, `ring1` and `glow`, stay muted on Pure until the burst's spawn pattern is read,
//! and play when an effect is loaded with [`Kept::templates`]
//! (`docs/rendering/weapon-absorb-pairs.md`, "Pure: the burst's streak draw").

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::psys::streak::{Kept, KeptBar, StreakDraw};
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
    assert!(oag_pure::TITLE.effects.templates.is_empty());
    assert!(oag_pure::TITLE.effects.names().contains(&ABSORB_EFFECT));
}

/// How the burst is loaded: as Pure's own table loads it, with nothing kept, or with the templates kept.
#[derive(Clone, Copy)]
enum Load {
    Table,
    Plain,
    WithTemplates,
}

fn absorb_burst(title: &Title, load: Load) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("pure-psp-usa.chd")?;
    let mut archives = oag_pure::open(&image.display().to_string()).expect("Pure opens");
    let dir = title.race.effect_dir_for("");
    let kept = |kept| {
        particle_effect(
            &mut archives_for(&image),
            dir,
            ABSORB_EFFECT,
            false,
            false,
            false,
            kept,
        )
    };
    let (effect, _) = match load {
        Load::Table => {
            listed_particle_effect(&mut archives, dir, title.effects, ABSORB_EFFECT, false)
        }
        Load::Plain => kept(Kept::NONE),
        Load::WithTemplates => kept(Kept {
            bar: KeptBar::Mean,
            templates: true,
        }),
    }
    .expect("the burst loads");
    Some(effect)
}

fn archives_for(image: &std::path::Path) -> oag_assets::Archives {
    oag_pure::open(&image.display().to_string()).expect("Pure opens")
}

/// `WO_WEAPON_ABSORB.POB` on Pure holds the streak emitter and two template billboards, `ring1` and `glow`.
/// Loaded by Pure's table the streak is the pool bar with its own `res+0x4c8` aspect (2.0) and its embedded
/// sprite placed, and the templates stay muted; dropping the table's entry puts the thin procedural line back;
/// asking for the templates keeps their emission, their roll and their sprite, which is what a title listing
/// them in `Effects::templates` gets.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pure_loads_its_absorb_burst_as_the_pool_bar_and_can_play_its_templates() {
    let Some(table) = absorb_burst(oag_pure::TITLE, Load::Table) else {
        return;
    };
    let names: Vec<&str> = table.emitters.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, [ABSORB_EFFECT, "ring1", "glow"]);
    let streak = &table.emitters[0];
    assert_eq!(streak.streak, StreakDraw::Bar { aspect: 2.0 });
    assert!(streak.sprite.is_some(), "the file's own sprite is read");
    for template in &table.emitters[1..] {
        assert_eq!(template.per_emission, (0, 0), "{} is muted", template.name);
    }

    let plain = absorb_burst(oag_pure::TITLE, Load::Plain).expect("same image");
    assert!(
        plain
            .emitters
            .iter()
            .all(|e| e.streak == StreakDraw::Procedural)
    );

    let played = absorb_burst(oag_pure::TITLE, Load::WithTemplates).expect("same image");
    assert_eq!(played.emitters[0].streak, StreakDraw::Bar { aspect: 2.0 });
    for template in &played.emitters[1..] {
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

    // Played, the templates are what is drawn on the first tick besides the streak's own spawn.
    let alive = |effect: &oag_fx::psys::Effect| {
        let mut system = System::new();
        system.ignite(effect, Vec3::ZERO, 1.0);
        system.advance(effect, 1.0 / TICK_HZ, Vec3::ZERO, Vec3::Y, &mut Rng::new(1));
        system.alive_count()
    };
    assert_eq!(alive(&played), alive(&table) + 2);
}
