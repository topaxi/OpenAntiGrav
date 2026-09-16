//! The rate a waveform plays at: SCREAM's note-to-pitch arithmetic, ported.
//!
//! Nothing in a bank states a sample rate. What the descriptor carries is a
//! **centre note** and a **centre fine-tune** (`+0x02`, `+0x03`, one signed
//! byte each), and the engine turns those into the pitch word it hands
//! `sceSasSetPitch`, where `0x1000` is the SAS core's own rate of 44,100 Hz.
//! The chain on `psp-pulse-usa`, every link decompiled and the whole of it
//! confirmed live (190 of 190 breakpoint hits at `__sceSasSetPitch`, front
//! end and race, reproduced to the last bit by this port):
//!
//! ```text
//! Scream_StartSound   0x0898f864   note = 60, fine = 0 on every play
//! Scream_KeyOnVoice   0x0899456c   reads the descriptor's +0x02 / +0x03
//!   Scream_NoteToPitch  0x08994fec   abs() of a negative centre, then * 0x1278b >> 16
//!     Scream_Note2Pitch   0x089952c4   the table walk below
//!   Sas_QueuePitch      0x0898bf14   voice + 0x50, dirty bit 0x10
//! Sas_CommitVoices    0x0898c7e8   sceSasSetPitch(core, voice, pitch)
//! ```
//!
//! `Scream_Note2Pitch` is a 12-entry semitone table and a 128-entry fine table,
//! both Q15, both read straight off the executable at `0x08ac36dc` and
//! `0x08ac370c` (they abut: 560 bytes, 140 words, no slack). Their closed forms
//! are `floor(32768 * 2^(i/12))` and `floor(32768 * 2^(i/1536))` - checked entry
//! for entry in this module's tests - so a fine-tune step is 1/128 of a semitone
//! and a note is a MIDI note. Neither table is computed here: this crate is
//! scanned for platform transcendentals (`scripts/check-transcendentals.py`),
//! and the integer walk is what the hardware was actually given.
//!
//! **Every centre note on every Pulse and Pure disc is negative** (0 of 2,334
//! key-on descriptors positive, none `-128`), so the `0x1278b` multiply is
//! universal on this data. `0x1278b / 0x10000 = 1.154465` is read off
//! `lui a0,0x1; addiu a0,a0,0x278b` and is not explained: it is not
//! `48000 / 44100`, and not that times `2^(1/12)` either (`0x12735`). What it
//! does is put `(-86, 66)` on exactly `0x400` (11,025 Hz), `(-74, 66)` on
//! `0x800` (22,050 Hz) and `(-62, 66)` on `0x1000` (44,100 Hz), which is what
//! the disc's own banks are full of.
//!
//! What this decodes is the **playback rate at the default note**, not an
//! authored sample rate: `(note, fine)` cancelling against the centre gives
//! `0x1000`, so a deliberate transposition and a native rate are the same
//! bytes. About two thirds of the descriptors on the PSP disc land within 0.2%
//! of a standard rate (11,025 / 16,000 / 18,000 / 22,050 / 32,000 / 44,100 /
//! 48,000); the rest do not, and `speech.bnk`'s thirty-eight are all at
//! 18,002 Hz. Whether 15,569 Hz is a 16 kHz recording pitched down or a
//! recording at 15,569 Hz is not separable from the disc, and it does not
//! matter for playing it.
//!
//! See `docs/formats/psp-audio.md` and
//! `docs/ghidra/functions/psp-pulse-usa/sound.md`.

/// The pitch word at which SAS plays a voice at its own rate.
pub const SAS_PITCH_BASE: u32 = 0x1000;

/// The SAS core's output rate, and so the rate a pitch of [`SAS_PITCH_BASE`]
/// plays at. `Sas_Init` (`0x08a2ac90`) opens the core at this rate.
pub const SAS_SAMPLE_RATE: u32 = 44_100;

/// The note every play starts at. `Scream_StartSound` writes `0x3c` to the
/// handler's `+0x34` unconditionally, and nothing else in the sound module
/// writes that byte (searched: one `sb` to `0x34(...)` in `0x0898xxxx`-
/// `0x0899xxxx`, and it is this one).
pub const DEFAULT_NOTE: i32 = 60;

/// `Scream_NoteToPitch`'s multiplier for a negative centre note, `Q16`.
///
/// `lui a0,0x1; addiu a0,a0,0x278b` at `0x08995078`. Applied after the table
/// walk, `pitch * 0x1278b >> 16`. Not explained; see the module doc.
pub const NEGATIVE_CENTRE_SCALE: u32 = 0x1278b;

/// `2^(i/12) * 32768`, truncated, at `0x08ac36dc`.
pub const SEMITONE_TABLE: [u16; 12] = [
    0x8000, 0x879c, 0x8fac, 0x9837, 0xa145, 0xaadc, 0xb504, 0xbfc8, 0xcb2f, 0xd744, 0xe411, 0xf1a1,
];

