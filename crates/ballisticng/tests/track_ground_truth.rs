//! The 32 Workshop tracks, every one: the wrapper parses, the bundle opens, the
//! track classes read into typed structs or the file is refused by name, and
//! the numbers that cross-check the reader hold. Disc-backed (`data/ballisticng`),
//! so `#[ignore]`d and run by `just test-data`.

mod fields;

use oag_ballisticng::track::{ColliderKind, LoopEnd, Tile, TileKind, Track};
use oag_ballisticng::wtrk::Wtrk;
use oag_unity::assets::Assets;
use oag_unity::bundle::Bundle;

struct Loaded {
    id: String,
    title: String,
    track: Result<Track, String>,
}

/// What every `Texture2D` of every track decoded to, gathered on the same walk
/// that reads the tracks: a bundle is LZMA- or LZ4-unpacked once, not once per
/// claim.
#[derive(Default)]
struct Textures {
    ok: usize,
    empty: usize,
    refused: std::collections::BTreeMap<String, usize>,
}

impl Textures {
    fn decode_every(&mut self, a: &Assets) {
        use oag_ballisticng::track::ObjectRef;
        use oag_unity::serialized::class_id;
        use oag_unity::value::PPtr;
        for (fi, sf) in a.files().iter().enumerate() {
            for o in sf
                .objects
                .iter()
                .filter(|o| o.class_id == class_id::TEXTURE_2D)
            {
                let at = ObjectRef {
                    file: fi,
                    pptr: PPtr {
                        file_id: 0,
                        path_id: o.path_id,
                    },
                };
                match oag_ballisticng::texture::decode(a, at) {
                    Ok(img) if img.rgba.is_empty() => self.empty += 1,
                    Ok(img) => {
                        assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
                        self.ok += 1;
                    }
                    Err(e) => *self.refused.entry(e.to_string()).or_default() += 1,
                }
            }
        }
    }
}

fn load_all() -> Option<(Vec<Loaded>, Textures, fields::Fields)> {
    let root = oag_testdata::exact("data/ballisticng/workshop-473770")?;
    let mut dirs: Vec<_> = std::fs::read_dir(&root)
        .expect("workshop dir")
        .map(|e| e.expect("entry").path())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    let mut textures = Textures::default();
    let mut fields = fields::Fields::default();
    for d in dirs {
        for e in std::fs::read_dir(&d).expect("item dir") {
            let path = e.expect("entry").path();
            if path.extension().and_then(|x| x.to_str()) != Some("wtrk") {
                continue;
            }
            let file = std::fs::read(&path).expect("read item");
            let w = Wtrk::parse(&file).expect("wrapper parses");
            let search = file
                .windows(8)
                .position(|x| x == b"UnityFS\0")
                .expect("signature");
            assert_eq!(w.bundle_start, search, "{}: wrapper offset", w.title);
            assert_eq!(
                w.workshop_id.to_string(),
                d.file_name().unwrap().to_string_lossy(),
                "{}: id is the item directory",
                w.title
            );
            let b = Bundle::parse(w.bundle(&file)).expect("bundle opens");
            let a = Assets::open(&b).expect("serialized files open");
            textures.decode_every(&a);
            fields.count(&a);
            out.push(Loaded {
                id: w.workshop_id.to_string(),
                title: w.title,
                track: Track::read(&a).map_err(|e| e.to_string()),
            });
        }
    }
    Some((out, textures, fields))
}

/// Every claim over the 32 tracks, on one walk of them. Each claim is its own
/// function below; the walk is shared because each one used to unpack every
/// bundle again (`CLAUDE.md`, "find what already walks that data").
#[test]
#[ignore = "needs data/ballisticng"]
fn the_workshop_tracks() {
    let Some((all, textures, fields)) = load_all() else {
        return;
    };
    every_workshop_track_reads_or_is_refused_by_name(&all);
    the_section_graph_is_classified_on_every_track(&all);
    tile_positions_are_the_placed_centroids_of_their_vertices(&all);
    ai_lines_and_collider_kinds_across_the_census(&all);
    lions_den_matches_the_probe(&all);
    every_track_texture_decodes_or_is_refused_by_name(&textures);
    fields.the_fields_this_layer_does_not_read_are_frozen();
}

