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
//! # The shader-input table, solved 2026-10-05
//!
//! **An earlier pass read two entries and stopped**, finding that "the struct's
//! shape changes with entry count". It does not: the table is `+0x30` entries
//! at the offset in `+0x34`, **`0x18` bytes each at every count**, and what
//! changes is that an entry is one of two kinds, which the earlier pass read
//! as one:
//!
//! ```text
//!   +0x00  u32   name hash, `~crc32(name)` (`crate::rcsmaterial::name_hash`)
//!   +0x04  u32   kind: 0x12 a sampler, 1 a uniform
//!   sampler:  +0x10 u32 offset of the `.gxt` path
//!   uniform:  +0x0c u32 offset of the value
//!             +0x14 u32 which pool the value is in: 0x1000 the 32-bit float
//!                       pool (`+0x20` count, `+0x24` offset), 0x2000 the half
//!                       float pool (`+0x28` count, `+0x2c` offset)
//! ```
//!
//! **Why the two-entry case looked like a clean array**: two samplers (a
//! diffuse and a lightmap) are adjacent entries of one kind, so the stride was
//! right and the hash and the `.gxt` pointer sat where the earlier pass put
//! them; a third entry of the other kind put a uniform's fields where a sampler's
//! were, and reading past it landed on `0x1000`-scale numbers - the `+0x14` pool
//! tag. Verified over all three EU packages
//! (`crates/rcs/tests/psp2_material_param_ground_truth.rs`): **52,637 sampler
//! entries, every one of them a `.gxt` the address-order scan below also finds**
//! (the 52,637 the corpus counted by string before this table was read), and
//! **15,561 of 15,565 uniform hashes are names the material's own GXP programs
//! declare in the clear**. [`Material::params`] and [`Material::samplers`] carry
//! them; see `docs/formats/2048-material-params.md`.
//!
//! The scan stays: [`Material::textures`] is the address-order list every caller
//! already uses, and [`Material::diffuse_texture`] still answers "the first
//! one" - ordinal rather than semantic - for every material the sampler names
//! do not decide.
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
//! 6,037 of the rest were `PVRTII4BPP`, which `oag_texture::gxt` refused. That
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
/// Offset of the `u16` render-state word of a Vita material header: 22,657
/// headers, whose `+0x10` word takes 17 values, all of them `state << 16` and
/// the same set the PS4 header and HD's own state word hold (`0x7c` opaque,
/// `0x39`/`0x3d` blended, `0x3e` alpha-tested).
const VITA_STATE: usize = 0x12;
/// Offset of the technique/shading-group name pointer.
const TECHNIQUE_POINTER: usize = 0x18;
/// Offset of the parameter-entry count of a material header.
const PARAM_COUNT: usize = 0x30;
/// Offset of the pointer to the parameter entries.
const PARAM_TABLE: usize = 0x34;
/// Bytes per parameter entry.
const PARAM_STRIDE: usize = 0x18;
/// The `+0x04` word of a **uniform** entry; a sampler's is `0x12`.
const PARAM_KIND_UNIFORM: u32 = 1;
/// The `+0x04` word of a **sampler** entry.
const PARAM_KIND_SAMPLER: u32 = 0x12;
/// The `+0x14` word of a uniform whose value lives in the 32-bit float pool.
const PARAM_POOL_F32: u32 = 0x1000;
/// The same word for one whose value lives in the half-float pool.
const PARAM_POOL_F16: u32 = 0x2000;
/// Offsets of the two value pools' (count, pointer) pairs in the header: the
/// float pool at `+0x20`/`+0x24`, the half pool at `+0x28`/`+0x2c`.
const POOL_F32: usize = 0x20;
const POOL_F16: usize = 0x28;
/// The most entries a header may claim.
const MAX_PARAMS: usize = 64;
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
    /// The `.gnf` bound to the material's **`lightmap` sampler**, by the
    /// sampler's own name hash - a PS4 material only; `None` on every Vita
    /// one, whose sampler table is not read (see [`read`]).
    ///
    /// Also one of [`Self::textures`], last in its ranking. A submesh's
    /// declaration carries `lightmapUV` if and only if its material has one of
    /// these: 50,042 of 226,381 submeshes over Omega's five base archives, no
    /// exception in either direction
    /// (`crates/rcs/tests/omega_declaration_ground_truth.rs`).
    pub lightmap: Option<String>,
    /// The uniforms this material instance authors, by name hash, in table
    /// order: a shader's tuning numbers and colours. See [`Param`] and
    /// [`Self::param`]. On PS4 read by `ps4_params`.
    pub params: Vec<Param>,
    /// The samplers this material instance binds, by name hash, with the
    /// `.gxt` path each is given - the entries `0x12` of the same table
    /// [`Self::params`] reads (`+0x10` the path pointer on the Vita, `.gnf`
    /// at `+0x18` on PS4). Table order. See [`Self::sampler`].
    pub samplers: Vec<(u32, String)>,
    /// The material's render-state word, where this reading has located it.
    ///
    /// **The `u16` at header `+0x22` on PS4 and `+0x12` on the Vita.** It is the same word
    /// Wipeout HD authors at `+0x10` of its own material (low two bits the
    /// transparency mode, bit 7 the `_atoc` flag) - see [`Self::transparency`].
    /// `None` where the header is too short to hold it.
    pub state: Option<u16>,
}

