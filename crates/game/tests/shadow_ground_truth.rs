//! The `blob` shadow tier's placement, against a real circuit's own geometry.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this test:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all shadow
//! ```
//!
//! Its own file rather than another test in `race_ground_truth.rs`, which is
//! at its own size ratchet - see `scripts/check-file-size.py`.
//!
//! `oag_render::shadow`'s tests build a quad from a placement handed to them.
//! What they cannot ask is whether the *placement* is right on a real track,
//! which is a question about the cast: a wall hit taken for a floor, a reach
//! that does not reach, a normal read off the wrong surface. None of that
//! shows up in synthetic data.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_raceplay as race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// A full grid on the default circuit, so every slot's cast is exercised
/// rather than only the player's.
fn load_with_opponents() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// Every craft on the grid gets a blob shadow, on the floor it is actually
/// hovering over.
///
/// The composition question `oag_render::shadow`'s own tests cannot ask: those
/// build a quad from a placement handed to them, and this checks the placement
/// against a real circuit's own collision geometry. What would break it is the
/// cast - a wall hit taken for a floor, a reach that does not reach, a normal
/// read off the wrong surface - and none of that shows up in synthetic data.
///
/// Nothing here pins a *size* or a darkness: the fade and the lift are this
/// project's own numbers (see `docs/rendering/shadows.md`), and a test that
/// pinned them would be pinning our own arithmetic and calling it a
/// measurement.
#[test]
#[ignore = "needs a real disc image under data/images/"]
fn every_craft_on_the_grid_casts_a_blob_shadow_onto_the_floor() {
    let Some(loaded) = load_with_opponents() else {
        return;
    };
    let race = race::Race::start(loaded.setup);
    let placements = race.shadow_placements();
    assert_eq!(
        placements.len(),
        usize::from(race.ship_count()),
        "a craft resting on the grid has ground under it, so every slot places"
    );

    for placement in &placements {
        let slot = placement.silhouette;
        let ship = &race.sim.world.ships[slot];
        let body = &ship.physics.body;
        let up = body.up();
        // Below the craft, along its own up axis - not merely nearby.
        let height = (body.position - placement.contact).dot(up);
        assert!(
            height > 0.0,
            "slot {slot}: the contact is {height:.3} above the craft"
        );
        // On a floor, which is the whole point of filtering the cast: a wall
        // hit would give a normal pointing across the track rather than out of
        // it, and the quad would stand up on the barrier.
        let along_up = placement.normal.dot(up);
        assert!(
            along_up > 0.5,
            "slot {slot}: the surface normal is {along_up:.3} against the craft's own up, \
             which is a wall rather than a floor"
        );
        assert!(
            placement.strength > 0.0 && placement.strength <= 1.0,
            "slot {slot}: strength {}",
            placement.strength
        );
        // The hull's own footprint, so a zero here is a handling table that
        // did not arrive rather than a shadow that is merely small.
        assert!(
            placement.half_length > 0.0 && placement.half_width > 0.0,
            "slot {slot}: {:.3} x {:.3}",
            placement.half_length,
            placement.half_width
        );
        println!(
            "slot {slot}: {:.2} above a floor, strength {:.3}, {:.2} x {:.2} units",
            height,
            placement.strength,
            placement.half_length * 2.0,
            placement.half_width * 2.0
        );
    }
}

