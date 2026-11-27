//! One number for "the fragment program a material draws with", on each platform.
//!
//! The key is FNV-1a over the code bytes only, so two materials that compile the same
//! program group together whatever their names, parameters or textures
//! (`docs/rendering/shader-census.md`, "The key").

/// FNV-1a, 64 bit: stable across toolchains, unlike `DefaultHasher`.
fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

const SEED: u64 = 0xcbf2_9ce4_8422_2325;

/// HD (RSX): the fragment block at `at`, its code with every slot a parameter patches at
/// draw time zeroed. `None` where the block does not decode.
#[must_use]
pub fn hd_fragment(file: &[u8], at: usize) -> Option<u64> {
    let code = crate::rcsmaterial::fragment::Program::unpatched_code(file, at)?;
    Some(fnv1a(SEED, &code))
}

/// Every program of one stage of a file, as one key: each program's code hashed, the
/// hashes sorted and deduplicated, then hashed in order. `None` for a file with none.
fn set_key(mut each: Vec<u64>) -> Option<u64> {
    each.sort_unstable();
    each.dedup();
    (!each.is_empty()).then(|| {
        each.iter()
            .fold(SEED, |hash, one| fnv1a(hash, &one.to_le_bytes()))
    })
}

/// 2048 (Vita): every fragment `GXP` program of a `.rcsmaterial`, code and literals.
#[must_use]
pub fn gxp_fragments(file: &[u8]) -> Option<u64> {
    let each = crate::gxp::programs(file)
        .into_iter()
        .filter_map(|(at, program)| {
            let program = program.ok().filter(crate::gxp::Program::is_fragment)?;
            let body = &file[at..at + program.size];
            let code = [program.secondary, program.primary]
                .iter()
                .fold(SEED, |hash, region| {
                    fnv1a(hash, &body[region.start..region.end])
                });
            Some(program.literals.iter().fold(code, |hash, (word, value)| {
                fnv1a(fnv1a(hash, &word.to_le_bytes()), &value.to_le_bytes())
            }))
        })
        .collect();
    set_key(each)
}

/// Omega (PS4): every pixel-stage `Shdr` program of a `.rcsmaterial`.
#[must_use]
pub fn gcn_pixels(file: &[u8]) -> Option<u64> {
    let each = crate::gcn::scan(file)
        .iter()
        .filter_map(|(_, program)| {
            let program = program.as_ref().ok()?;
            (program.stage == crate::gcn::Stage::Pixel).then(|| {
                let length = (program.info >> 8) as usize;
                fnv1a(SEED, &file[program.code_at..program.code_at + length])
            })
        })
        .collect();
    set_key(each)
}
