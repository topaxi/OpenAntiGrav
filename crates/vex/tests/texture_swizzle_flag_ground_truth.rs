//! Measures the byte at `Texture` node payload `+0x06` across every `.vex` file
//! Pulse ships, on both of its pressings, and pins what bit 0 of it does.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! **Settled 2026-10-01**: bit 0 is the pre-swizzle flag on Pulse too, and
//! `vex::textures` acts on it on every version (see
//! `shield_texture_swizzle_ground_truth.rs` and `docs/formats/pure-status.md`).
//! The sweep's distribution is unchanged and pinned here.
//!
//! # The question
//!
//! `docs/formats/pure-status.md` records (confidence 88) that the `Texture`
//! payload's flags byte at `+0x06` has bit 0 = "texels already in the GE's
//! 16-byte by 8-row block order". It first said "set only on font atlases" on
//! Pulse (since corrected there, by this sweep). Acting on it is free for Pulse
//! only if the bit is clear on **every** Pulse `Texture` node; otherwise a decoder
//! returns different texels for a Pulse asset and a circuit screenshot need not
//! cover the model that changed. Hence: measure, then pin.
//!
//! # What the sweep found: bit 0 **is** set on Pulse
//!
//! On **88 of 5,375** `Texture` nodes on the PSP pressing and **120 of 8,972** on
//! the PS2 one, none font atlases: every livery, glass, engine and environment-map
//! texture of the EGX, Feisar, Goteki, Piranha, Triakis and Zone ships, the mine,
//! bomb and shuriken effects, and the Zone shipwreck set.
//!
//! **Only the byte is measured, never its meaning.** Pulse's Feisar model has
//! eight `Texture` nodes reading **`0xe5`**, where `pure-status.md` still records
//! Pulse's eight as `0xe4`: one bit apart, the bit the claim turns on (a measured
//! discrepancy, not a correction).
//!
//! Independent of that: acting on bit 0 decodes 88 Pulse PSP textures differently,
//! a Pulse behaviour change on either reading, and a version gate cannot avoid it
//! (set on version-6 files, 74 + 1 nodes, *and* version-4 ones, 13).
//!
//! # What the payload byte is, and what it is not
//!
//! The full per-version distribution is pinned in [`PSP_FLAGS`] and [`PS2_FLAGS`]
//! (eight distinct values on the PSP, eleven on the PS2) so a change in any bit
//! shows. **Only bit 0 has a documented meaning and is interpreted.**
//!
//! - **The flag is whole-file**: every file that sets bit 0 sets it on *all* its
//!   `Texture` nodes (3 of 3, 8 of 8, 90 of 90), an export-time setting rather
//!   than a per-asset one. Asserted.
//! - **The 33 version-4 files read identically on both pressings** (`0x22` x22,
//!   `0x60` x20, `0x61` x13): the same legacy assets shipped twice.
//! - **The PS2 pressing's dominant value is `0x64`** (PSP: `0x80`, `0xc2`). Its
//!   `.vex` texture block is empty ([`vex::textures`]), so these bytes do not
//!   select a layout for texels the file does not carry.
//!
//! # Why the class ID comes from the file
//!
//! [`vex::classes_of`] picks the table from each file's version word. Sweeping on
//! [`vex::CLASS_TEXTURE`] (version 6's `0x3c1`) would silently miss version-4
//! files' `0x373` textures, 13 of the 88 flagged PSP nodes.
//!
//! # Coverage
//!
//! All four PSP archives, not just the two a title package names: `FEData.wad`
//! and `BEData.wad` hold 70 version-6 `.vex` files and four flagged nodes. On PS2,
//! `WADS2.WAD` and `WADSP.WAD`; `PS2MUSIC.WAD` and `PRERACE.WAD` are not
//! containers this project parses (`oag_pulse::archives::ps2`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use oag_assets::Archive;
use oag_formats::wad::Compression;
use oag_vex::vex;

/// The flags byte under test, in a `Texture` node's payload.
const FLAGS_AT: usize = 0x06;

/// The bit `pure-status.md` reads as "these texels are already swizzled".
const PRE_SWIZZLED: u8 = 0x01;

/// Largest texture dimension either corpus declares; used only to corroborate a
/// `Texture` payload before `+0x06` is read (see [`assert_texture_shaped`]).
const MAX_DIMENSION: u16 = 1024;

