//! Walks every `.pob` emitter tree on both discs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! [`oag_pob::ParticleSystem::emitters`] reads an emitter record at
//! fixed offsets and follows three pointer fields into a tree, all straight
//! out of the file's own bytes. Both halves of that are unverifiable from a
//! single hand-checked file:
//!
//! 1. **The offsets are a layout, not a coincidence.** They were decoded
//!    against one asset, `WO_SHIP_COLL_SPARK_DAMAGE`. If a field is really
//!    at `+0x5c` for every authored effect, then every file's lifetimes,
//!    schedules and per-emission counts come back positive and small - and
//!    if the reading is off, the corpus is where that shows.
//! 2. **The tree walk terminates on real data.** `WO_ROCKET_EXPLO` nests
//!    children two ways at once; the parser refuses cycles and unbounded
//!    walks, and this is the check that no real file trips either.
//!
//! It also pins the two enumerations a renderer must not guess at: the draw
//! class behind [`Emitter::draw_class`] and the channel modes behind
//! [`ChannelMode`]. An unknown value in either would otherwise reach a
//! renderer as a silent no-draw.
//!
//! No values are asserted against a table written down elsewhere - that
//! would just be the transcription this test exists to make unnecessary.
//! What is asserted is that every file agrees with the *shape* the layout
//! claims.

use std::path::PathBuf;

use oag_assets::Archive;
use oag_pob::{self as pob, ChannelMode, Emitter, ParticleSystem};

/// `.pob` blobs on the PSP disc, per `docs/formats/pob.md`.
const PSP_SYSTEMS: usize = 35;

/// And on the PS2 disc's main archive.
const PS2_SYSTEMS: usize = 41;

/// And across all seven PSARC archives on the Wipeout HD/Fury disc. Higher
/// than either Pulse corpus because HD authors per-weapon variants Pulse does
/// not, and because four names appear in two archives each.
const HD_SYSTEMS: usize = 88;

/// The disc this project's HD work reads, per `data/README.md`.
const HD_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Every `SYSP` blob in `archive`, decompressed, in directory order.
///
/// Found by content rather than by name: the entry names are hashes, and
/// scanning for the magic is what makes this work unchanged on the PS2
/// archive, whose nine extra effects have no PSP counterpart.
fn particle_systems(archive: &mut Archive) -> Vec<(usize, Vec<u8>)> {
    let count = archive.directory().entries.len();
    let mut out = Vec::new();
    for index in 0..count {
        let Ok(head) = archive.peek(index, 4) else {
            continue;
        };
        if !pob::looks_like_particle_system(&head) {
            continue;
        }
        let Ok(blob) = archive.read(index) else {
            continue;
        };
        out.push((index, blob));
    }
    out
}