/// The `original` tier's projected hull lands on the surface, under the craft.
///
/// The composition `oag_render::shadow::hull_triangles`' own tests cannot
/// check: a real hull, a real craft pose and a real contact plane. What breaks
/// here is a space mismatch - a hull in one space projected against a plane in
/// another - which produces a shadow drawn on the craft rather than under it.
#[test]
#[ignore = "needs a real disc image under data/images/"]
fn the_projected_hull_lands_on_the_surface_under_the_craft() {
    let Some(loaded) = load_with_opponents() else {
        return;
    };
    let hulls = loaded.shadow_hulls.clone();
    let race = race::Race::start(loaded.setup);
    let placements = race.shadow_placements();
    let placement = &placements[0];
    let hull = hulls[0].as_ref().expect("the player's craft authors one");

    let mut vertices = Vec::new();
    let rings = oag_render::shadow::hull_triangles(
        &oag_render::shadow::Cast {
            hull,
            model: race.ship_model_matrix_of(0),
            axis: Vec3::from_array(oag_pulse::shadow::AUTHORED_AXIS),
            contact: placement.contact,
            normal: placement.normal,
            strength: placement.strength,
        },
        &mut vertices,
    );
    assert!(rings > 0, "no ring was drawn");
    assert_eq!(vertices.len() % 3, 0);

    let craft = race.sim.world.ships[0].physics.body.position;
    println!(
        "craft at {craft:?}, contact {:?}, {rings} ring(s), {} vertices",
        placement.contact,
        vertices.len()
    );
    let mut lowest = f32::MAX;
    let mut highest = f32::MIN;
    for vertex in &vertices {
        let point = Vec3::from_array(vertex.position);
        let height = (point - placement.contact).dot(placement.normal);
        lowest = lowest.min(height);
        highest = highest.max(height);
    }
    println!("projected height above the contact plane: {lowest} .. {highest}");
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for vertex in &vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex.position[axis]);
            max[axis] = max[axis].max(vertex.position[axis]);
        }
    }
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    println!("projected span {span:?}");
    // And it lands *under the craft*, not beside it: the projection is nearly
    // vertical, so the polygon's own centre is within a hull length of the
    // contact point. A hull projected in the wrong space still comes out flat
    // on the plane - this is the assertion that catches that.
    let centre = Vec3::new(
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    );
    // The ring itself, in the plane, so a bad *outline* can be told from a bad
    // *fill*: shoelace area against the area of its own convex hull.
    let outline = hull.outline(oag_pulse::shadow::AUTHORED_AXIS);
    println!(
        "{} ring(s), lengths {:?}",
        outline.len(),
        outline.iter().map(|r| r.len()).collect::<Vec<_>>()
    );
    let model = race.ship_model_matrix_of(0);
    let direction = model
        .transform_vector3(Vec3::from_array(oag_pulse::shadow::AUTHORED_AXIS))
        .normalize();
    let facing = direction.dot(placement.normal);
    let flat: Vec<[f32; 2]> = outline[0]
        .iter()
        .map(|slot| {
            let local = Vec3::from_array(hull.vertices[usize::from(*slot)]);
            let world = model.transform_point3(local);
            let travel = (placement.contact - world).dot(placement.normal) / facing;
            let point = world + direction * travel;
            [point.x, point.z]
        })
        .collect();
    let shoelace = |ring: &[[f32; 2]]| {
        let mut sum = 0.0;
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            sum += a[0] * b[1] - b[0] * a[1];
        }
        (sum * 0.5).abs()
    };
    println!("ring area {:.2}", shoelace(&flat));
    for point in &flat {
        print!("({:.2},{:.2}) ", point[0], point[1]);
    }
    println!();
    // What the fill actually covers, against what the ring encloses: a
    // triangulation that gives up early draws a smaller shape, and on screen
    // that reads as a shadow far too small for the craft.
    let mut filled = 0.0;
    for triangle in vertices.as_chunks::<3>().0 {
        let p: Vec<Vec3> = triangle
            .iter()
            .map(|v| Vec3::from_array(v.position))
            .collect();
        filled += (p[1] - p[0]).cross(p[2] - p[0]).length() * 0.5;
    }
    println!(
        "filled area {filled:.2} against ring area {:.2}",
        shoelace(&flat)
    );
    let drift = (centre - placement.contact).length();
    println!("projected centre is {drift:.3} from the contact point");
    assert!(
        drift < placement.half_length * 2.0,
        "the shadow is {drift} from under the craft, whose hull is {} long",
        placement.half_length * 2.0
    );
    println!(
        "hull local span {:?}",
        [
            hull.bounds.1[0] - hull.bounds.0[0],
            hull.bounds.1[1] - hull.bounds.0[1],
            hull.bounds.1[2] - hull.bounds.0[2],
        ]
    );
    // Every vertex sits on the plane, one lift above it - that is what
    // "projected onto the surface" means, and a hull drawn in its own space
    // instead would span the craft's own height here.
    assert!(
        (lowest - oag_render::shadow::LIFT).abs() < 1e-3
            && (highest - oag_render::shadow::LIFT).abs() < 1e-3,
        "the projection is not flat on the surface: {lowest} .. {highest}"
    );
}