/// The PSP-only archives no title package names (not one of the two archive
/// *roles* `oag_pulse::TITLE` describes). Swept anyway: they hold 70 version-6
/// `.vex` files and four flagged nodes, so a corpus-wide claim without them is a
/// claim about part of it.
const PSP_EXTRA_ARCHIVES: &[&str] = &[oag_pulse::archives::FEDATA, oag_pulse::archives::BEDATA];

/// `.vex` files on the PSP pressing, per format version word.
const PSP_VERSIONS: &[(u32, usize)] = &[(4, 33), (6, 382)];

/// `Texture` nodes on the PSP pressing.
const PSP_TEXTURE_NODES: usize = 5375;

/// Every `(version, +0x06, count)` measured on the PSP pressing.
///
/// Pinned whole rather than reduced to "bit 0 is never set", so a change in any of
/// the other seven bits shows as a diff. `0x61`, `0xe1` and `0xe5` have bit 0 set:
/// 13 + 1 + 74 = 88.
const PSP_FLAGS: &[(u32, u8, usize)] = &[
    (4, 0x22, 22),
    (4, 0x60, 20),
    (4, 0x61, 13),
    (6, 0x80, 2384),
    (6, 0xc2, 1753),
    (6, 0xe1, 1),
    (6, 0xe4, 1108),
    (6, 0xe5, 74),
];

/// PSP `Texture` nodes with bit 0 set.
const PSP_PRE_SWIZZLED: usize = 88;

/// `.vex` files on the PS2 pressing, per format version word.
const PS2_VERSIONS: &[(u32, usize)] = &[(4, 33), (6, 1005)];

/// `Texture` nodes on the PS2 pressing.
const PS2_TEXTURE_NODES: usize = 8972;

/// Every `(version, +0x06, count)` measured on the PS2 pressing.
const PS2_FLAGS: &[(u32, u8, usize)] = &[
    (4, 0x22, 22),
    (4, 0x60, 20),
    (4, 0x61, 13),
    (6, 0x23, 90),
    (6, 0x64, 8164),
    (6, 0x80, 6),
    (6, 0xc2, 21),
    (6, 0xe1, 1),
    (6, 0xe4, 7),
    (6, 0xe5, 16),
    (6, 0xe6, 612),
];

/// PS2 `Texture` nodes with bit 0 set.
const PS2_PRE_SWIZZLED: usize = 120;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// One flagged node, in enough detail to look it up again.
struct Flagged {
    file: String,
    version: u32,
    name: Option<String>,
    flags: u8,
}

/// What one sweep measured, in the terms the question is asked in.
#[derive(Default)]
struct Sweep {
    /// Archives actually opened, by their own labels.
    archives: Vec<String>,
    /// Entries carrying the `.vex` magic, whatever their version.
    vex_files: usize,
    /// `.vex` files per format version word.
    versions: BTreeMap<u32, usize>,
    /// Files whose version has no class table at all
    /// ([`vex::Error::UnknownVersion`]), counted so a whole format generation
    /// contributing nothing shows as a number, not a clean-looking sweep.
    unknown_version: usize,
    /// Files whose version has a table, but whose `Texture` ID is unrecovered.
    no_texture_id: usize,
    /// Files whose node tree would not walk.
    unwalkable: usize,
    /// `Texture` nodes visited.
    texture_nodes: usize,
    /// Nodes whose payload is too short to hold the flags byte at all.
    short_payloads: usize,
    /// How often each value of the `+0x06` byte occurred, per format version.
    flags: BTreeMap<(u32, u8), usize>,
    /// Every node with bit 0 set.
    pre_swizzled: Vec<Flagged>,
    /// Per file: `Texture` nodes, of which flagged; only files with a flagged node
    /// are recorded, making "all eight of Feisar's" checkable.
    per_file: BTreeMap<String, (usize, usize)>,
}

impl Sweep {
    /// The distribution in the shape the pinned tables are written in.
    fn histogram(&self) -> Vec<(u32, u8, usize)> {
        self.flags
            .iter()
            .map(|(&(version, value), &count)| (version, value, count))
            .collect()
    }