/// Everything one emitter must satisfy for the layout to be a layout.
///
/// `blend_classes` is the set the corpus is allowed to author, and is the one
/// predicate that is not shared: Pulse uses 1-3 and HD adds a 4. Passing it in
/// rather than widening the range for everyone keeps the Pulse assertion as
/// tight as it was - a 4 appearing on a Pulse disc is still a failure.
fn check(system: &str, emitter: &Emitter, blend_classes: std::ops::RangeInclusive<u32>) {
    let where_ = format!("{system}/{} (+{:#x})", emitter.name, emitter.offset);

    assert!(
        emitter.draw_class().is_some(),
        "{where_}: render mode {} is past the blend table",
        emitter.render_mode
    );
    assert!(
        blend_classes.contains(&emitter.blend_class),
        "{where_}: blend class {}",
        emitter.blend_class
    );
    // Shapes 1, 2 and 8 exist in the dispatch and are unread; a value
    // outside the whole switch would mean the field is not the shape.
    assert!(
        emitter.shape <= 8,
        "{where_}: emitter shape {}",
        emitter.shape
    );

    for (name, channel) in [
        ("size", &emitter.size),
        ("alpha", &emitter.alpha),
        ("rotation", &emitter.rotation_speed),
        ("emission", &emitter.emission_scale),
    ] {
        assert!(
            !matches!(channel.mode, ChannelMode::Unknown(_)),
            "{where_}: {name} channel mode {:?}",
            channel.mode
        );
        // Keyframe times are a normalized age, ascending.
        let mut previous = f32::NEG_INFINITY;
        for &(time, _) in &channel.keys {
            assert!(
                (0.0..=1.0).contains(&time) && time >= previous,
                "{where_}: {name} channel key time {time}"
            );
            previous = time;
        }
    }

    // A schedule the interpreter can run: an emitter that emitted zero
    // particles at a zero interval would spin.
    assert!(
        emitter.duration_ticks >= 0.0 && emitter.duration_ticks.is_finite(),
        "{where_}: duration {}",
        emitter.duration_ticks
    );
    assert!(
        emitter.interval_ticks.0 >= 1 && emitter.interval_ticks.1 >= emitter.interval_ticks.0,
        "{where_}: interval {:?}",
        emitter.interval_ticks
    );
    assert!(
        emitter.per_emission.0 >= 1 && emitter.per_emission.1 >= emitter.per_emission.0,
        "{where_}: per emission {:?}",
        emitter.per_emission
    );
    assert!(
        emitter.lifetime_ticks.0 >= 0 && emitter.lifetime_ticks.1 >= 0,
        "{where_}: lifetime {:?}",
        emitter.lifetime_ticks
    );
    assert!(
        emitter.live_cap > 0,
        "{where_}: live cap {}",
        emitter.live_cap
    );
    assert!(
        emitter.speed_per_tick.0.is_finite() && emitter.speed_per_tick.1 >= 0.0,
        "{where_}: speed {:?}",
        emitter.speed_per_tick
    );
    assert!(
        emitter.cone_degrees >= 0.0 && emitter.cone_degrees <= 180.0,
        "{where_}: cone {} degrees",
        emitter.cone_degrees
    );
    assert!(
        emitter.atlas_grid.0 >= 1 && emitter.atlas_grid.1 >= 1 && emitter.atlas_frames >= 1,
        "{where_}: atlas {:?} x {}",
        emitter.atlas_grid,
        emitter.atlas_frames
    );
    assert!(
        emitter.child_spawn_probability >= 0.0 && emitter.child_spawn_probability <= 1.0,
        "{where_}: child probability {}",
        emitter.child_spawn_probability
    );
}

/// Parses every system in one archive and reports the corpus totals.
///
/// `assert_no_texture` is the PS2 negative control folded into this same
/// pass rather than a second full archive read - see
/// [`no_ps2_particle_system_embeds_a_texture`]'s own doc comment for why
/// that used to be a separate ~15s decompress of the whole 377 MiB WAD.
fn walk_archive(label: &str, spec: &str, expected: usize, assert_no_texture: bool) {
    let mut archive = Archive::open(spec).expect("open archive");
    let blobs = particle_systems(&mut archive);
    assert_eq!(
        blobs.len(),
        expected,
        "{label}: found {} SYSP blobs, expected {expected}",
        blobs.len()
    );
    walk_systems(label, &blobs, 1..=3, true, assert_no_texture);
}