/// The shadow hull is an approximation of the ship it belongs to.
///
/// **The check that catches loading the wrong node, or losing its placement in
/// the `.vex` tree** - both of which draw a shadow that is still a plausible
/// dark shape on the road. Rasterizes the craft's own mesh and its shadow hull
/// from above into one grid and compares coverage; the grids are printed too,
/// because the picture is the evidence and the percentages stand in for it.
#[test]
#[ignore = "needs a real disc image under data/images/"]
fn the_shadow_hull_is_the_shape_of_the_ship_it_belongs_to() {
    let Some(loaded) = load_with_opponents() else {
        return;
    };
    let hull = loaded.shadow_hulls[0].as_ref().expect("a hull");
    let model = &loaded.liveries[0].hull;

    // One grid, one scale, both shapes: the ship's triangles and the hull's
    // faces rasterized from above.
    let mut min = [f32::MAX; 2];
    let mut max = [f32::MIN; 2];
    let mut note = |x: f32, z: f32| {
        min[0] = min[0].min(x);
        max[0] = max[0].max(x);
        min[1] = min[1].min(z);
        max[1] = max[1].max(z);
    };
    for vertex in &model.vertices {
        note(vertex.position[0], vertex.position[2]);
    }
    for vertex in &hull.vertices {
        if *vertex != [0.0; 3] {
            note(vertex[0], vertex[2]);
        }
    }
    println!(
        "plan extent x {:.2}..{:.2}, z {:.2}..{:.2}",
        min[0], max[0], min[1], max[1]
    );

    const W: usize = 96;
    const H: usize = 40;
    let cell = |x: f32, z: f32| {
        let u = ((x - min[0]) / (max[0] - min[0]) * (W - 1) as f32).round() as usize;
        let v = ((z - min[1]) / (max[1] - min[1]) * (H - 1) as f32).round() as usize;
        (u.min(W - 1), v.min(H - 1))
    };
    let fill = |grid: &mut Vec<Vec<char>>, a: [f32; 3], b: [f32; 3], c: [f32; 3], mark: char| {
        // Scanline-free: sample the triangle's barycentric grid coarsely, which
        // is enough for a picture.
        for i in 0..=24 {
            for j in 0..=24 - i {
                let (u, v) = (i as f32 / 24.0, j as f32 / 24.0);
                let w = 1.0 - u - v;
                let x = a[0] * w + b[0] * u + c[0] * v;
                let z = a[2] * w + b[2] * u + c[2] * v;
                let (cx, cy) = cell(x, z);
                grid[cy][cx] = mark;
            }
        }
    };

    let mut ship = vec![vec![' '; W]; H];
    for triangle in model.indices.as_chunks::<3>().0 {
        let p: Vec<[f32; 3]> = triangle
            .iter()
            .map(|i| model.vertices[*i as usize].position)
            .collect();
        fill(&mut ship, p[0], p[1], p[2], '#');
    }
    let mut shadow = vec![vec![' '; W]; H];
    for face in &hull.faces {
        let index = face.indices();
        for corner in 1..index.len().saturating_sub(1) {
            let p: Vec<[f32; 3]> = [index[0], index[corner], index[corner + 1]]
                .iter()
                .map(|slot| hull.vertices[usize::from(*slot)])
                .collect();
            fill(&mut shadow, p[0], p[1], p[2], '#');
        }
    }

    println!("the ship's mesh from above:");
    for row in &ship {
        println!("|{}|", row.iter().collect::<String>());
    }
    println!("its shadow hull from above:");
    for row in &shadow {
        println!("|{}|", row.iter().collect::<String>());
    }

    let count = |grid: &Vec<Vec<char>>| {
        grid.iter()
            .flat_map(|row| row.iter())
            .filter(|c| **c == '#')
            .count()
    };
    let both = (0..H)
        .flat_map(|y| (0..W).map(move |x| (x, y)))
        .filter(|(x, y)| ship[*y][*x] == '#' && shadow[*y][*x] == '#')
        .count();
    let (ship_cells, shadow_cells) = (count(&ship), count(&shadow));
    let covered = 100.0 * both as f32 / ship_cells as f32;
    let spilled = 100.0 * (shadow_cells - both) as f32 / ship_cells as f32;
    println!(
        "hull covers {covered:.0}% of the ship's footprint and spills {spilled:.0}% beyond it"
    );
    // Generous bounds on purpose: what is being caught is a hull in the wrong
    // space or from the wrong node, which misses by most of the ship, not a
    // few percent of coverage. The shadow hull is a simplification and is
    // *expected* to be a little fatter than the mesh in places.
    assert!(
        covered > 80.0,
        "the hull covers only {covered:.0}% of the ship it belongs to"
    );
    assert!(
        spilled < 40.0,
        "the hull spills {spilled:.0}% beyond the ship it belongs to"
    );
}

