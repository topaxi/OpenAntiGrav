//! Which Vita material files write `1 - v` into the diffuse coordinate, read off their vertex
//! programs.
//!
//! A `.rcsmaterial` holds one vertex program per permutation. On 2048's disc exactly four files
//! carry the flip, and **every** vertex program of each does (`Amphiseum`'s `track_surface` and
//! `track_surface_no_emissive`, `Modesto Heights`' `track_wall`, `Tech De Ra`'s `track_surface`:
//! 289 of 289 programs that declare `Uv1`), while no program of the other 565 files has it
//! (`docs/formats/2048-material-params.md`, "The track wall that writes `1 - v`"). The
//! instruction is one `add.f32`, `Uv1.y` negated into the output (or into a register a `bw`
//! copies out) plus a literal that reads 1.0: `add.f32 o8.-y, -pa8.-y, sa42.-x`.
//!
//! Only that one shape is read, by its bits: the `nmad32` class, function `add`, first source the
//! `pa` register of the `Uv1` attribute negated, write mask `y`, source lane `y`. It is the
//! operand layout `scripts/vita-usse.py` lists, narrowed to what the question needs; a program
//! that negates `Uv1.y` another way reads as not flipping, the direction that leaves a
//! coordinate alone.

use super::{Category, Program, usse};

/// The attribute the diffuse coordinate is.
const UV: &str = "Uv1";

/// Whether the vertex program `p` (bytes `program`) negates the `y` lane of its `Uv1` attribute
/// with an `add`. `false` when it declares no `Uv1`.
fn program_flips(program: &[u8], p: &Program) -> Option<bool> {
    let uv = p
        .parameters
        .iter()
        .find(|q| q.category == Category::Attribute && q.name == UV)?;
    Some(usse::words(program, &p.primary).any(|w| negates_lane_y(w, uv.resource)))
}

/// `add.f32 <dst>.-y, -pa<resource>.-y, <src2>`: an `nmad32` word of function 1 (`add`), source 1
/// bank `pa` (selector 2, no extension bit) numbered `resource` (the field counts register
/// pairs), negated (modifier 1 or 3), writing lane `y` alone from lane `y`.
fn negates_lane_y(word: u64, resource: u32) -> bool {
    if word & 0xf800_0000_0000_0000 != 0x0800_0000_0000_0000 {
        return false;
    }
    let field = |lo: u32, n: u32| ((word >> lo) & ((1 << n) - 1)) as u32;
    let swizzle = field(15, 7) | (field(34, 2) << 7) | (field(50, 1) << 9) | (field(53, 2) << 10);
    field(12, 3) == 1
        && field(49, 1) == 0
        && field(30, 2) == 2
        && field(6, 6) << 1 == resource
        && matches!(field(37, 2), 1 | 3)
        && field(39, 4) == 0b0010
        && (swizzle >> 3) & 7 == 1
}

/// Whether a `.rcsmaterial` file (`data`) writes `1 - v` into its diffuse coordinate: at least one
/// vertex program declares `Uv1`, and every one that does negates it. `false` for a file with no
/// `GXP` vertex program, a PS4 material among them.
#[must_use]
pub fn flips_v(data: &[u8]) -> bool {
    let mut seen = false;
    for (at, parsed) in super::programs(data) {
        let Ok(p) = parsed else { continue };
        if p.is_fragment() {
            continue;
        }
        let bytes = &data[at..at + p.size];
        match program_flips(bytes, &p) {
            Some(true) => seen = true,
            Some(false) => return false,
            None => {}
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_shapes_the_disc_writes_read_as_a_flip_of_their_attribute() {
        // Modesto's `track_wall` #32 (Uv1 at resource 8): `add.f32 o8.-y, -pa8.-y, sa42.-x`.
        assert!(negates_lane_y(0x08c0_0129_b104_9115, 8));
        // The same file's #15 (Uv1 at resource 12): `add.f32 pa12.-y, -pa12.-y, sa40.-x`.
        assert!(negates_lane_y(0x08c0_092a_b184_9194, 12));
    }

    #[test]
    fn another_register_or_a_plain_move_is_not_a_flip() {
        assert!(!negates_lane_y(0x08c0_0129_b104_9115, 12));
        assert!(!negates_lane_y(0x08c0_092a_b184_9194, 8));
        // `mul.f32 o10.xy, pa0.xy, sa38.xy`, a different function of the same class.
        assert!(!negates_lane_y(0x08a4_4185_b144_0013, 0));
        // `mov.f32 o10.xy, pa16.xy`.
        assert!(!negates_lane_y(0x3880_0521_8314_0200, 16));
    }
}