fn centroid(v: &[[f32; 3]], idx: &[usize]) -> [f32; 3] {
    let mut c = [0.0; 3];
    for &i in idx {
        for (a, b) in c.iter_mut().zip(v[i]) {
            *a += b;
        }
    }
    c.map(|x| x / idx.len() as f32)
}

fn count(tiles: &[Tile], kind: TileKind) -> usize {
    tiles.iter().filter(|t| t.kind == kind).count()
}

fn every_workshop_track_reads_or_is_refused_by_name(all: &[Loaded]) {
    assert_eq!(all.len(), 32, "the census");
    let refused: Vec<_> = all
        .iter()
        .filter_map(|l| l.track.as_ref().err().map(|e| (l.id.as_str(), e.as_str())))
        .collect();
    assert_eq!(refused, Vec::<(&str, &str)>::new(), "refusals");
}

fn the_section_graph_is_classified_on_every_track(all: &[Loaded]) {
    let (mut closed, mut rho, mut open) = (0, 0, 0);
    let mut unreached = Vec::new();
    let mut branches = 0;
    let mut pit_branches = 0;
    for l in all {
        let t = l.track.as_ref().expect("reads");
        match t.chains.end {
            LoopEnd::Closed => closed += 1,
            LoopEnd::Rho { .. } => rho += 1,
            LoopEnd::Open => open += 1,
        }
        if !t.chains.unreached.is_empty() {
            unreached.push((l.id.clone(), t.chains.unreached.len()));
        }
        branches += t.chains.branches.len();
        pit_branches += t.chains.branches.iter().filter(|b| b.pit).count();
        // Every section is on a chain or reported unreached.
        let on: usize = t.chains.main.len()
            + t.chains
                .branches
                .iter()
                .map(|b| b.sections.len())
                .sum::<usize>()
            + t.chains.unreached.len();
        assert_eq!(on, t.sections.len(), "{}: sections accounted for", l.title);
        // The lap line and mid line are on the lap loop.
        let lap = t.chains.lap_loop();
        assert!(
            lap.contains(&t.start_line),
            "{}: start line on the loop",
            l.title
        );
        assert!(
            lap.contains(&t.mid_line),
            "{}: mid line on the loop",
            l.title
        );
    }
    assert_eq!((closed, rho, open), (31, 1, 0), "how the main chains end");
    assert_eq!(unreached, vec![("1782764483".to_string(), 92)]);
    assert_eq!((branches, pit_branches), (33, 32));
}

fn tile_positions_are_the_placed_centroids_of_their_vertices(all: &[Loaded]) {
    let mut worst = 0.0f32;
    let mut checked = 0;
    let mut scales = std::collections::BTreeMap::<String, usize>::new();
    for l in all {
        let t = l.track.as_ref().expect("reads");
        let fp = t.floor_placement().expect("one floor placement");
        *scales
            .entry(format!("{:?}", fp.uniform_scale()))
            .or_default() += 1;
        for tile in t.floor_tiles.iter().filter(|x| !x.indices.is_empty()) {
            let c = fp.apply(centroid(&t.floor_vertices, &tile.indices));
            for (a, b) in c.iter().zip(tile.position) {
                worst = worst.max((a - b).abs());
            }
            checked += 1;
        }
        // Wall tiles are the exception: unscaled mesh space.
        for tile in t.wall_tiles.iter().filter(|x| !x.indices.is_empty()) {
            let c = centroid(&t.wall_vertices, &tile.indices);
            for (a, b) in c.iter().zip(tile.position) {
                assert!(
                    (a - b).abs() < 1e-3,
                    "{}: wall tile {}",
                    l.title,
                    tile.index
                );
            }
        }
        assert_eq!(
            Some(fp),
            t.wall_placement(),
            "{}: floor and wall share a pose",
            l.title
        );
    }
    assert!(
        worst < 1e-3,
        "floor tile positions are placed centroids: {worst}"
    );
    assert_eq!(checked, 31_748);
    assert_eq!(
        scales.into_iter().collect::<Vec<_>>(),
        vec![
            ("Some(0.92)".to_string(), 1),
            ("Some(1.0)".to_string(), 26),
            ("Some(1.1)".to_string(), 3),
            ("Some(1.15)".to_string(), 2),
        ]
    );
}