/// One named uniform value a material instance supplies to its shader.
///
/// **The name is `~crc32(name)`** ([`crate::rcsmaterial::name_hash`]), the
/// same hash Wipeout HD's records use, and the GXP programs name the same
/// uniforms in the clear (`oag_rcs::gxp`): 100 percent of the hashes tried
/// against those names matched, e.g. `TimeScaler` `0xfe740a78`,
/// `Emissive_UV_Offset` `0x78256a45`, `Zone_Colour1` `0x69bde6c6`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// `~crc32(name)`.
    pub hash: u32,
    /// The value's components as `f32` bit patterns - up to four, fewer where
    /// the next value or the end of its pool comes first. Bits rather than
    /// floats so a [`Material`] stays `Eq`; [`Material::param`] widens them.
    pub bits: Vec<u32>,
}

/// An IEEE binary16 widened to `f32`, exactly.
#[must_use]
pub fn half_to_f32(bits: u16) -> f32 {
    let sign = u32::from(bits >> 15) << 31;
    let exponent = u32::from((bits >> 10) & 0x1f);
    let mantissa = u32::from(bits & 0x3ff);
    let magnitude = match (exponent, mantissa) {
        (0, 0) => 0,
        (0, m) => {
            // A subnormal half is `m * 2^-24`, a normal float once shifted up.
            let shift = m.leading_zeros() - 21;
            let m = (m << shift) & 0x3ff;
            ((127 - 14 - shift) << 23) | (m << 13)
        }
        (0x1f, m) => (0xff << 23) | (m << 13),
        (e, m) => ((e + 112) << 23) | (m << 13),
    };
    f32::from_bits(sign | magnitude)
}

/// What the low two bits of a material's state word select, on the same terms
/// as [`crate::rcsmodel::Transparency`] - the word is HD's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `0`: opaque.
    Opaque,
    /// `1`: blended.
    Blended,
    /// `2`: an alpha test.
    AlphaTest,
}

impl Material {
    /// The transparency mode of [`Self::state`], or `None` where the state is
    /// unread or holds the encoding nothing on the disc uses.
    #[must_use]
    pub fn mode(&self) -> Option<Mode> {
        match self.state? & 3 {
            0 => Some(Mode::Opaque),
            1 => Some(Mode::Blended),
            2 => Some(Mode::AlphaTest),
            _ => None,
        }
    }

    /// The components of the uniform whose name hashes to `hash`, or `None`
    /// when this material authors none.
    #[must_use]
    pub fn param(&self, hash: u32) -> Option<Vec<f32>> {
        let found = self.params.iter().find(|p| p.hash == hash)?;
        Some(found.bits.iter().map(|&b| f32::from_bits(b)).collect())
    }

    /// Whether this is `fc01_dummy`, the placeholder shader: its fragment
    /// program has one uniform (`fogColour`) and no sampler, so its picture is
    /// a constant the engine fogs - and what constant is in undecoded GPU
    /// bytecode. Used by 1,716 of `altima`'s `trackZone` submeshes, which is
    /// its road and walls.
    #[must_use]
    pub fn is_placeholder(&self) -> bool {
        self.name
            .rsplit(['/', '\\'])
            .next()
            .is_some_and(|f| f.eq_ignore_ascii_case("fc01_dummy.rcsmaterial"))
    }

