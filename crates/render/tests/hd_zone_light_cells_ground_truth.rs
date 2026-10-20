//! HD's Zone hull draws its side light cells from their own authored flash.
//!
//! **`#[ignore]`d and never run in CI**: needs a decrypted PS3 image in `data/images`
//! (`just test-data`).
//!
//! `dg_zonelights1`'s program adds `dg_flasha` (a scrolling mask) times `dg_flash3` (blue),
//! both read through the chunk's **second** coordinate set, to the picture
//! (`mesh::rcs::damage_lights`). Dropping the binding leaves the cells dark; dropping the
//! second coordinate set leaves every cell reading one texel of the blue, which looks the
//! same on a still, so the coordinates are asserted directly.

use crate::archive_cache;

use oag_mesh::mesh;

const ZONE_HULL: &str = "/data/ships/zone/ship.vex";

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_zone_hulls_light_cells_bind_their_flash_and_read_a_second_uv_set() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let data = archive_cache::read(&spec, ZONE_HULL).expect("the Zone hull's .vex reads");
    let geometry =
        mesh::rcs::sibling_geometry(&spec, ZONE_HULL, &data).expect("a sibling .rcsmodel");
    let (model, report) = mesh::rcs::build(
        ZONE_HULL,
        &data,
        &geometry,
        &mut |name| archive_cache::read(&spec, name),
        |c| c.mesh,
    )
    .expect("the Zone hull builds");

    assert_eq!(
        report.damage_lights_bound, 1,
        "one light-cell material binds"
    );
    assert_eq!(report.damage_lights_unread, 0);

    // The slot: the magstrip role with the light-cell glow entry (rate 3).
    let cells: Vec<usize> = model
        .material_slots
        .iter()
        .enumerate()
        .filter(|(_, packed)| **packed & mesh::slots::MAG_WAVE != 0)
        .map(|(slot, _)| slot)
        .collect();
    assert_eq!(
        cells.len(),
        1,
        "exactly one material takes the role: {cells:?}"
    );
    let slot = cells[0];
    let entry = mesh::slots::material_index(model.material_slots[slot]) as usize;
    let glow = model.emissive[entry - 1];
    assert!(
        (glow.rate - 3.0).abs() < f32::EPSILON,
        "the entry marks light cells: {glow:?}"
    );
    assert_eq!(
        glow.tint, [0.0; 3],
        "C is the authored colour, zero on the disc and live"
    );
    assert!(
        model.pad_masks.get(slot).is_some_and(Option::is_some)
            && model.wave_maps.get(slot).is_some_and(Option::is_some),
        "the blue (third binding) and the mask (fourth) are bound"
    );

    // Its vertices carry a second coordinate set that varies.
    let uv2: Vec<[f32; 2]> = model
        .vertices
        .iter()
        .filter(|v| mesh::slots::material_index(v.slots) as usize == entry)
        .map(|v| v.texcoord2)
        .collect();
    assert!(uv2.len() >= 16, "the cells have vertices: {}", uv2.len());
    let first = uv2[0];
    assert!(
        uv2.iter().any(|uv| *uv != first) && uv2.iter().any(|uv| *uv != [0.0, 0.0]),
        "texcoord2 is read, not left at the origin"
    );
}