fn ai_lines_and_collider_kinds_across_the_census(all: &[Loaded]) {
    let mut no_line = Vec::new();
    let mut kind3_tracks = 0;
    let mut unresolved = Vec::new();
    for l in all {
        let t = l.track.as_ref().expect("reads");
        if t.racing_lines.is_empty() {
            no_line.push(l.id.as_str());
        }
        for line in &t.racing_lines {
            assert_eq!(
                line.len(),
                t.sections.len(),
                "{}: a point per section",
                l.title
            );
            for (i, p) in line.iter().enumerate() {
                assert_eq!(p.section, i, "{}", l.title);
            }
        }
        kind3_tracks += usize::from(
            t.colliders
                .iter()
                .any(|c| c.kind == oag_ballisticng::track::ColliderKind::Unknown(3)),
        );
        if t.unresolved_scripts > 0 {
            unresolved.push((l.id.as_str(), t.unresolved_scripts));
        }
    }
    assert_eq!(no_line, ["2075028166"]);
    assert_eq!(kind3_tracks, 15);
    assert_eq!(unresolved, [("2175514157", 3)]);
}

fn lions_den_matches_the_probe(all: &[Loaded]) {
    let l = all
        .iter()
        .find(|l| l.id == "1392209921")
        .expect("Lion's Den");
    let t = l.track.as_ref().expect("reads");
    assert_eq!(t.sections.len(), 344);
    assert_eq!(t.chains.main.len(), 307);
    assert_eq!(t.chains.end, LoopEnd::Closed);
    assert_eq!(t.chains.branches.len(), 1);
    let pit = &t.chains.branches[0];
    assert_eq!(
        (pit.from, pit.sections.len(), pit.joins, pit.pit),
        (9, 37, Some(49), true)
    );
    assert!(t.chains.unreached.is_empty());
    assert_eq!((t.start_section, t.start_line, t.mid_line), (0, 52, 219));
    assert_eq!(t.floor_tiles.len(), 688);
    assert_eq!(t.wall_tiles.len(), 650);
    assert_eq!(count(&t.floor_tiles, TileKind::Speed), 54);
    assert_eq!(count(&t.floor_tiles, TileKind::Weapon), 11);
    assert_eq!(count(&t.floor_tiles, TileKind::Recharge), 74);
    assert_eq!(count(&t.floor_tiles, TileKind::Grid), 8);
    assert_eq!(count(&t.floor_tiles, TileKind::Plain), 541);
    assert_eq!(count(&t.wall_tiles, TileKind::Plain), 650);
    assert_eq!(t.floor_triangles().len(), 1_376);
    assert_eq!(t.wall_triangles().len(), 1_300);
    assert_eq!(
        (
            t.speed_atlas,
            t.weapon_atlas,
            t.recharge_atlas,
            t.spawn_atlas
        ),
        (0, 1, 3, 7)
    );
    let grid: Vec<usize> = t
        .floor_tiles
        .iter()
        .filter(|x| x.kind == TileKind::Grid)
        .map(|x| x.section.expect("floor tile section"))
        .collect();
    assert_eq!(grid.iter().copied().min(), Some(18));
    assert_eq!(grid.iter().copied().max(), Some(46));
    // Pad atlas indices pair with the tile type exactly.
    for tile in &t.floor_tiles {
        let want = match tile.kind {
            TileKind::Speed => Some(t.speed_atlas),
            TileKind::Weapon => Some(t.weapon_atlas),
            TileKind::Recharge => Some(t.recharge_atlas),
            TileKind::Grid => Some(t.spawn_atlas),
            _ => None,
        };
        if let Some(w) = want {
            assert_eq!(tile.atlas_index, w);
        }
    }
    // The AI line has a point per section and follows the section chain.
    assert_eq!(t.racing_lines.len(), 1);
    let line = &t.racing_lines[0];
    assert_eq!(line.len(), 344);
    for (i, p) in line.iter().enumerate() {
        assert_eq!(p.section, i);
        assert_eq!(Some(p.next), t.sections[i].next, "AI line follows Next");
    }
    let kinds: Vec<_> = t.colliders.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds.iter().filter(|k| **k == ColliderKind::Floor).count(),
        1
    );
    assert_eq!(
        kinds.iter().filter(|k| **k == ColliderKind::Wall).count(),
        1
    );
    assert_eq!(t.floor_placement().unwrap().uniform_scale(), Some(1.15));
}

