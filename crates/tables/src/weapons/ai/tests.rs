//! What [`super`] is asserted to do, against an invented fixture - see
//! `docs/architecture/adr/0006-no-copyrighted-content.md` for why every number
//! here is a plain 1, 2, 3 in document order rather than the disc's own.

use super::*;

const FIXTURE: &str = r#"<WeaponAIStats>
<Rockets useAgainstPlayer="1" useAgainstAI="2" absorb="3"/>
<Missiles useAgainstPlayer="4" useAgainstAI="5" absorb="6"/>
<Quake useAgainstPlayer="7" useAgainstAI="8" absorb="9"/>
<Turbo useAgainstPlayer="10" useAgainstAI="11" absorb="12"/>
<Shield useAgainstPlayer="13" useAgainstAI="14" absorb="15"/>
<Cannon useAgainstPlayer="16" useAgainstAI="17" absorb="18"/>
<Autopilot useAgainstPlayer="19" useAgainstAI="20" absorb="21"/>
<Plasma useAgainstPlayer="22" useAgainstAI="23" absorb="24"/>
<Bomb useAgainstPlayer="25" useAgainstAI="26" absorb="27"/>
<Mines useAgainstPlayer="28" useAgainstAI="29" absorb="30"/>
<Leachbeam useAgainstPlayer="31" useAgainstAI="32" absorb="33"/>
<Repulser useAgainstPlayer="34" useAgainstAI="35" absorb="36"/>
<Shuriken useAgainstPlayer="37" useAgainstAI="38" absorb="39"/>
<Template useAgainstPlayer="0" useAgainstAI="0" absorb="0"/>
</WeaponAIStats>"#;

#[test]
fn every_named_weapon_reads_its_own_row() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    assert_eq!(
        stats.get(Weapon::Rocket),
        Some(WeaponAiOdds {
            use_against_player: 1.0,
            use_against_ai: 2.0,
            absorb: 3.0,
        })
    );
    assert_eq!(
        stats.get(Weapon::Mine),
        Some(WeaponAiOdds {
            use_against_player: 28.0,
            use_against_ai: 29.0,
            absorb: 30.0,
        })
    );
    assert_eq!(
        stats.get(Weapon::LeachBeam),
        Some(WeaponAiOdds {
            use_against_player: 31.0,
            use_against_ai: 32.0,
            absorb: 33.0,
        })
    );
    assert_eq!(
        stats.get(Weapon::Shuriken),
        Some(WeaponAiOdds {
            use_against_player: 37.0,
            use_against_ai: 38.0,
            absorb: 39.0,
        })
    );
}

/// The Disruptor is Pure's, not authored by this file at all - a lookup for it
/// answers `None`, the same way a weapon a title's file omits does.
#[test]
fn a_weapon_the_file_does_not_author_answers_none() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    assert_eq!(stats.get(Weapon::Disruptor), None);
}

/// `<Template>` is matched by the loader and discarded, per `ai-stats.md` - it
/// is an authoring convention, not a fourteenth weapon, so it must not surface
/// as one.
#[test]
fn template_is_not_a_weapon() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    assert_eq!(
        stats.odds.len(),
        13,
        "Template should not have joined the rows"
    );
}

#[test]
fn no_root_is_an_error() {
    assert_eq!(parse("<NotThis/>"), Err(Error::MissingRoot));
}

#[test]
fn a_missing_attribute_is_an_error() {
    let xml = r#"<WeaponAIStats><Rockets useAgainstPlayer="1" useAgainstAI="2"/></WeaponAIStats>"#;
    assert_eq!(
        parse(xml),
        Err(Error::MissingAttribute {
            element: "Rockets",
            attribute: "absorb"
        })
    );
}

#[test]
fn a_non_numeric_attribute_is_an_error() {
    let xml = r#"<WeaponAIStats><Rockets useAgainstPlayer="not-a-number" useAgainstAI="2" absorb="3"/></WeaponAIStats>"#;
    assert_eq!(
        parse(xml),
        Err(Error::NotANumber {
            element: "Rockets",
            attribute: "useAgainstPlayer",
            value: "not-a-number".to_string()
        })
    );
}