    /// The `.gxt` path bound to the sampler whose name hashes to `hash`.
    #[must_use]
    pub fn sampler(&self, hash: u32) -> Option<&str> {
        self.samplers
            .iter()
            .find(|(h, _)| *h == hash)
            .map(|(_, p)| p.as_str())
    }

    /// A Zone material's flat colour: the first three components of its
    /// `Zone_Colour1` to `Zone_Colour8` uniform - each `zonefc06_colour_emissive_scalar_N`
    /// file names its own `Zone_ColourN`. **Measured against the names**:
    /// `C_Red` reads `1 0 0` and `C_Pink` `1 0 1`, and the eight files read
    /// eight distinct colours. The `Zone_ColourN_Emissive` scalar beside it
    /// (`0.5` on every one) is read and left unapplied. `None` for a material
    /// without one.
    #[must_use]
    pub fn zone_colour(&self) -> Option<[f32; 3]> {
        (1..=8).find_map(|n| {
            let v = self.param(crate::rcsmaterial::name_hash(&format!("Zone_Colour{n}")))?;
            Some([*v.first()?, *v.get(1)?, *v.get(2)?])
        })
    }

    /// The first texture found that is not the lightmap - this reading's
    /// answer for "the" diffuse texture. Ordinal, not semantic; see the module
    /// doc.
    ///
    /// A material that names only a lightmap has no diffuse one. None of
    /// `tech_de_ra`'s 461 does, so on every circuit measured this is the first
    /// texture, as it always was.
    #[must_use]
    pub fn diffuse_texture(&self) -> Option<&str> {
        self.textures
            .iter()
            .map(String::as_str)
            .find(|path| Some(*path) != self.lightmap.as_deref())
    }
}

/// The sampler entries (`kind == 0x12`) of the material header at
/// `header_at`: name hash and the `.gxt` path at `+0x10`.
fn samplers_at(cpu: &[u8], header_at: usize) -> Vec<(u32, String)> {
    let count = u32_at(cpu, header_at + PARAM_COUNT).unwrap_or(0) as usize;
    let table = u32_at(cpu, header_at + PARAM_TABLE).unwrap_or(0) as usize;
    if count == 0 || count > MAX_PARAMS || table == 0 {
        return Vec::new();
    }
    (0..count)
        .filter_map(|i| {
            let at = table + i * PARAM_STRIDE;
            if u32_at(cpu, at + 4)? != PARAM_KIND_SAMPLER {
                return None;
            }
            let path = cstr_at(cpu, u32_at(cpu, at + 0x10)? as usize)?;
            ends_with_ci(&path, ".gxt").then_some((u32_at(cpu, at)?, path))
        })
        .collect()
}