fn open_item(id: &str) -> Option<(Vec<u8>, Wtrk)> {
    let root = oag_testdata::exact("data/ballisticng/workshop-473770")?;
    let path = std::fs::read_dir(root.join(id)).ok()?.next()?.ok()?.path();
    let file = std::fs::read(path).ok()?;
    let w = Wtrk::parse(&file).ok()?;
    Some((file, w))
}

fn byte_sum(b: &[u8]) -> u32 {
    b.iter().fold(0u32, |a, &x| a.wrapping_add(u32::from(x)))
}

/// UnityPy 1.25.4 is the second source: the byte sum of the RGBA bytes of the
/// same texture, top row first, and its first texel run.
#[test]
#[ignore = "needs data/ballisticng"]
fn lions_den_atlas_decodes_as_unitypy_does() {
    let Some((file, w)) = open_item("1392209921") else {
        return;
    };
    let b = Bundle::parse(w.bundle(&file)).unwrap();
    let a = Assets::open(&b).unwrap();
    let t = Track::read(&a).unwrap();
    let diffuse = oag_ballisticng::texture::decode(&a, t.diffuse.unwrap()).unwrap();
    assert_eq!((diffuse.width, diffuse.height), (1024, 512));
    // BC1 interpolates a third of the way between the end colours: this reader
    // rounds, UnityPy truncates. Measured over the whole atlas: alpha identical
    // on all 524,288 texels, every colour byte within 1 (87-94% identical),
    // which puts our byte sum 130,683 above UnityPy's 200,735,355.
    let sum = byte_sum(&diffuse.rgba);
    assert_eq!(sum - 200_735_355, 130_683);
    // Orientation: the byte sums of the top and the bottom row, top row first.
    let row = 1024 * 4;
    let near = |ours: u32, theirs: u32| ours.abs_diff(theirs) < theirs / 100;
    assert!(near(byte_sum(&diffuse.rgba[..row]), 437_159));
    assert!(near(
        byte_sum(&diffuse.rgba[diffuse.rgba.len() - row..]),
        368_786
    ));
    let illum = oag_ballisticng::texture::decode(&a, t.illum.unwrap()).unwrap();
    assert_eq!((illum.width, illum.height), (1024, 512));
    assert_eq!(byte_sum(&illum.rgba), 75_854_033);
}

fn every_track_texture_decodes_or_is_refused_by_name(t: &Textures) {
    assert_eq!(t.refused, std::collections::BTreeMap::new());
    // 1,753 textures (the probe's census); the 32 empty ones are each track's
    // 0 by 0 dynamic-font atlas.
    assert_eq!((t.ok, t.empty), (1_721, 32));
}

#[test]
#[ignore = "needs data/ballisticng"]
fn every_ship_item_is_a_bare_bundle_and_the_wrapper_counts_hold() {
    let Some(root) = oag_testdata::exact("data/ballisticng/workshop-473770") else {
        return;
    };
    let (mut csf, mut wtrk, mut shp, mut original_tracks, mut ports) = (0, 0, 0, 0, 0);
    for d in std::fs::read_dir(&root).unwrap() {
        for e in std::fs::read_dir(d.unwrap().path()).unwrap() {
            let path = e.unwrap().path();
            let file = std::fs::read(&path).unwrap();
            match path.extension().and_then(|x| x.to_str()) {
                Some("csf") => {
                    oag_ballisticng::wtrk::csf_bundle(&file).expect("a .csf is a bundle");
                    csf += 1;
                }
                Some("wtrk") => {
                    let w = Wtrk::parse(&file).expect("wrapper");
                    original_tracks +=
                        usize::from(w.has_tag("Original") && !w.has_tag("Port/Remake"));
                    ports += usize::from(w.has_tag("Port/Remake"));
                    wtrk += 1;
                }
                Some("shp") => shp += 1,
                other => panic!("an item of kind {other:?}"),
            }
        }
    }
    assert_eq!((csf, wtrk, shp), (47, 32, 13));
    // The tracks later steps may play: tagged Original and not Port/Remake.
    assert_eq!((original_tracks, ports), (27, 5));
}