    fn versions(&self) -> Vec<(u32, usize)> {
        self.versions.iter().map(|(&v, &n)| (v, n)).collect()
    }

    fn report(&self, label: &str) {
        println!("{label}: {} archive(s)", self.archives.len());
        for archive in &self.archives {
            println!("    {archive}");
        }
        println!(
            "  {} .vex files, versions {:?}",
            self.vex_files,
            self.versions()
        );
        println!(
            "  skipped: {} unknown version, {} without a Texture id, {} unwalkable",
            self.unknown_version, self.no_texture_id, self.unwalkable
        );
        println!(
            "  {} Texture nodes, {} payloads too short for +0x{FLAGS_AT:02x}",
            self.texture_nodes, self.short_payloads
        );
        for ((version, value), count) in &self.flags {
            let bit = if value & PRE_SWIZZLED != 0 {
                " <- bit 0 set"
            } else {
                ""
            };
            println!("    v{version} +0x{FLAGS_AT:02x} = {value:#04x}: {count}{bit}");
        }
        println!("  bit 0 set on {} node(s), in:", self.pre_swizzled.len());
        for (file, (total, flagged)) in &self.per_file {
            println!("    {file}: {flagged} of {total} Texture node(s)");
        }
    }

    /// Names of the flagged nodes, deduplicated (one texture is embedded in several
    /// models).
    fn flagged_names(&self) -> BTreeSet<&str> {
        self.pre_swizzled
            .iter()
            .filter_map(|f| f.name.as_deref())
            .collect()
    }

    /// Versions that contribute at least one flagged node.
    fn flagged_versions(&self) -> BTreeSet<u32> {
        self.pre_swizzled.iter().map(|f| f.version).collect()
    }
}

/// Corroborates that `payload` really is a `Texture` payload before `+0x06` is
/// believed: a node walk off by any amount would put `+0x06` inside another
/// structure and give a distribution that still *looks* like flags. The
/// dimensions and `bits_per_pixel` at `+0x00..+0x05` are the cheap check (a wrong
/// base gives non-power-of-two sizes and nonsense bit depths).
fn assert_texture_shaped(label: &str, payload: &[u8]) {
    let width = u16::from_le_bytes([payload[0], payload[1]]);
    let height = u16::from_le_bytes([payload[2], payload[3]]);
    let bits_per_pixel = payload[4];
    assert!(
        width.is_power_of_two() && width <= MAX_DIMENSION,
        "{label}: width {width} is not a texture dimension"
    );
    assert!(
        height.is_power_of_two() && height <= MAX_DIMENSION,
        "{label}: height {height} is not a texture dimension"
    );
    assert!(
        matches!(bits_per_pixel, 4 | 8 | 16 | 32),
        "{label}: {bits_per_pixel} bits per pixel"
    );
}

/// Every `Texture` node in one archive, added to `sweep`.
fn sweep_archive(archive: &mut Archive, sweep: &mut Sweep) {
    let label = archive.label().to_string();
    sweep.archives.push(label.clone());

    let entries: Vec<(usize, u32, Compression)> = archive
        .directory()
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (index, entry.size, entry.compression))
        .collect();

    for (index, size, compression) in entries {
        if size == 0 {
            continue;
        }
        // A stored blob's magic is readable without decompressing (every PSP
        // archive is stored), turning a read of 315 MiB of `Data.wad` into a 16-byte
        // peek for non-`.vex` entries. Compressed entries (every PS2 entry) are
        // decompressed anyway, so the peek is skipped for them.
        if compression == Compression::None {
            let Ok(head) = archive.peek(index, vex::FILE_HEADER_LEN as u64) else {
                continue;
            };
            if !vex::has_magic(&head) {
                continue;
            }
        }
        let Ok(bytes) = archive.read(index) else {
            continue;
        };
        if !vex::has_magic(&bytes) {
            continue;
        }

        sweep.vex_files += 1;
        if let Ok(version) = vex::version(&bytes) {
            *sweep.versions.entry(version).or_default() += 1;
        }

        // The class ID comes from the file's version word: `Data\Defaults\Skycube.vex`
        // is version 4 on a version-6 disc with `Texture` `0x373`, which a sweep on
        // `vex::CLASS_TEXTURE` would walk past.
        let classes = match vex::classes_of(&bytes) {
            Ok(classes) => classes,
            Err(vex::Error::UnknownVersion { .. }) => {
                sweep.unknown_version += 1;
                continue;
            }
            Err(_) => {
                sweep.unwalkable += 1;
                continue;
            }
        };
        let Some(texture_class) = classes.texture else {
            sweep.no_texture_id += 1;
            continue;
        };
        let Ok(tree) = vex::nodes(&bytes) else {
            sweep.unwalkable += 1;
            continue;
        };

        let file = format!("{label} entry {index}");
        let mut in_file = 0;
        let mut flagged_in_file = 0;

        for node in tree.iter().filter(|n| n.class_id == texture_class) {
            sweep.texture_nodes += 1;
            in_file += 1;
            let payload = &bytes[node.payload()];
            let Some(&flags) = payload.get(FLAGS_AT) else {
                sweep.short_payloads += 1;
                continue;
            };
            *sweep.flags.entry((classes.version, flags)).or_default() += 1;
            if flags & PRE_SWIZZLED != 0 {
                assert_texture_shaped(&file, payload);
                flagged_in_file += 1;
                sweep.pre_swizzled.push(Flagged {
                    file: file.clone(),
                    version: classes.version,
                    name: node.name.clone(),
                    flags,
                });
            }
        }
        if flagged_in_file > 0 {
            sweep.per_file.insert(file, (in_file, flagged_in_file));
        }
    }
}

