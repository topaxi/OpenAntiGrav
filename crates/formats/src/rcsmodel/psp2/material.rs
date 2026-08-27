//! Wipeout 2048's material table.
//!
//! **Found the same way the buffer pointers and the vertex declaration
//! were**: `.gxt`/`.rcsmaterial` ASCII text sits in the clear inside section
//! B (52,637 and 34,388 occurrences across the 993-file corpus), so this
//! reading located the pointer field that reaches one such string, then the
//! header around it, then the file-level field that reaches the whole table -
//! the same "pair a pointer to the thing it points at" method throughout this
//! format's reading. See `docs/formats/2048-rcsmodel.md`.
//!
//! ```text
//! file header (section B, from its own start):
//!   +0x44  u32   material count
//!   +0x48  u32   offset of the material offset table: `count` little-endian
//!                u32s, each the offset of one material header
//!
//! one material header, 64 bytes:
//!   +0x00  u32   unidentified - not a hash of anything tried, see below
//!   +0x04  u32   offset of the `.rcsmaterial` path, NUL-terminated
//!   +0x08  u32   unidentified, on the same terms as +0x00
//!   +0x18  u32   offset of the technique/shading-group name, NUL-terminated
//! ```
//!
//! # Why textures are found by scanning, not by a sampler struct
//!
//! A material's shader input table (what HD calls samplers -
//! `crates/formats/src/rcsmodel/material.rs`) **does** sit right after the
//! 64-byte header, and on every material with exactly two entries it reads
//! as a clean 24-byte-stride array (name hash at `+0x04`, `.gxt` pointer at
//! `+0x14` - `0x37b5db58`, HD's own `~crc32("lightmap")`, was seen verbatim
//! in one, corroborating that the hash function transferred along with the
//! string pool it hashes). **It does not hold at four, six or eight
//! entries**: reading past the second entry at the same stride lands on
//! values with none of a hash's entropy (`0x1000`-scale numbers, not the
//! ~random 32-bit words the two-entry case shows), so the struct's shape
//! changes with entry count in a way this reading has not solved.
//!
//! Rather than ship an unverified guess for the general case, this reading
//! sidesteps the struct entirely: **every material's own byte extent -
//! from its header to the next material header found in the section, sorted
//! by address rather than table order - is scanned word-aligned for any
//! pointer that resolves to `.gxt` text**, the same discovery method that
//! found the table itself. This finds every texture a material references
//! regardless of the input table's internal shape, at the cost of not
//! knowing which shader unit (diffuse, normal, emissive, ...) each one binds
//! to - [`Material::diffuse_texture`] answers "the first one", ordinal
//! rather than semantic, the same rule HD's own reading falls back to when
//! its sampler-hash lookup does not resolve a unit
//! (`crates/render/src/mesh/rcs/skin.rs`).
//!
//! # Which submesh uses which material: a plain index, 0x18 bytes before the
//! record
//!
//! **An earlier pass of this reading recorded this as genuinely unresolved.**
//! It had checked every submesh field against every material header's own
//! identity - the two unidentified 32-bit words at `+0x00` and `+0x08`, and
//! the name pointer - and found no match anywhere in the corpus. That search
//! was sound and its conclusion was still wrong, for a reason worth recording:
//! **an index does not resemble the thing it indexes**. A submesh carrying
//! `3` matches nothing about material 3, so a search for a material's identity
//! could not have seen it however far it looked.
//!
//! Searching for an *index* instead - a small integer in `[0, count)` - finds
//! exactly one field, at [`super::MATERIAL_INDEX_BEFORE_RECORD`] bytes before
//! the record's own start. What makes that a measurement rather than a guess
//! is its **control group**, since a great many fields are accidentally small:
//!
//! - On a model whose table names exactly **one** material, the field must be
//!   `0` on every submesh. It is, on all 147 such models.
//! - On every model, the value must land inside `[0, count)`. It does, on all
//!   **244,889** submeshes the three EU packages ship - not one violation.
//! - The values must actually *use* the table. This is the discriminator that
//!   separates it from everything else: swept over every word-, half- and
//!   byte-aligned offset from `-0x60` to `+0x80`, nineteen offsets satisfy the
//!   first two conditions, and eighteen of them average **3.2 or fewer**
//!   distinct values per multi-material model. This one averages **80.3**, and
//!   86 of the 414 multi-material models name *every single* material they
//!   declare through it, 138 name at least 90%. Altima's own circuit uses 525
//!   of its 527 materials across 2,800 submeshes.
//!
//! And it reads correctly where a human can check it. `feisar2048\3`'s craft
//! resolves to `2048_ship_tech` on its hull panels, `2048_ship_paint_shiny_final`
//! (the team livery) on its painted shells, `2048_ship_plastic`, one submesh of
//! `2048_ship_lights`, and `2048_ship_glass_dg` on **exactly one** submesh.
//! That last one is the canary: the submesh a name match would have guessed
//! at (`GlassShape`) is the one this index independently lands on, without any
//! name being compared - which is what the earlier pass rightly refused to do.
//!
//! **Confidence 90, and what holds it there.** This is measured, not read out
//! of the executable. `RcsModel_Load` (`0x812f15b2`) never touches section B
//! past relocating it, and the function that actually binds a submesh to a
//! GPU texture unit was **not located**: the eleven callers of `sceGxmDraw`
//! (NID `0xBC059AFC`) in `vita-2048-eu-v104` are debug primitives, sprite
//! quads and the engine-flare effect, so this engine's mesh path reaches the
//! GPU some other way - a deferred command buffer, by hypothesis, not read.
//! Confirming this field at the instruction level is the one thing that would
//! take it past 90, and it stays open. See
//! `docs/formats/2048-rcsmodel.md`.
//!
//! The three words beside it are still unread: `-0x10` is `0xffffffff` and
//! `-0x08` is `0x00010001` on every file sampled, and `-0x20`/`-0x1c` are `0`.
//!
//! # A texture bound would not have painted either, until the same day
//!
//! **Closed, 2026-08-27.** Until that day a sweep of every distinct diffuse
//! `.gxt` 950 files' materials name (6,135 textures) found **15 decode**;
//! 6,037 of the rest were `PVRTII4BPP`, which [`crate::gxt`] refused. That
//! module now decodes it - see `oag_formats::pvrtc` - so a resolved binding paints
//! real texels. The two gaps were independent and both had to close before
//! anything was visible in a race; this note stays because "the binding is
//! read" alone was never sufficient and the next reader should not have to
//! rediscover why.
//!
//! # Confidence
//!
//! **85** on the header shape (name/technique pointers, count/table fields):
//! every field closes structurally on both files this reading was built
//! against (a 16-submesh craft, a 2,800-submesh circuit), the offset table's
//! own entries all land on headers whose name pointer resolves to
//! `.rcsmaterial` text on both (527 of 527 checked on the circuit), and the
//! scan-based texture discovery resolves a `.gxt` on 83.1% of materials
//! corpus-wide (`crates/formats/tests/psp2_rcsmodel_material_ground_truth.rs`).
//! **60** on ordinal-first being the right choice of texture when several are
//! found - HD's own reading needed the sampler-name hash to answer that
//! question correctly, and this reading does not have one. **90** on the
//! submesh-to-material index above: the confidence rubric's
//! `docs/reverse-engineering/confidence-rubric.md` puts "an exact arithmetic
//! invariant across many real files" at a ceiling of 94, and this is one -
//! 244,889 submeshes with no violation, plus a control group eighteen rival
//! offsets fail. It sits below that ceiling, not at it, because the field has
//! not been read out of the executable; see above for the route that was
//! tried and did not reach it.