/// The uniform entries of the material header at `header_at`.
///
/// Each entry is `0x18` bytes: the name hash, a kind word (`1` for a uniform,
/// `0x12` for a sampler), the value pointer at `+0x0c`, and at `+0x14` which
/// pool the value is in - `0x1000` the 32-bit float pool (`+0x20`/`+0x24`),
/// `0x2000` the half pool (`+0x28`/`+0x2c`). A value runs to the next entry's
/// value in the same pool or the pool's end, at most four components. An entry
/// whose pointer is outside its pool is dropped.
fn params_at(cpu: &[u8], header_at: usize) -> Vec<Param> {
    let count = u32_at(cpu, header_at + PARAM_COUNT).unwrap_or(0) as usize;
    let table = u32_at(cpu, header_at + PARAM_TABLE).unwrap_or(0) as usize;
    if count == 0 || count > MAX_PARAMS || table == 0 {
        return Vec::new();
    }
    let pool = |at: usize, width: usize| -> Option<(usize, usize)> {
        let n = u32_at(cpu, header_at + at)? as usize;
        let start = u32_at(cpu, header_at + at + 4)? as usize;
        let end = start.checked_add(n.checked_mul(width)?)?;
        (n > 0 && end <= cpu.len()).then_some((start, end))
    };
    let pools = [(pool(POOL_F32, 4), 4usize), (pool(POOL_F16, 2), 2usize)];
    let mut raw: Vec<(u32, usize, usize, usize)> = Vec::new();
    for i in 0..count {
        let at = table + i * PARAM_STRIDE;
        let (Some(hash), Some(kind), Some(ptr), Some(tag)) = (
            u32_at(cpu, at),
            u32_at(cpu, at + 4),
            u32_at(cpu, at + 0x0c),
            u32_at(cpu, at + 0x14),
        ) else {
            break;
        };
        let which = match tag {
            PARAM_POOL_F32 => 0,
            PARAM_POOL_F16 => 1,
            _ => continue,
        };
        if kind != PARAM_KIND_UNIFORM {
            continue;
        }
        let ptr = ptr as usize;
        if let (Some((start, end)), width) = pools[which]
            && (start..end).contains(&ptr)
        {
            raw.push((hash, width, ptr, end));
        }
    }
    raw.iter()
        .map(|&(hash, width, ptr, end)| {
            let next = raw
                .iter()
                .filter(|&&(_, w, p, e)| w == width && e == end && p > ptr)
                .map(|&(_, _, p, _)| p)
                .min()
                .unwrap_or(end);
            let take = ((next.min(end) - ptr) / width).min(4);
            let bits = (0..take)
                .map(|k| {
                    let at = ptr + k * width;
                    if width == 4 {
                        u32_at(cpu, at).unwrap_or(0)
                    } else {
                        let h = u16::from_le_bytes([cpu[at], cpu[at + 1]]);
                        half_to_f32(h).to_bits()
                    }
                })
                .collect();
            Param { hash, bits }
        })
        .collect()
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
                lightmap: None,
                params: params_at(cpu, header_at),
                samplers: samplers_at(cpu, header_at),
                state: cpu
                    .get(header_at + VITA_STATE..header_at + VITA_STATE + 2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]])),
            }
        })
        .collect()
}

mod ps4_params;

/// Bytes from the start of a PS4 sampler entry to its texture pointer.
const PS4_SAMPLER_POINTER: usize = 0x18;
/// Bytes per PS4 sampler entry. Consecutive entries of one material sit this
/// far apart (`ag_systems\ship.rcsmodel`'s three at `0x1570`, `0x1598` and
/// `0x15c0`), which is what says the layout above is a record and not a
/// coincidence of one pointer's position. Nothing reads it - the scan looks at
/// every eighth byte - so it is a fact for the tests to build fixtures by.
#[cfg(test)]
pub(super) const PS4_SAMPLER_STRIDE: usize = 0x28;

/// The sampler names that mean "this is the surface's own colour", in the
/// project's recovered `~crc32` preimages (`crate::rcsmaterial::names`).
///
/// `Texture1` is the first numbered slot, which is what every material that
/// names only one texture uses (`diffuse.rcsmaterial`: `hull_girder.gnf` under
/// `Texture1`); the rest are the diffuse spellings the same table carries.
const DIFFUSE_SAMPLERS: &[&str] = &[
    "Texture1",
    "DiffuseTexture",
    "DiffuseTexture1",
    "diffuseTexture",
    "diffuseTexture1",
    "Diffuse",
    "diffuse",
    "Colour",
    "Colour1",
];

/// Where a sampler ranks when a material names several textures: the diffuse
/// spellings first, then a hash nothing has named, then every role that is
/// known *not* to be the surface's colour (a normal, a specular, a lightmap).
///
/// **The unnamed rank sits above the known-other one on purpose**: 4 of the 5
/// entries of `ag_systems\ship.rcsmodel` and most of a circuit's props carry a
/// hash no preimage has been recovered for, and the texture they bind is the
/// colour map. Preferring a named non-diffuse role over an unnamed one would
/// bind a normal map where a diffuse belongs.
fn sampler_rank(hash: u32) -> u8 {
    match crate::rcsmaterial::names::sampler_name(hash) {
        Some(name) if DIFFUSE_SAMPLERS.contains(&name) => 0,
        None => 1,
        Some(_) => 2,
    }
}