/// The silhouettes a race load on `source` hands the `blob` tier, and the
/// load report's lines about them.
fn silhouettes(source: &std::path::Path, mode: oag_race::Mode) -> Silhouettes {
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        mode,
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded
        .report
        .iter()
        .filter(|line| line.contains("shadow silhouette") || line.contains("generated"))
        .cloned()
        .collect();
    Silhouettes {
        images: loaded.shadows,
        report,
    }
}

struct Silhouettes {
    images: Vec<oag_render::shadow::Silhouette>,
    report: Vec<String>,
}

/// Row sums of the red channel, the coverage `shadow.wgsl` reads, over every
/// eighth row. A ship's shadow is far from symmetric front to back (about 1,000
/// at the first and last rows against 13,000 in the middle, in a
/// 1,000/1,800/10,000/12,800/9,600/13,100/9,000/1,300 profile), so the order
/// of this vector is the image's row order.
fn row_profile(s: &oag_render::shadow::Silhouette) -> Vec<u32> {
    let (w, h) = (s.width as usize, s.height as usize);
    (0..h)
        .step_by(h / 8)
        .map(|y| (0..w).map(|x| u32::from(s.rgba[(y * w + x) * 4])).sum())
        .collect()
}

fn l1(a: &[u32], b: impl Iterator<Item = u32>) -> u32 {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(y)).sum()
}

/// A Zone race on Wipeout 2048 and the Omega Collection gives every slot the
/// Zone craft's own `Ambient_Shadow` - the one silhouette either title ships -
/// and it is the image HD ships for the same craft: the same size, coverage in
/// the red channel, and the same row order, so no flip is needed (unlike
/// Omega's front-end `.gnf`, see `FrontEnd::bottom_up_gnf`).
///
/// Fails if `shadow::silhouettes` goes back to composing `<ship_dir>\<team>`
/// (2048's native `Ships` tree has no Zone directory), stops swapping the
/// `.gtf` for the platform's `.gxt`/`.gnf`, or stops reaching the Zone hull's
/// directory in Zone mode.
#[test]
#[ignore = "needs HD's disc plus the decrypted Vita and PS4 packages"]
fn zone_craft_blob_shadow_is_the_discs_own_on_2048_and_omega() {
    let Some(hd) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let hd_zone = silhouettes(&hd, oag_race::Mode::Zone);
    let reference = &hd_zone.images[0];
    assert!(!reference.generated, "HD's zone craft ships its own");
    let profile = row_profile(reference);

    let titles = [
        (
            "data/extracted/vita/PCSF00007",
            "Zone\\textures\\ambient_shadow.gxt",
        ),
        ("data/extracted/ps4", "Zone\\textures\\ambient_shadow.gnf"),
    ];
    for (source, entry) in titles {
        let Some(path) = oag_testdata::exact(source) else {
            continue;
        };
        let zone = silhouettes(&path, oag_race::Mode::Zone);
        assert!(!zone.images.is_empty(), "{source}: a grid");
        for (slot, image) in zone.images.iter().enumerate() {
            assert!(!image.generated, "{source} slot {slot}: the disc's own");
            assert_eq!(
                (image.width, image.height),
                (reference.width, reference.height),
                "{source} slot {slot}"
            );
            assert!(
                image.rgba.as_chunks::<4>().0.iter().any(|px| px[0] > 200),
                "{source} slot {slot}: coverage must arrive in the red channel"
            );
            let own = row_profile(image);
            let straight = l1(&own, profile.iter().copied());
            let flipped = l1(&own, profile.iter().rev().copied());
            assert!(
                straight * 10 < flipped,
                "{source} slot {slot}: rows are HD's order, not reversed ({straight} vs {flipped})"
            );
        }
        assert!(
            zone.report.iter().all(|line| line.contains(entry)),
            "{source}: {:?}",
            zone.report
        );

        // Outside Zone mode these titles' own craft ship none.
        let race = silhouettes(&path, oag_race::Mode::SingleRace);
        assert!(
            race.images.iter().all(|image| image.generated),
            "{source}: no native craft has a silhouette on the disc"
        );
    }
}