/// Bytes per material header.
const HEADER_LEN: usize = 0x40;
/// Offset of the `.rcsmaterial` path pointer within a material header.
const NAME_POINTER: usize = 0x04;
/// Offset of the technique/shading-group name pointer.
const TECHNIQUE_POINTER: usize = 0x18;
/// Offset of the file-level material count, from the CPU section's own start.
const FILE_MATERIAL_COUNT: usize = 0x44;
/// Offset of the file-level material offset table pointer.
const FILE_MATERIAL_TABLE: usize = 0x48;
/// The largest material count this reading trusts without more evidence.
///
/// **Not calibrated from a small file.** An earlier version of this reading
/// capped this at 64 off a census that only sampled small props and craft -
/// altima's own circuit names 527 materials, all of them real (every offset
/// table entry lands on a header whose name resolves), so 64 was rejecting
/// a genuine track outright rather than catching a bad read. This bound
/// exists only to stop a wrong count from turning into an unbounded loop;
/// each entry is independently checked by [`header_at_valid`], so a bound
/// this generous costs nothing but a few wasted iterations on a file that
/// fails it.
const MAX_COUNT: usize = 8192;
/// How far past a material's own header this reading scans for a texture
/// pointer when it is the last material the section's own headers name -
/// generous, since real per-material data measured is a few hundred bytes.
const TAIL_SCAN_LIMIT: usize = 0x1000;

/// One entry of the material table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Material {
    /// The `.rcsmaterial` path, out of the file's own string pool.
    pub name: String,
    /// The technique/shading-group name, e.g. `ATGMaterial1SG` - Maya's own
    /// naming, on the same terms HD's chunk names are (see
    /// `crates/formats/src/rcsmodel/vertex_decl.rs`'s `Attribute::name` doc).
    pub technique: Option<String>,
    /// Every `.gxt` path reachable from a pointer inside this material's own
    /// byte extent, in ascending address order - see the module doc for why
    /// this is a scan rather than a decoded sampler array.
    pub textures: Vec<String>,
}

impl Material {
    /// The first texture found - this reading's answer for "the" diffuse
    /// texture. Ordinal, not semantic; see the module doc.
    #[must_use]
    pub fn diffuse_texture(&self) -> Option<&str> {
        self.textures.first().map(String::as_str)
    }
}