/// Every distinct `.gnf` path an 8-byte-aligned 64-bit pointer inside
/// `[from, to)` resolves to, **diffuse-first** rather than in address order:
/// ranked by [`sampler_rank`] of the sampler-name hash the entry carries
/// [`PS4_SAMPLER_POINTER`] bytes before its pointer, and by address within a
/// rank.
///
/// **Why the order needs a rank at all:** address order is what the Vita
/// reading uses, and on a PS4 circuit it binds the wrong map on the road -
/// `track_surface_displacement2out` lists `track_de_ra_displacement_df2.gnf`
/// (a displacement map) ahead of `track_de_ra.gnf` (the `Diffuse` entry), and
/// the first frame drew the road as a white sheet with rainbow banding. Read
/// against the entry's own sampler hash the same material gives `Diffuse` ->
/// `track_de_ra.gnf`, `Normal` -> `ds_track_n.gnf` and `lightmap` -> the
/// per-object `-lmap.gnf`, which is the role a person would give each.
fn gnf_textures_in(cpu: &[u8], from: usize, to: usize) -> (Vec<String>, Option<String>) {
    let to = to.min(cpu.len());
    let mut found: Vec<(u8, usize, String)> = Vec::new();
    let mut lightmap: Option<(usize, String)> = None;
    let mut at = from.next_multiple_of(8).max(PS4_SAMPLER_POINTER);
    while at + 8 <= to {
        if let Some(target) = u64_at(cpu, at)
            && let Ok(target) = usize::try_from(target)
            && target > 0
            && cpu.get(target - 1) == Some(&0)
            && let Some(path) = cstr_at(cpu, target)
            && ends_with_ci(&path, ".gnf")
            && !found.iter().any(|(_, _, p)| *p == path)
        {
            let hash = u32_at(cpu, at - PS4_SAMPLER_POINTER);
            if hash == Some(crate::rcsmaterial::LIGHTMAP_SAMPLER) && lightmap.is_none() {
                lightmap = Some((at, path.clone()));
            }
            found.push((hash.map_or(1, sampler_rank), at, path));
        }
        at += 8;
    }
    found.sort_by_key(|&(rank, at, _)| (rank, at));
    (
        found.into_iter().map(|(_, _, path)| path).collect(),
        lightmap.map(|(_, path)| path),
    )
}