/// Opens every archive on `source` that this project parses, as Pulse.
///
/// The two archive *roles* come from `oag_pulse::open`, so this works unchanged on
/// the PS2 pressing (`WADS2.WAD`, `WADSP.WAD` under a serial-derived directory).
/// [`PSP_EXTRA_ARCHIVES`] are tried by name and fail to open on a PS2 source, so
/// no platform test is needed.
fn sweep_source(path: &Path) -> Sweep {
    let source = path.to_str().expect("image path is utf-8");
    let mut archives = oag_pulse::open(source).expect("open as Pulse");
    let mut sweep = Sweep::default();

    // Both Pulse sources are WADs, so unwrapping here states that.
    sweep_archive(
        archives
            .data
            .as_wad_mut("the entry directory")
            .expect("a WAD"),
        &mut sweep,
    );
    if let Some(fe) = archives.fe.as_mut() {
        sweep_archive(
            fe.as_wad_mut("the entry directory").expect("a WAD"),
            &mut sweep,
        );
    }
    for extra in PSP_EXTRA_ARCHIVES {
        if let Ok(mut archive) = Archive::open(&format!("{source}:{extra}")) {
            sweep_archive(&mut archive, &mut sweep);
        }
    }
    sweep
}

/// Checks the parts of a sweep that must hold whichever pressing it came from.
fn assert_sweep_covered(sweep: &Sweep, label: &str) {
    assert!(
        !sweep.archives.is_empty(),
        "{label}: no archive opened, so the sweep measured nothing"
    );
    // An empty node set is a finding, not a pass: PS2 `.vex` scenes carry `Texture`
    // nodes with real dimensions and an empty texel block (`vex::textures`), so
    // zero nodes would mean the sweep stopped working.
    assert!(
        sweep.texture_nodes > 0,
        "{label}: no Texture nodes found; an empty sweep is not a clean one"
    );
    assert_eq!(
        sweep.short_payloads, 0,
        "{label}: a Texture payload too short to hold +0x{FLAGS_AT:02x}"
    );
    assert_eq!(
        (sweep.unknown_version, sweep.no_texture_id, sweep.unwalkable),
        (0, 0, 0),
        "{label}: files were skipped, so the distribution is over a subset"
    );
    assert_eq!(
        sweep.flags.values().sum::<usize>(),
        sweep.texture_nodes,
        "{label}: the histogram does not account for every node"
    );

    // The flag is a property of the *file*: on both pressings every file that sets
    // bit 0 sets it on all of its `Texture` nodes (3 of 3, 8 of 8, 90 of 90), an
    // export-time setting, worth knowing before a decoder decides where to branch.
    for (file, &(total, flagged)) in &sweep.per_file {
        assert_eq!(
            total, flagged,
            "{file}: {flagged} of {total} Texture nodes flagged, so the flag is \
             no longer whole-file"
        );
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn pulse_psp_sets_bit_zero_of_the_texture_flags_byte_on_ship_models() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let sweep = sweep_source(&path);
    sweep.report("pulse-psp-usa");
    assert_sweep_covered(&sweep, "pulse-psp-usa");

    assert_eq!(sweep.versions(), PSP_VERSIONS);
    assert_eq!(sweep.texture_nodes, PSP_TEXTURE_NODES);
    assert_eq!(
        sweep.histogram(),
        PSP_FLAGS,
        "the +0x{FLAGS_AT:02x} distribution moved; the pinned reading is stale"
    );

    // The headline: the bit is *not* clear across Pulse, so acting on it is a Pulse
    // behaviour change, not a Pure-only fix. Its meaning is not decided here.
    assert_eq!(
        sweep.pre_swizzled.len(),
        PSP_PRE_SWIZZLED,
        "the number of pre-swizzle-flagged Pulse PSP textures moved"
    );

    // And it cannot be gated away by format version: both generations carry it.
    assert_eq!(
        sweep.flagged_versions(),
        BTreeSet::from([4, 6]),
        "the flag no longer spans both format versions, which is what makes a \
         `version <= 4` gate insufficient"
    );

    // Not font atlases but ship liveries and weapon effects: a disagreement with
    // the recorded claim, not an extension of it.
    let names = sweep.flagged_names();
    for expected in [
        "Z:/WipeoutPSP/X2/Data/Ships/Feisar/Textures/liveryFeisar1_leftwing.tga",
        "Z:/WipeoutPSP/X2/Data/Weapons/Textures/pulse_bomb.tga",
    ] {
        assert!(
            names.contains(expected),
            "{expected} is no longer among the flagged textures"
        );
    }
    assert!(
        !names.iter().any(|n| n.contains(".fnt")),
        "a font atlas turned up among the flagged nodes; these are .vex textures"
    );

    // The one asset the two readings compare on: `pure-status.md` records Pulse's
    // Feisar as eight textures reading `0xe4`; there are eight and they read
    // `0xe5`. Pinned as a measurement so whoever settles it has it in front of them.
    let feisar = sweep
        .pre_swizzled
        .iter()
        .find(|f| {
            f.name
                .as_deref()
                .is_some_and(|n| n.contains("Ships/Feisar"))
        })
        .map(|f| f.file.clone())
        .expect("a flagged Feisar ship texture");
    assert_eq!(
        sweep.per_file[&feisar],
        (8, 8),
        "{feisar}: the Feisar model no longer has eight Texture nodes, all flagged"
    );
    assert!(
        sweep
            .pre_swizzled
            .iter()
            .filter(|f| f.file == feisar)
            .all(|f| f.flags == 0xe5),
        "{feisar}: the ship model's flags are no longer 0xe5"
    );
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd; run with `just test-data`"]
fn pulse_ps2_sets_bit_zero_too_on_a_pressing_that_embeds_no_texels() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let sweep = sweep_source(&path);
    sweep.report("pulse-ps2-eu");
    assert_sweep_covered(&sweep, "pulse-ps2-eu");

    assert_eq!(sweep.versions(), PS2_VERSIONS);
    assert_eq!(sweep.texture_nodes, PS2_TEXTURE_NODES);
    assert_eq!(
        sweep.histogram(),
        PS2_FLAGS,
        "the +0x{FLAGS_AT:02x} distribution moved; the pinned reading is stale"
    );
    assert_eq!(
        sweep.pre_swizzled.len(),
        PS2_PRE_SWIZZLED,
        "the number of pre-swizzle-flagged Pulse PS2 textures moved"
    );

    // The 33 version-4 files are the PSP pressing's assets with an identical flag
    // distribution: two pressings reading one field the same way shows it is read.
    let v4: Vec<(u32, u8, usize)> = sweep
        .histogram()
        .into_iter()
        .filter(|&(version, _, _)| version == 4)
        .collect();
    let psp_v4: Vec<(u32, u8, usize)> = PSP_FLAGS
        .iter()
        .copied()
        .filter(|&(version, _, _)| version == 4)
        .collect();
    assert_eq!(
        v4, psp_v4,
        "the legacy version-4 assets no longer read identically on both pressings"
    );
}