/// Everything asserted about a corpus of blobs, whichever disc they came off.
///
/// `uniform_looping` is whether every emitter of a tree agrees with its root
/// about [`pob::flags::LOOPING`]. True on both Pulse discs, false on HD - see
/// [`every_hd_particle_system_walks_the_same_way_byte_swapped`].
///
/// `assert_no_texture` is PS2's negative control: every emitter of every
/// system must have no [`ParticleSystem::embedded_texture`] at all. `false`
/// on PSP (checked separately and more specifically, root-emitter by
/// root-emitter, by
/// [`every_psp_root_emitter_texture_is_where_the_layout_says`]) and on HD
/// (never checked here - HD ships separate `.gtf` sprites instead, see
/// `oag_pob::texture`'s module doc).
fn walk_systems(
    label: &str,
    blobs: &[(usize, Vec<u8>)],
    blend_classes: std::ops::RangeInclusive<u32>,
    uniform_looping: bool,
    assert_no_texture: bool,
) {
    let (mut emitters, mut trees, mut deepest) = (0usize, 0usize, (0usize, String::new()));
    for (index, blob) in blobs {
        let system = ParticleSystem::parse(blob)
            .unwrap_or_else(|error| panic!("{label} entry {index}: {error}"));
        let parsed = system
            .emitters(blob)
            .unwrap_or_else(|error| panic!("{label} {}: {error}", system.name));

        assert_eq!(
            parsed[0].name, system.name,
            "{label}: the root emitter's name is the resource's own"
        );
        // `+0x04` counts from *after* the 32-byte name, so a file is always
        // longer than the field says by exactly the header, the slot table and
        // that name. Asserted on every disc because a survey that compared the
        // field to the file length instead read the constant slack as an
        // HD-only disagreement - see `docs/formats/pob.md`.
        let declared = system.order.u32(blob, 0x04) as usize;
        assert_eq!(
            blob.len() - declared,
            system.resource_base() + 16,
            "{label} {}: slack over the declared length",
            system.name
        );
        // `flags::LOOPING` is what its doc comment claims - a property of
        // the whole effect - only while no file mixes set and clear
        // emitters. This is the assertion that claim rests on.
        let looping = parsed[0].looping();
        for emitter in &parsed {
            assert!(
                emitter.looping() == looping || !uniform_looping,
                "{label} {}: {} disagrees with the root about LOOPING",
                system.name,
                emitter.name
            );
        }
        for emitter in &parsed {
            check(&system.name, emitter, blend_classes.clone());
            // Every tree index a record hands out must name a real record.
            for child in [emitter.death_child, emitter.particle_child]
                .into_iter()
                .flatten()
            {
                assert!(
                    child < parsed.len(),
                    "{label} {}: child {child}",
                    system.name
                );
            }
            assert!(
                !assert_no_texture || system.embedded_texture(blob, emitter).is_none(),
                "{label} {}/{}: unexpectedly carries an embedded texture",
                system.name,
                emitter.name
            );
        }

        if parsed.len() > deepest.0 {
            deepest = (parsed.len(), system.name.clone());
        }
        emitters += parsed.len();
        trees += 1;
    }

    println!(
        "{label}: {trees} systems, {emitters} emitters, largest {} with {}",
        deepest.1, deepest.0
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_psp_particle_system_walks_its_emitter_tree() {
    let Some(image) = image("pulse-psp-usa.chd") else {
        return;
    };
    walk_archive(
        "psp",
        &format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()),
        PSP_SYSTEMS,
        false,
    );
}

/// The PSP root emitters with no embedded texture at their positional
/// offset: **none**. There used to be six (`WO_PLASMA_FLASH`, `WO_RAIN`,
/// `WO_SNOW`, `WO_LEACHBEAM_CHARGING`, `WO_REPULSER`, `WO_ROCKET_FLARE`) and
/// the reader refused them because their headers are 4 bits per pixel, which
/// it did not accept until 2026-10-01 (`oag_pob::texture`, "Four bits per
/// pixel"). A file entering this set is a real change to what the corpus
/// measures, not noise - update this list and `pob.md` together.
const PSP_ROOTS_WITH_NO_TEXTURE: &[&str] = &[];

/// [`oag_pob::ParticleSystem::embedded_texture`] over the whole PSP
/// corpus: every root emitter either has one, or is on
/// [`PSP_ROOTS_WITH_NO_TEXTURE`] - nothing else silently returns `None`. A
/// found texture's own bytes must be internally consistent (level 0 no
/// larger than the declared pixel region, which [`pob::texture::parse_at`]
/// already refuses, so this just re-asserts the dimensions are sane on
/// every real file rather than only the one this was decoded against).
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_psp_root_emitter_texture_is_where_the_layout_says() {
    let Some(image) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archive =
        Archive::open(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display())).expect("open");
    let blobs = particle_systems(&mut archive);
    assert_eq!(blobs.len(), PSP_SYSTEMS);

    let mut with_texture = Vec::new();
    let mut without_texture = Vec::new();
    for (_, blob) in &blobs {
        let system = ParticleSystem::parse(blob).expect("parse");
        let parsed = system.emitters(blob).expect("emitters");
        let root = &parsed[0];
        match system.embedded_texture(blob, root) {
            Some(texture) => {
                assert!(
                    texture.width >= 8 && texture.height >= 8,
                    "{}: implausibly small root texture {}x{}",
                    system.name,
                    texture.width,
                    texture.height
                );
                assert_eq!(
                    texture.indices.len(),
                    usize::from(texture.width)
                        * usize::from(texture.height)
                        * usize::from(texture.bits_per_pixel)
                        / 8,
                    "{}: level 0 stored byte count",
                    system.name
                );
                with_texture.push(system.name.clone());
            }
            None => without_texture.push(system.name.clone()),
        }
    }
    without_texture.sort();
    let mut expected: Vec<String> = PSP_ROOTS_WITH_NO_TEXTURE
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    expected.sort();
    assert_eq!(
        without_texture, expected,
        "which root emitters have no embedded texture is a measurement, not a tolerance"
    );
    println!(
        "psp: {} of {PSP_SYSTEMS} root emitters carry an embedded texture",
        with_texture.len()
    );
}