/// `2^(i/1536) * 32768`, truncated, at `0x08ac370c`.
pub const FINE_TABLE: [u16; 128] = [
    0x8000, 0x800e, 0x801d, 0x802c, 0x803b, 0x804a, 0x8058, 0x8067, 0x8076, 0x8085, 0x8094, 0x80a3,
    0x80b1, 0x80c0, 0x80cf, 0x80de, 0x80ed, 0x80fc, 0x810b, 0x811a, 0x8129, 0x8138, 0x8146, 0x8155,
    0x8164, 0x8173, 0x8182, 0x8191, 0x81a0, 0x81af, 0x81be, 0x81cd, 0x81dc, 0x81eb, 0x81fa, 0x8209,
    0x8218, 0x8227, 0x8236, 0x8245, 0x8254, 0x8263, 0x8272, 0x8282, 0x8291, 0x82a0, 0x82af, 0x82be,
    0x82cd, 0x82dc, 0x82eb, 0x82fa, 0x830a, 0x8319, 0x8328, 0x8337, 0x8346, 0x8355, 0x8364, 0x8374,
    0x8383, 0x8392, 0x83a1, 0x83b0, 0x83c0, 0x83cf, 0x83de, 0x83ed, 0x83fd, 0x840c, 0x841b, 0x842a,
    0x843a, 0x8449, 0x8458, 0x8468, 0x8477, 0x8486, 0x8495, 0x84a5, 0x84b4, 0x84c3, 0x84d3, 0x84e2,
    0x84f1, 0x8501, 0x8510, 0x8520, 0x852f, 0x853e, 0x854e, 0x855d, 0x856d, 0x857c, 0x858b, 0x859b,
    0x85aa, 0x85ba, 0x85c9, 0x85d9, 0x85e8, 0x85f8, 0x8607, 0x8617, 0x8626, 0x8636, 0x8645, 0x8655,
    0x8664, 0x8674, 0x8683, 0x8693, 0x86a2, 0x86b2, 0x86c1, 0x86d1, 0x86e0, 0x86f0, 0x8700, 0x870f,
    0x871f, 0x872e, 0x873e, 0x874e, 0x875d, 0x876d, 0x877d, 0x878c,
];

/// `Scream_Note2Pitch` (`0x089952c4`): the table walk, with a non-negative
/// centre note.
///
/// `centre_fine + fine - 127` is the fine offset, borrowing a semitone from
/// `note` while it is negative - so a centre fine of `127` is "in tune" and a
/// centre fine of `0` is one semitone flat. The result is `0x1000 * 2^(d/12)`
/// with `d` the signed semitone distance from the centre, split into an octave
/// shift and a table entry, then scaled by the fine entry. Returned masked to
/// 16 bits, as the original is.
#[must_use]
pub fn note_to_pitch(centre_note: i32, centre_fine: i32, note: i32, fine: i32) -> u16 {
    let mut note = note;
    let mut fine_index = centre_fine + fine - 0x7f;
    while fine_index < 0 {
        fine_index += 0x7f;
        note -= 1;
    }
    let fine_scale = i64::from(FINE_TABLE[(fine_index & 0x7f) as usize]);
    let distance = note - centre_note;
    let semitone = if distance >= 0 {
        // C division and remainder truncate toward zero; both are non-negative here.
        let octaves = (distance / 12) & 0x1f;
        let index = (distance - (distance / 12) * 12) as usize;
        ((i64::from(SAS_PITCH_BASE) << octaves) * i64::from(SEMITONE_TABLE[index])) >> 15
    } else {
        // The original's `-(d / 12)` and `d % 12` truncate toward zero, which
        // is what Rust's `/` and `%` do on signed integers too.
        let mut octaves = -(distance / 12);
        let index = if distance % 12 == 0 {
            0
        } else {
            octaves += 1;
            (distance % 12 + 12) as usize
        };
        ((i64::from(SAS_PITCH_BASE) >> (octaves & 0x1f)) * i64::from(SEMITONE_TABLE[index])) >> 15
    };
    ((semitone * fine_scale) >> 15) as u16
}

/// `Scream_NoteToPitch` (`0x08994fec`): the pitch word SAS is given for a
/// descriptor's `(centre_note, centre_fine)` played at `(note, fine)`.
///
/// A negative centre note is negated before the walk and the result scaled by
/// [`NEGATIVE_CENTRE_SCALE`]. Every descriptor on the Pulse and Pure discs
/// takes that branch.
#[must_use]
pub fn sas_pitch(centre_note: i8, centre_fine: i8, note: i32, fine: i32) -> u16 {
    let negative = centre_note < 0;
    // `-(-128)` does not fit an `i8`; the original's `subu; sll; sra; andi
    // 0xffff` leaves it at `0xff80`, and the walk then runs with that as the
    // centre. No descriptor on any disc surveyed carries it, so match the
    // original rather than guess.
    let centre = if negative {
        i32::from(centre_note.wrapping_neg()) & 0xffff
    } else {
        i32::from(centre_note)
    };
    let pitch = u32::from(note_to_pitch(centre, i32::from(centre_fine), note, fine));
    if negative {
        ((pitch * NEGATIVE_CENTRE_SCALE) >> 16) as u16
    } else {
        pitch as u16
    }
}

/// The playback rate in Hz a pitch word means, rounded to the nearest Hz.
///
/// `SAS_SAMPLE_RATE * pitch / SAS_PITCH_BASE`; the rounding is this port's,
/// since SAS never expresses the rate in Hz - it steps through the waveform by
/// `pitch / 0x1000` samples per output sample. A mixer that resamples by ratio
/// loses nothing to it worth hearing (under 0.005% at 11 kHz).
#[must_use]
pub fn sample_rate_hz(pitch: u16) -> u32 {
    let scaled = SAS_SAMPLE_RATE * u32::from(pitch);
    (scaled + SAS_PITCH_BASE / 2) / SAS_PITCH_BASE
}

#[cfg(test)]
mod tests;
