//! Binds a PS2 track's texture slots by name, and proves it never just
//! reshuffles the old flat ordinal scheme's answer.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(ps2_texture_binding_ground_truth)'
//! ```
//!
//! # What this pins
//!
//! `docs/formats/ps2-texture.md`'s "how a model finds its texture set" section
//! measured this exactly: 27 of the PS2 disc's 32 `<n>_Track`/`track_reversed`
//! models have as many nested-set entries as `Texture` nodes (no declared name
//! repeats), and the other 5 are short by 1-2 entries because a repeated name
//! collapses two nodes onto one entry. Under the flat ordinal scheme
//! `oag-render` used before 2026-09-05, a short set silently shifts every
//! ordinal past the duplicate onto its neighbour's texture, and the last
//! ordinal falls off the end - `12_Track`'s sky face, drawn white. Under
//! by-name resolution ([`mesh::Ps2TextureSet::resolve`]) a name is looked up
//! in the nested set's own directory rather than walked positionally, so the
//! collapse is invisible to it: every node, duplicate or not, resolves to the
//! one entry its own name always named.
//!
//! Two things below, not one: the 27 exact-match circuits must resolve
//! *exactly* what positional indexing already got right (a name-based
//! resolver reproducing the same answer where there is no collapse to
//! disagree with), and `12_Track` must resolve to *something different* from
//! the old positional answer at the ordinals the collapse used to shift.
//! "Every slot is `Some`" alone would also be true of a resolver that filled
//! every gap with the wrong neighbour's texture - which is exactly the bug
//! this exists to catch, and exactly why the RE thread that motivated this
//! test insisted on checking content, not just presence.
//!
//! [`ps2_ship_texture_slots_are_unchanged_by_name_resolution`] is the third
//! thing: no ship ever showed a duplicate name, so the whole roster should be
//! a byte-for-byte no-op under the new mechanism, checked rather than assumed.

use std::path::PathBuf;

use oag_formats::wad;

use oag_render::mesh;
use oag_vex::vex;

const PS2_IMAGE: &str = "pulse-ps2-eu.chd";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS2_IMAGE)
}

/// The nested set's own directory, parsed independently of
/// [`mesh::Ps2TextureSet`] - what "positional" meant before this change, and
/// the yardstick the exact-match circuits are checked against.
fn positional_hashes(set_blob: &[u8]) -> Vec<u32> {
    let directory = wad::Directory::parse(set_blob, Some(set_blob.len() as u64))
        .unwrap_or_else(|e| panic!("parsing the texture set directory: {e}"));
    directory.entries.iter().map(|e| e.name_hash).collect()
}