/// Every embedded texture's two pointer words (`+0x10` pixels, `+0x14`
/// palette) are entries of the file's own pointer-fixup table - the
/// structural proof that both are offsets from the resource base, which the
/// loader turns into pointers, rather than offsets from the blob's start.
/// See `oag_pob::texture`'s "from the resource base" section.
///
/// Also checks what the corrected reading draws: palette entry 0 is black
/// on every additive (blend class 2) emitter's sprite. An additive sprite's
/// field adds nothing only if its background index is black, and the old,
/// unbased reading put a navy `(11, 30, 53)` there.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_psp_texture_pointer_is_a_fixup_site() {
    let Some(image) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archive =
        Archive::open(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display())).expect("open");
    let blobs = particle_systems(&mut archive);
    assert_eq!(blobs.len(), PSP_SYSTEMS);

    let (mut textures, mut additive) = (0, 0);
    for (_, blob) in &blobs {
        let system = ParticleSystem::parse(blob).expect("parse");
        let sites: Vec<usize> = system.slots.iter().flatten().map(|&s| s as usize).collect();
        for emitter in system.emitters(blob).expect("emitters") {
            let Some(texture) = system.embedded_texture(blob, &emitter) else {
                continue;
            };
            textures += 1;
            // Relative to the resource base, the header sits at
            // `emitter.offset + EMITTER_LEN`.
            let header = emitter.offset + pob::EMITTER_LEN;
            for (field, what) in [(0x10, "pixels"), (0x14, "palette")] {
                assert!(
                    sites.contains(&(header + field)),
                    "{} / {}: the {what} word at +{field:#x} is not a fixup site",
                    system.name,
                    emitter.name
                );
            }
            if emitter.blend_class == 2 {
                additive += 1;
                // Black (to within 2: `WO_REPULSER`'s is `2, 2, 2`), or
                // transparent (alpha 0 or 1): the field around the picture
                // adds nothing. `WO_LEACHBEAM_CHARGING`'s is a transparent
                // blue.
                assert!(
                    texture.palette[..3].iter().all(|&c| c <= 2) || texture.palette[3] <= 1,
                    "{} / {}: an additive sprite's index 0 adds light",
                    system.name,
                    emitter.name
                );
            }
        }
    }
    println!(
        "psp: {textures} embedded texture(s), {additive} on additive emitters, every pointer a fixup site"
    );
    assert!(textures >= 29, "at least the 29 root textures");
}

// The negative control `docs/formats/pob.md` draws on - PS2's `.pob`s embed
// nothing at all, at any emitter's positional offset - is folded into
// `every_ps2_particle_system_walks_the_same_way` above (`assert_no_texture`)
// rather than a second full archive walk. See that test's doc comment.

