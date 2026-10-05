//! HD's `MagStripArc_fp`, the arc wake's fragment program, read out of `EBOOT.elf`.
//!
//! **`#[ignore]`d and never run in CI**: it needs the decrypted executable
//! under `data/extracted/ps3/hdfury-eu/`. Run through `just test-data`.
//!
//! The program is compiled into the executable's data, named by a TOC slot
//! (`0x008b389c` holds the name string `0x007a0d28`, `0x008b38a0` the `SHO`
//! block at `0x0092f180`). Its whole combine is `rgb = vertex.rgb * tex.rgb *
//! tex.a`, `a = vertex.a * tex.a`, with no inline constant: that is what
//! `oag_fx::magstrip` implements, and why it carries no gain. This test fails if
//! the decoder, or the reading, changes: see
//! `docs/ghidra/functions/ps3-hdfury-eu/magstrip-wake.md`, "2026-10-05,
//! magstrip-arc-fp lane".

use std::path::Path;

use oag_rcs::rcsmaterial::fragment::{Program, Source};

const EBOOT: &str = "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf";

fn be32(blob: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(blob[at..at + 4].try_into().unwrap())
}

fn be64(blob: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(blob[at..at + 8].try_into().unwrap())
}

/// Maps a virtual address through the ELF64 program headers.
fn offset(blob: &[u8], vaddr: u64) -> usize {
    let phoff = be64(blob, 0x20) as usize;
    let size = usize::from(u16::from_be_bytes([blob[0x36], blob[0x37]]));
    let count = usize::from(u16::from_be_bytes([blob[0x38], blob[0x39]]));
    for index in 0..count {
        let at = phoff + index * size;
        let (file, virt, filesz) = (be64(blob, at + 8), be64(blob, at + 16), be64(blob, at + 32));
        if be32(blob, at) == 1 && (virt..virt + filesz).contains(&vaddr) {
            return (file + vaddr - virt) as usize;
        }
    }
    panic!("{vaddr:#x} is in no loaded segment");
}

#[test]
#[ignore = "needs the decrypted HD EBOOT.elf under data/extracted"]
fn the_arc_fragment_program_is_a_modulate_with_no_gain() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(EBOOT);
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        return;
    }
    let blob = std::fs::read(&path).expect("reading the executable");

    let name = be32(&blob, offset(&blob, 0x008b_389c));
    let at = offset(&blob, u64::from(name));
    assert_eq!(&blob[at..at + 15], b"MagStripArc_fp\0");
    let block = offset(&blob, u64::from(be32(&blob, offset(&blob, 0x008b_38a0))));
    let program = Program::parse(&blob, block).expect("the block decodes");

    let names: Vec<_> = program.instructions.iter().map(|i| i.name()).collect();
    assert_eq!(
        names,
        [
            Some("TEX"),
            Some("MOV"),
            Some("MUL"),
            Some("MUL"),
            Some("MUL")
        ]
    );
    assert!(
        program.instructions.iter().all(|i| i.constant.is_none()),
        "an inline constant would be a gain the port does not carry"
    );
    assert_eq!(program.instructions[0].unit, 0, "one sampler, unit 0");
    // The vertex colour (COL0 = interpolator 1) is read once, by the MOV, and
    // every later instruction works on registers: nothing scales it.
    assert_eq!(program.instructions[1].input, 1);
    assert_eq!(program.instructions[1].sources[0], Source::Input);
    assert!(
        program.instructions[2..]
            .iter()
            .all(|i| i.operands().all(|s| matches!(s, Source::Register { .. })))
    );
    // The last instruction multiplies alpha by the texel (R1.w), and the one
    // before multiplies the colour by that same texel alpha: `a * rgb` first.
    assert_eq!(program.instructions[2].swizzles[1], [3, 3, 3, 3]);
    assert!(program.instructions[4].end);
}
