//! The music wave on the 2048 campaign map: the levels the music path hands over
//! lift the cells round the current event, and with none the map is the static
//! picture.

use super::wipeout2048::{boot, reach_the_shell, three_events, tick};
use super::*;
use crate::frontend::{canvas_cells, canvas_lattice};

fn on_the_map() -> (Frontend, Input) {
    let mut frontend = boot(0);
    frontend.set_campaign(three_events());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    let mut put = |name: &str, x: u32, size: (u32, u32)| {
        frontend.placements.push((
            name.to_string(),
            Placed {
                x,
                y: 0,
                width: size.0,
                height: size.1,
                quad_extent: None,
                quad_centre: None,
                blend: None,
                scale: [1.0, 1.0],
            },
        ));
    };
    put(canvas_lattice::TILE, 0, (960, 544));
    for fill in 0..=4_u8 {
        for up in [true, false] {
            put(
                canvas_cells::name(fill, up),
                1000 + u32::from(fill) * 200 + if up { 0 } else { 100 },
                canvas_cells::layout(up).size,
            );
        }
    }
    (frontend, input)
}

fn bass(level: f32) -> Option<[f32; 12]> {
    let mut levels = [0.0; 12];
    levels[1] = level;
    Some(levels)
}

fn triangles(frontend: &Frontend) -> usize {
    frontend
        .draw_list()
        .iter()
        .map(|draw| match draw {
            Draw::Triangles(list) => list.len(),
            _ => 0,
        })
        .sum()
}

#[test]
fn a_moving_bass_raises_cells_and_no_levels_leaves_the_static_map() {
    let (mut frontend, mut input) = on_the_map();
    for _ in 0..30 {
        tick(&mut frontend, &mut input, 0);
    }
    assert_eq!(triangles(&frontend), 0, "no levels, no wave");

    for step in 0..120 {
        frontend.set_music_levels(bass(if (step / 15) % 2 == 0 { 0.9 } else { 0.1 }));
        tick(&mut frontend, &mut input, 0);
    }
    assert!(
        triangles(&frontend) > 50,
        "the bass lifts the cells round the event"
    );

    frontend.set_music_levels(None);
    tick(&mut frontend, &mut input, 0);
    assert_eq!(triangles(&frontend), 0, "levels gone, the wave is cleared");
}

#[test]
fn a_steady_bass_does_not_move_the_map() {
    let (mut frontend, mut input) = on_the_map();
    for _ in 0..120 {
        frontend.set_music_levels(bass(0.6));
        tick(&mut frontend, &mut input, 0);
    }
    assert_eq!(
        triangles(&frontend),
        0,
        "only a change in the bass lifts a cell"
    );
}