fn cstr_at(cpu: &[u8], at: usize) -> Option<String> {
    if at >= cpu.len() {
        return None;
    }
    let end = (at..cpu.len()).find(|&k| cpu[k] == 0)?;
    if end == at {
        return None;
    }
    Some(String::from_utf8_lossy(&cpu[at..end]).into_owned())
}

/// Byte-slice suffix comparison, deliberately not `str`-based: [`cstr_at`]
/// can hand back a lossily-decoded string whose byte length no longer lines
/// up with its own char boundaries, and slicing a `&str` by byte count would
/// panic on one. Slicing the raw bytes instead never can.
fn ends_with_ci(s: &str, suffix: &str) -> bool {
    let (sb, xb) = (s.as_bytes(), suffix.as_bytes());
    sb.len() >= xb.len() && sb[sb.len() - xb.len()..].eq_ignore_ascii_case(xb)
}

fn u32_at(cpu: &[u8], at: usize) -> Option<u32> {
    cpu.get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}

/// Every distinct `.gxt` path a word-aligned pointer inside `[from, to)`
/// resolves to, in ascending address order - see the module doc.
fn textures_in(cpu: &[u8], from: usize, to: usize) -> Vec<String> {
    let to = to.min(cpu.len());
    let mut out = Vec::new();
    let mut at = from;
    while at + 4 <= to {
        if let Some(target) = u32_at(cpu, at)
            && let Some(path) = cstr_at(cpu, target as usize)
            && ends_with_ci(&path, ".gxt")
            && !out.contains(&path)
        {
            out.push(path);
        }
        at += 4;
    }
    out
}

/// Whether `at` looks like a real material header, checked by resolving its
/// name pointer.
///
/// **Also requires the string to actually start where the pointer says**:
/// the byte immediately before it must be `0` (or the string sits at the
/// section's very start). Without that, a pointer landing *inside* a longer
/// path - `materials/fc06_lambert_simple.rcsmaterial` read from partway
/// through, say - still passes the suffix check and reads back as a shorter,
/// truncated name that is not any header's own. Found while scanning
/// altima's own file for headers directly (before the file-level count and
/// table below were confirmed to reach every one of its 527 correctly):
/// without this check a full-section scan finds hundreds of these
/// truncated impostors alongside the real headers.
fn header_at_valid(cpu: &[u8], at: usize) -> Option<String> {
    if at.checked_add(HEADER_LEN)? > cpu.len() {
        return None;
    }
    let name_ptr = u32_at(cpu, at + NAME_POINTER)? as usize;
    if name_ptr != 0 && *cpu.get(name_ptr.checked_sub(1)?)? != 0 {
        return None;
    }
    cstr_at(cpu, name_ptr).filter(|s| ends_with_ci(s, ".rcsmaterial"))
}

/// Reads the material table off the CPU section's own bytes, or an empty
/// table when the file-level header does not check out.
///
/// **Empty rather than an error** on a file whose header does not carry this
/// shape - a file too short to hold `+0x48`, an implausible count, or a table
/// offset outside the section - the same graceful-degradation rule
/// [`super::Model::unpaired_pointers`] and [`super::SubMesh::normals`] already
/// use elsewhere in this format: an absent table is a real, countable state
/// rather than a panic.
#[must_use]
pub fn read(cpu: &[u8]) -> Vec<Material> {
    let Some(count) = u32_at(cpu, FILE_MATERIAL_COUNT) else {
        return Vec::new();
    };
    let count = count as usize;
    if count == 0 || count > MAX_COUNT {
        return Vec::new();
    }
    let Some(table) = u32_at(cpu, FILE_MATERIAL_TABLE) else {
        return Vec::new();
    };
    let table = table as usize;

    let headers: Vec<(usize, String)> = (0..count)
        .filter_map(|i| {
            let entry_at = table.checked_add(i.checked_mul(4)?)?;
            let header_at = u32_at(cpu, entry_at)? as usize;
            let name = header_at_valid(cpu, header_at)?;
            Some((header_at, name))
        })
        .collect();
    if headers.is_empty() {
        return Vec::new();
    }

    // Extents come from every valid header's own address, sorted - not from
    // table order, which need not be ascending - so a material's scan never
    // wanders into its neighbour's data regardless of how the table lists
    // them.
    let mut starts: Vec<usize> = headers.iter().map(|(at, _)| *at).collect();
    starts.sort_unstable();
    starts.dedup();

    headers
        .into_iter()
        .map(|(header_at, name)| {
            let technique_ptr = u32_at(cpu, header_at + TECHNIQUE_POINTER).unwrap_or(0) as usize;
            let technique = cstr_at(cpu, technique_ptr);
            let next = starts
                .iter()
                .find(|&&s| s > header_at)
                .copied()
                .unwrap_or_else(|| header_at.saturating_add(TAIL_SCAN_LIMIT));
            let textures = textures_in(cpu, header_at + HEADER_LEN, next);
            Material {
                name,
                technique,
                textures,
            }
        })
        .collect()
}
