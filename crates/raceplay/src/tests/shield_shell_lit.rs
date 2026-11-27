//! Which shield shells draw under the read lit program (`oag_title::Looks::shield_shell_lit`,
//! `docs/ghidra/functions/ps3-hdfury-eu/shield.md`, "The shell's draw state").

use super::scene_build::triangle;
use crate::load::roster::lit_shield_shell;
use oag_mesh::mesh::{Emissive, Model, PROGRAM_SHIELD_LIGHTNING};

/// A shell as HD's loader builds it: one blended batch under a material whose state word
/// carries the cull bit (`0x39`, the Feisar shell's own).
fn shell() -> Model {
    let mut model = triangle();
    let mut draw = model.draws.remove(0);
    draw.texture = Some(0);
    model.transparent_draws = vec![draw];
    model.material_state = vec![0x39];
    model.shield_shell = true;
    model.emissive = vec![Emissive {
        tint: [0.0, 3.0, 0.0],
        offset: 0.0,
        scale: 1.0,
        rate: 0.0,
        program: [PROGRAM_SHIELD_LIGHTNING, 0.0],
    }];
    model
}

fn taken(mut model: Model, title: &oag_title::Title, own: bool) -> (bool, bool) {
    let options = crate::Options {
        own_shield: own,
        ..crate::Options::default()
    };
    lit_shield_shell(Some(&mut model), &options, title);
    (model.shield_lit, model.transparent_draws[0].culled)
}

/// HD's shell is lit by its own program and culled by its own state word. A title whose shell
/// program is unread draws as before, as does a shell with no `ShieldColour` or under another
/// program (Zone's).
#[test]
fn only_hd_takes_the_lit_culled_shell() {
    assert_eq!(taken(shell(), oag_hd::TITLE, false), (true, true));
    for unread in [oag_omega::TITLE, oag_2048::TITLE, oag_pulse::TITLE] {
        assert_eq!(
            taken(shell(), unread, false),
            (false, false),
            "{}",
            unread.name
        );
    }
    let mut plain = shell();
    plain.shield_shell = false;
    assert_eq!(taken(plain, oag_hd::TITLE, false), (false, false));
    let mut another = shell();
    another.emissive.clear();
    assert_eq!(taken(another, oag_hd::TITLE, false), (false, false));
}