fn u64_at(cpu: &[u8], at: usize) -> Option<u64> {
    cpu.get(at..at + 8)
        .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

/// Where a PS4 file states its material count: a 64-bit word at this offset of
/// the CPU section, `461` on `tech_de_ra` and `593` on `05_ubermall`.
const PS4_FILE_MATERIAL_COUNT: usize = 0x70;
/// Where a PS4 file states the address of its material table: `count` 64-bit
/// pointers, each to a material header whose `.rcsmaterial` name pointer sits
/// [`PS4_HEADER_NAME_POINTER`] bytes in.
const PS4_FILE_MATERIAL_TABLE: usize = 0x78;
/// Bytes from the start of a PS4 material header to its name pointer.
const PS4_HEADER_NAME_POINTER: usize = 8;
/// Bytes from the start of a PS4 material header to its `u16` render-state word.
const PS4_STATE: usize = 0x22;

/// Every 8-aligned 64-bit word that points at a NUL-preceded string ending in
/// `.rcsmaterial`, with that string, in address order.
///
/// **Not the same thing as the material table.** A file can carry a name
/// pointer that no table entry reaches - `05_ubermall`'s
/// `diffuse_normal_specular.rcsmaterial` at `0x1a248` is one, 594 sites against
/// a table of 593 - which is why [`read_ps4`] indexes by the table and uses
/// these only to bound each material's extent.
fn ps4_name_sites(cpu: &[u8]) -> Vec<(usize, String)> {
    let mut sites: Vec<(usize, String)> = Vec::new();
    let mut at = 0usize;
    while at + 8 <= cpu.len() {
        if let Some(target) = u64_at(cpu, at)
            && let Ok(target) = usize::try_from(target)
            && target > 0
            && cpu.get(target - 1) == Some(&0)
            && let Some(name) = cstr_at(cpu, target)
            && ends_with_ci(&name, ".rcsmaterial")
        {
            sites.push((at, name));
        }
        at += 8;
    }
    sites
}

/// The material table the file itself states, as an index into `sites` for each
/// entry in table order, or `None` when it does not check out.
///
/// **Checked in full**: the count is plausible, the table lies inside the
/// section, and every one of its entries is a header whose name pointer is one
/// of `sites` - so a file whose header is not laid out this way falls back to
/// the address-order reading rather than reading garbage as a table.
fn ps4_table(cpu: &[u8], sites: &[(usize, String)]) -> Option<Vec<usize>> {
    let count = usize::try_from(u64_at(cpu, PS4_FILE_MATERIAL_COUNT)?).ok()?;
    let table = usize::try_from(u64_at(cpu, PS4_FILE_MATERIAL_TABLE)?).ok()?;
    if count == 0 || count > MAX_COUNT || table.checked_add(count.checked_mul(8)?)? > cpu.len() {
        return None;
    }
    (0..count)
        .map(|i| {
            let header = usize::try_from(u64_at(cpu, table + i * 8)?).ok()?;
            let site = header.checked_add(PS4_HEADER_NAME_POINTER)?;
            sites.binary_search_by_key(&site, |(at, _)| *at).ok()
        })
        .collect()
}

/// The PS4 Omega Collection's material table.
///
/// **The same container with 64-bit pointers, and a different header.** The
/// file states its table outright at [`PS4_FILE_MATERIAL_COUNT`] and
/// [`PS4_FILE_MATERIAL_TABLE`]: a count and the address of `count` pointers to
/// material headers, and a material's `.rcsmaterial` name pointer is 8 bytes
/// into its header. On all `tech_de_ra`'s 461 and `05_ubermall`'s 593 the
/// table's entries are the headers of the name sites the ASCII paths in the
/// clear lead to - the way the Vita's reading was found in the first place -
/// and a submesh's material index ([`super::PS4_MATERIAL_INDEX_BEFORE_RECORD`])
/// is an index into **this** table.
///
/// **What the earlier reading did instead, and why it was wrong for some
/// files.** It took every name site in address order as the table. Where no
/// site lies outside the table the two are the same list, which is why
/// `tech_de_ra` and the craft read right; where one does (`05_ubermall`'s
/// stray `diffuse_normal_specular.rcsmaterial` sits at `0x1a248`, thirteen
/// circuits in all), every material after it was **one index off** - a
/// submesh drew with its neighbour's material and the neighbour's texture.
/// The control that says which reading is right: a submesh's material names a
/// `lightmap` sampler if and only if its own declaration carries `lightmapUV`
/// (`crates/rcs/tests/omega_declaration_ground_truth.rs`), which holds exactly
/// on the table's order and fails on the address order in those thirteen files.
///
/// A material's own extent still runs to the next name site of any kind in
/// address order, and its textures are every `.gnf` path an 8-aligned pointer
/// in that extent resolves to, which is the scan [`textures_in`] does for the
/// Vita's `.gxt`. A file whose header does not state a table that checks out
/// falls back to address order.
///
/// The technique name is not read: no pointer to it has been placed.
#[must_use]
pub fn read_ps4(cpu: &[u8]) -> Vec<Material> {
    let sites = ps4_name_sites(cpu);
    let extent_end = |site: usize| {
        let next = sites.partition_point(|(at, _)| *at <= site);
        sites
            .get(next)
            .map_or_else(|| site.saturating_add(TAIL_SCAN_LIMIT), |(at, _)| *at)
    };
    let build = |(site, name): &(usize, String)| {
        let header = site - PS4_HEADER_NAME_POINTER;
        let (textures, lightmap) = gnf_textures_in(cpu, site + 8, extent_end(*site));
        Material {
            name: name.clone(),
            technique: None,
            textures,
            lightmap,
            params: ps4_params::params_at(cpu, header),
            samplers: ps4_params::samplers_at(cpu, header),
            state: cpu
                .get(header + PS4_STATE..header + PS4_STATE + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]])),
        }
    };
    match ps4_table(cpu, &sites) {
        Some(order) => order.into_iter().map(|i| build(&sites[i])).collect(),
        None => sites.iter().map(build).collect(),
    }
}