/// The rubric's "a second binary is worth more than a second reading of the
/// first": the PS2 port's own 41 systems, through the same parser, with no
/// adjustment. Also the negative control `oag_pob::texture`'s module
/// doc cites: every emitter of every PS2 system is asserted to carry no
/// embedded texture at all, folded into this same walk rather than a
/// second ~15-18s decompress of the whole 377 MiB `WADS2.WAD` - see
/// [`walk_systems`]'s `assert_no_texture` parameter.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn every_ps2_particle_system_walks_the_same_way() {
    let Some(image) = image("pulse-ps2-eu.chd") else {
        return;
    };
    walk_archive(
        "ps2",
        &format!("{}:54748/WADS2.WAD", image.display()),
        PS2_SYSTEMS,
        true,
    );
}

/// Every `.psarc` archive spec on the HD disc image at `path`, `image:archive`
/// form, ready for [`oag_assets::psarc::Archive::open`]. Shared by both HD
/// tests so there is one place that walks the ISO's own file list.
fn hd_psarc_specs(path: &std::path::Path) -> Vec<String> {
    let mut disc = oag_disc::DiscImage::open(path).expect("open image");
    disc.entries()
        .expect("iso walk")
        .iter()
        .filter(|entry| !entry.is_directory && entry.path.to_ascii_lowercase().ends_with(".psarc"))
        .map(|entry| format!("{}:{}", path.display(), entry.path))
        .collect()
}

/// The third binary, and the one that moves the format's byte order out of a
/// reading and into a run: Wipeout HD/Fury's 88 systems, big-endian, through
/// this same parser with no offset changed.
///
/// It asserts more than "they parse". Every predicate [`check`] applies to the
/// PSP and PS2 corpora - positive schedules, in-range channel modes, a draw
/// class the blend table has, an atlas of at least one frame - has to hold on
/// records read the other way round, and a byte-order bug shows up there rather
/// than at the magic.
///
/// # Two things HD authors that Pulse does not
///
/// Both are content, not byte order, and both are pinned here as measurements
/// rather than waved through as tolerances:
///
/// 1. **`blend_class` 4**, on the seven emitters of `WO_NITRO_SHIP_DEATH` and
///    nowhere else. Pulse only ever writes 1-3.
/// 2. **`LOOPING` is per-emitter.** The flag's doc comment calls it a property
///    of the whole effect on the strength of the two Pulse corpora; four HD
///    systems mix set and clear emitters inside one tree, so on that disc it
///    is not. This test is where that claim was found to be disc-specific.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_hd_particle_system_walks_the_same_way_byte_swapped() {
    let Some(path) = image(HD_IMAGE) else { return };

    let specs = hd_psarc_specs(&path);
    let mut blobs = Vec::new();
    for spec in &specs {
        let mut archive = oag_assets::psarc::Archive::open(spec).expect("the archive opens");
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|entry| entry.to_ascii_lowercase().ends_with(".pob"))
            .cloned()
            .collect();
        for entry in paths {
            let blob = archive.read_path(&entry).expect("the entry reads");
            assert!(
                pob::looks_like_particle_system(&blob),
                "{entry}: not a particle system at all"
            );
            assert_eq!(
                pob::byte_order(&blob),
                oag_formats::ByteOrder::Big,
                "{entry}: a PS3 blob that is not big-endian"
            );
            blobs.push((blobs.len(), blob));
        }
    }

    assert_eq!(
        blobs.len(),
        HD_SYSTEMS,
        "hd: found {} PSYS blobs, expected {HD_SYSTEMS}",
        blobs.len()
    );
    // 1..=4 and per-emitter LOOPING are the two ways HD's *content* differs;
    // every other predicate is asserted exactly as it is on Pulse. HD's own
    // texture mechanism (separate `.gtf` PSARC entries) is not this
    // module's concern, so `assert_no_texture` is false here - a positional
    // hit on HD would be surprising but is not this test's job to police.
    walk_systems("hd", &blobs, 1..=4, false, false);

    let mut mixed = Vec::new();
    let mut fourth = Vec::new();
    for (_, blob) in &blobs {
        let system = ParticleSystem::parse(blob).expect("parse");
        let parsed = system.emitters(blob).expect("emitters");
        let looping = parsed[0].looping();
        if parsed.iter().any(|e| e.looping() != looping) {
            mixed.push(system.name.clone());
        }
        fourth.extend(
            parsed
                .iter()
                .filter(|e| e.blend_class == 4)
                .map(|e| format!("{}/{}", system.name, e.name)),
        );
    }
    mixed.sort();
    mixed.dedup();
    assert_eq!(
        mixed,
        [
            "WO_CANNON_MUZZLEFLASH",
            "WO_MISSILE_HEAD",
            "WO_QUAKE_DETONATOR_TRAILS",
            "WO_ROCKET_FLARE",
        ],
        "which systems mix LOOPING is a measurement, not a tolerance"
    );
    assert_eq!(
        fourth.len(),
        7,
        "blend class 4 is confined to WO_NITRO_SHIP_DEATH: {fourth:?}"
    );
    assert!(
        fourth.iter().all(|e| e.starts_with("WO_NITRO_SHIP_DEATH/")),
        "blend class 4 escaped WO_NITRO_SHIP_DEATH: {fourth:?}"
    );
}