/// Every `Texture`-class node's own resolved entry, by calling
/// [`mesh::Ps2TextureSet::resolve`] directly - not a reimplementation of the
/// canonicalisation, so a bug in `resolve` itself shows up here too.
/// [`mesh::Ps2TextureSet::parse`] labels each decoded entry with its own hex
/// `name_hash`, which is what turns a binding into a string compare below
/// instead of a pointer identity that a coincidental re-decode could still
/// satisfy.
///
/// **Deliberately not [`mesh::resolve_texture_slots`]:** that function now
/// overwrites `resolve`'s hash label with the node's own declared name before
/// returning a slot, so a ship-skin or gantry-billboard match sees
/// `"texture1.tga"` rather than a hash it can never compare against - see its
/// own doc. `resolve` itself, called here, is unaffected: it still returns the same
/// hash-labelled entry it always did.
fn resolved_labels(data: &[u8], set: &mesh::Ps2TextureSet) -> Vec<Option<String>> {
    let classes = vex::classes_of(data).expect("class table");
    let nodes = vex::nodes(data).expect("decoding nodes");
    nodes
        .iter()
        .filter(|n| Some(n.class_id) == classes.texture)
        .map(|node| {
            let payload = data.get(node.payload())?;
            let name = vex::texture_asset_path(payload)?;
            set.resolve(&name).map(|t| t.label.clone())
        })
        .collect()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn ps2_texture_slots_resolve_by_name_not_position() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let mut exact_match_checked = 0usize;
    let mut near_miss_checked = 0usize;
    let mut sky_slot_151_is_some = false;
    let mut rebind_count = 0usize;

    for n in 1..=16u32 {
        for variant in ["track.vex", "track_reversed.vex"] {
            let circuit = format!("{n:02}_Track");
            let track = format!(r"Data\Environments\{circuit}\{variant}");
            let blob = archives
                .read_name(&track)
                .unwrap_or_else(|e| panic!("reading {track}: {e}"));
            let set_blob = archives
                .read_preceding(&track)
                .unwrap_or_else(|e| panic!("{track}: reading the entry before it: {e}"));

            let positional = positional_hashes(&set_blob);
            let set = mesh::Ps2TextureSet::parse(&set_blob).unwrap_or_else(|e| {
                panic!("{track}: the preceding entry is not a texture set ({e})")
            });
            let resolved = resolved_labels(&blob, &set);

            assert!(
                !resolved.is_empty(),
                "{track}: no Texture nodes at all - the class table lookup broke"
            );

            if resolved.len() == positional.len() {
                exact_match_checked += 1;
                for (i, label) in resolved.iter().enumerate() {
                    let want = format!("{:08x}", positional[i]);
                    assert_eq!(
                        label.as_deref(),
                        Some(want.as_str()),
                        "{track}: ordinal {i} resolved to {label:?} against the \
                         exact-match set's own entry {want} - a name-based resolver \
                         should reproduce the positional answer exactly where no \
                         declared name repeats"
                    );
                }
            } else {
                near_miss_checked += 1;
                for (i, label) in resolved.iter().enumerate() {
                    assert!(
                        label.is_some(),
                        "{track}: ordinal {i} resolved to nothing - by-name \
                         resolution should fill every slot on these 5 documented \
                         near-miss circuits (docs/formats/ps2-texture.md)"
                    );
                }
                if circuit == "12_Track" && variant == "track.vex" {
                    sky_slot_151_is_some = resolved.get(151).is_some_and(Option::is_some);
                    // Ordinals 64..=150: the RE thread's own sweep found these
                    // 87 bound their neighbour's texture one slot early under
                    // the old positional scheme - see
                    // docs/formats/ps2-texture.md and skycube.md. Asserting
                    // the exact count, not just "at least one", is what tells
                    // a resolver that rebinds the *wrong* 87 apart from one
                    // that rebinds the documented set.
                    for (i, want_hash) in positional.iter().enumerate().take(151).skip(64) {
                        let would_have_been = format!("{want_hash:08x}");
                        if resolved[i].as_deref() != Some(would_have_been.as_str()) {
                            rebind_count += 1;
                        }
                    }
                }
            }
        }
    }

    assert_eq!(
        exact_match_checked, 27,
        "expected 27 of 32 track models to have no repeated Texture node name \
         (docs/formats/ps2-texture.md); got {exact_match_checked}"
    );
    assert_eq!(
        near_miss_checked, 5,
        "expected 5 of 32 track models to be short a nested-set entry from a \
         repeated Texture node name; got {near_miss_checked}"
    );
    assert!(
        sky_slot_151_is_some,
        "12_Track's sky face (ordinal 151) still resolved to nothing - the bug \
         this test exists to catch"
    );
    assert_eq!(
        rebind_count, 87,
        "12_Track: expected exactly 87 of ordinals 64..=150 to resolve to a \
         different entry than the old positional scheme would have bound them \
         to (docs/formats/ps2-texture.md), got {rebind_count}"
    );
}

/// No ship on the roster ever showed a duplicate `Texture` node name, so this
/// change should be a no-op for every one - checked directly rather than
/// assumed, the same way the track side is above.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn ps2_ship_texture_slots_are_unchanged_by_name_resolution() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    // The full PS2 roster - see `docs/formats/dlc-pack.md` for why this is
    // twelve, not the PSP disc's eight.
    const TEAMS: [&str; 12] = [
        "AG_Systems",
        "Assegai",
        "Auricom",
        "EGX",
        "Feisar",
        "Goteki",
        "Harimau",
        "Icaras",
        "Mantis",
        "Piranha",
        "Qirex",
        "Triakis",
    ];

    let mut checked = 0usize;
    for team in TEAMS {
        let ship = format!(r"Data\Ships\{team}\Ship.vex");
        let blob = archives
            .read_name(&ship)
            .unwrap_or_else(|e| panic!("reading {ship}: {e}"));
        let set_blob = archives
            .read_preceding(&ship)
            .unwrap_or_else(|e| panic!("{ship}: reading the entry before it: {e}"));

        let positional = positional_hashes(&set_blob);
        let set = mesh::Ps2TextureSet::parse(&set_blob)
            .unwrap_or_else(|e| panic!("{ship}: the preceding entry is not a texture set ({e})"));
        let resolved = resolved_labels(&blob, &set);

        assert_eq!(
            resolved.len(),
            positional.len(),
            "{ship}: {} Texture node(s) against {} nested-set entries - a \
             duplicate name here would mean the ship roster is not the \
             clean no-op this test assumes",
            resolved.len(),
            positional.len()
        );
        for (i, label) in resolved.iter().enumerate() {
            let want = format!("{:08x}", positional[i]);
            assert_eq!(
                label.as_deref(),
                Some(want.as_str()),
                "{ship}: ordinal {i} resolved to {label:?} against the old \
                 positional scheme's own entry {want} - name resolution \
                 should be byte-for-byte unchanged for every ship"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, TEAMS.len(), "every team on the roster was checked");
}
