//! The fragment opcode mnemonic table, split out of `fragment.rs` under the
//! 1,000-line rule in `scripts/check-file-size.py`.
//!
//! Two public sources, and which one names an opcode matters:
//!
//! - **Mesa's `nvfx_shader.h`** (`NVFX_FP_OP_OPCODE_*`), the table this was
//!   first built from, `0x00`..`0x3c`.
//! - **RPCS3's `rpcs3/Emu/RSX/Program/Assembler/FPOpcodes.h`**
//!   (`RSX_FP_OPCODE_*`, GPLv2), which adds the RSX-only numbers Mesa lacks.
//!   Only three of those occur in shipped HD code, and those three are
//!   corroborated disc-wide (`crates/render/examples/hd_op3b_op3d_census.rs`,
//!   1,632 `.rcsmaterial` files, 76,358 fragment blocks); the rest are named
//!   off RPCS3 alone and never appear on this disc. See
//!   `docs/formats/rcsmaterial.md`.
//!
//! `0x30` and `0x32` are in neither header and stay unnamed.

/// The mnemonic for `opcode`, or `None` for one neither source names.
pub(super) fn name(opcode: u8) -> Option<&'static str> {
    Some(match opcode {
        0x00 => "NOP",
        0x01 => "MOV",
        0x02 => "MUL",
        0x03 => "ADD",
        0x04 => "MAD",
        0x05 => "DP3",
        0x06 => "DP4",
        0x07 => "DST",
        0x08 => "MIN",
        0x09 => "MAX",
        0x0a => "SLT",
        0x0b => "SGE",
        0x0c => "SLE",
        0x0d => "SGT",
        0x0e => "SNE",
        0x0f => "SEQ",
        0x10 => "FRC",
        0x11 => "FLR",
        0x12 => "KIL",
        0x13 => "PK4B",
        0x14 => "UP4B",
        0x15 => "DDX",
        0x16 => "DDY",
        0x17 => "TEX",
        0x18 => "TXP",
        0x19 => "TXD",
        0x1a => "RCP",
        0x1b => "RSQ",
        0x1c => "EX2",
        0x1d => "LG2",
        0x1e => "LIT",
        0x1f => "LRP",
        0x20 => "STR",
        0x21 => "SFL",
        0x22 => "COS",
        0x23 => "SIN",
        0x24 => "PK2H",
        0x25 => "UP2H",
        0x26 => "POW",
        0x27 => "PK4UB",
        0x28 => "UP4UB",
        0x29 => "PK2US",
        0x2a => "UP2US",
        0x2e => "DP2A",
        0x2f => "TXL",
        0x31 => "TXB",
        0x36 => "RFL",
        0x3a => "DIV",
        // `NVFX_FP_OP_OPCODE_LITEX2_NV40` in Mesa's `nvfx_shader.h`,
        // confirmed against the primary source; absent from every
        // fragment block on this disc (`hd_litex2_census.rs`, 0 of
        // 76,358) - named for completeness, not because it's needed.
        0x3c => "LIT_EX2_NV40",
        // `RSX_FP_OPCODE_DIVSQ` (`a / sqrt(b)`); confidence 84.
        0x3b => "DIVSQ",
        // `RSX_FP_OPCODE_FENCT` and `RSX_FP_OPCODE_FENCB` ("Fence T?",
        // "Fence B?", RPCS3's own hedges). Every use on this disc writes
        // destination register 63, no real destination: 59,256 of 59,256 and
        // 3,115 of 3,115. Confidence 90 on that, not on "fence".
        0x3d => "FENCT",
        0x3e => "FENCB",
        // RPCS3 alone, and none of these occurs in shipped HD code.
        0x2b => "BEM",
        0x2c => "PKG",
        0x2d => "UPG",
        0x33 => "TEXBEM",
        0x34 => "TXPBEM",
        0x35 => "BEMLUM",
        0x37 => "TIMESWTEX",
        0x38 => "DP2",
        0x39 => "NRM",
        _ => return None,
    })
}