/// An emitter's own texture-path string, resolved through whichever slot
/// names `emitter.offset + 0x4c4` as its fixup site - this project already
/// reads that field as "an emitter's own texture slot" on PSP, see
/// `docs/ghidra/functions/psp-pulse-usa/particle-system.md`'s
/// collision-spark section. `None` if no slot names that site, or the
/// target does not decode as a NUL-terminated ASCII string.
fn own_texture_path(system: &ParticleSystem, blob: &[u8], emitter: &Emitter) -> Option<String> {
    /// The fixed offset, within any emitter record, this project already
    /// reads as the emitter's own texture reference.
    const OWN_TEXTURE_FIELD: u32 = 0x4c4;

    let site = emitter.offset as u32 + OWN_TEXTURE_FIELD;
    let index = system.slots.iter().position(|slot| *slot == Some(site))?;
    let relative = system.resolve_slot(blob, index).ok().flatten()?;
    let target = system.resource_base() + pob::NAME_LEN + relative;
    let end = blob.get(target..)?.iter().position(|&b| b == 0)?;
    let raw = &blob[target..target + end];
    (!raw.is_empty() && raw.iter().all(|&b| b.is_ascii_graphic() || b == b' '))
        .then(|| String::from_utf8_lossy(raw).into_owned())
}

/// HD does not embed sprite pixels the way PSP does (see
/// `oag_pob::texture`'s module doc), but every emitter still names its
/// own texture through the identical `+0x4c4` field, and HD ships the
/// sprites as separate `/data/psys/tex/*.gtf` PSARC entries rather than a
/// hash the way PSP's `.wad` does - so a name resolves by simple basename
/// lookup, no hash table needed.
///
/// This test does not decode a `.gtf` or wire anything into a draw path -
/// it answers only "does the name resolve", the bounded deliverable this
/// was asked for. See `docs/formats/pob.md`'s "HD names its own texture
/// the same way Pulse does" for the write-up.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn hd_own_textures_mostly_resolve_to_a_shipped_gtf() {
    let Some(path) = image(HD_IMAGE) else { return };
    let specs = hd_psarc_specs(&path);

    let mut gtf_basenames = std::collections::HashSet::new();
    let mut blobs = Vec::new();
    for spec in &specs {
        let mut archive = oag_assets::psarc::Archive::open(spec).expect("the archive opens");
        for entry in archive.paths().to_vec() {
            let lower = entry.to_ascii_lowercase();
            if lower.contains("/psys/tex/")
                && lower.ends_with(".gtf")
                && let Some(basename) = lower.rsplit('/').next()
            {
                gtf_basenames.insert(basename.to_string());
            }
            if lower.ends_with(".pob") {
                let blob = archive.read_path(&entry).expect("the entry reads");
                blobs.push(blob);
            }
        }
    }
    assert!(
        !gtf_basenames.is_empty(),
        "hd: found no /data/psys/tex/*.gtf entries at all"
    );

    let mut resolved = std::collections::BTreeSet::new();
    let mut unresolved = std::collections::BTreeSet::new();
    let mut no_own_reference = 0usize;
    for blob in &blobs {
        let system = ParticleSystem::parse(blob).expect("parse");
        let parsed = system.emitters(blob).expect("emitters");
        for emitter in &parsed {
            let Some(path) = own_texture_path(&system, blob, emitter) else {
                no_own_reference += 1;
                continue;
            };
            let basename = path
                .replace('\\', "/")
                .rsplit('/')
                .next()
                .unwrap_or(&path)
                .to_ascii_lowercase();
            let Some(stem) = basename.strip_suffix(".tga") else {
                continue;
            };
            let gtf = format!("{stem}.gtf");
            if gtf_basenames.contains(&gtf) {
                resolved.insert(gtf);
            } else {
                unresolved.insert(gtf);
            }
        }
    }

    println!(
        "hd: {} distinct texture name(s) resolved to a shipped .gtf, {} unresolved, \
         {no_own_reference} emitter(s) with no own-texture reference at all",
        resolved.len(),
        unresolved.len()
    );
    assert_eq!(
        no_own_reference, 0,
        "hd: every emitter named its own texture in the session this was measured - \
         a nonzero count here is a real change, not noise"
    );
    // The exact unresolved set is asserted, not just its size, per this
    // project's own "measurement, not a tolerance" convention -
    // `vandergraf_balls_1024x1024.gtf` ships under a size-renamed sibling
    // (`vandergraf_balls_1024x512.gtf`/`_missile.gtf`, both present) and
    // `plasma_8x8_1024x1024.gtf` does not ship under any name.
    assert_eq!(
        unresolved.into_iter().collect::<Vec<_>>(),
        [
            "plasma_8x8_1024x1024.gtf".to_string(),
            "vandergraf_balls_1024x1024.gtf".to_string(),
        ],
        "which HD texture names fail to resolve is a measurement, not a tolerance"
    );
}

/// The animated-attribute records (`+0x93c`/`+0x940`) of the PSP corpus: eleven, on eleven
/// emitters, ten of them selector 2 (the extent's co-factor) and `WO_REPULSER_BLAST`'s
/// selector 5. `WO_BOMB_SMOKERING`'s root is the one a live detonation measured: a keyed
/// `1 + 1 * (0.0068 .. 1)` that widens its spawn ring from 12.94 to 23.3 units.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_psp_corpus_authors_eleven_attribute_animations() {
    let Some(image) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archive =
        Archive::open(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display())).expect("open");
    let mut selectors = Vec::new();
    let mut smoke = None;
    for (_, blob) in particle_systems(&mut archive) {
        let system = ParticleSystem::parse(&blob).expect("parse");
        for emitter in system.emitters(&blob).expect("emitters") {
            assert_eq!(
                emitter.attribute_animations.len(),
                usize::try_from(emitter.animated_attributes).unwrap_or(0),
                "{} / {}: every counted record is read",
                system.name,
                emitter.name
            );
            for animation in &emitter.attribute_animations {
                selectors.push(animation.selector);
                if system.name == "WO_BOMB_SMOKERING" && emitter.name == "WO_BOMB_SMOKERING" {
                    smoke = Some(animation.clone());
                }
            }
        }
    }
    selectors.sort_unstable();
    assert_eq!(selectors, [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 5]);
    let smoke = smoke.expect("the smoke ring's record");
    assert_eq!((smoke.channel.lo, smoke.channel.hi), (1.0, 2.0));
    assert_eq!(smoke.channel.keys.len(), 2);
    assert!((smoke.channel.scaled_at(1.0) - 2.0).abs() < 1e-5);
}
