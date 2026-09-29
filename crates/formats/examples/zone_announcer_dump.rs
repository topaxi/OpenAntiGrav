//! Dump every grain of a Zone announcer bank's cues, and each waveform to WAV.
//!
//! ```sh
//! cargo run -p oag-formats --example zone_announcer_dump -- speech_zone.bnk out_dir
//! ```

use std::fmt::Write as _;

use oag_formats::sblk::{Bank, decode_adpcm};

fn wav(rate: u32, pcm: &[i16]) -> Vec<u8> {
    let data = (pcm.len() * 2) as u32;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("bank path");
    let out = args.next().expect("out dir");
    std::fs::create_dir_all(&out).unwrap();
    let blob = std::fs::read(&path).unwrap();
    let bank = Bank::parse(&blob).unwrap();
    let names = bank.sound_names();
    let sounds = bank.sounds();
    for name in &names {
        let Some(cue) = bank.cue(name.cue) else {
            continue;
        };
        let mut line = String::new();
        writeln!(
            line,
            "cue {} {} vol {} flags {:#x} cmds {:?}",
            cue.index,
            name.name,
            cue.volume,
            cue.flags,
            cue.range()
        )
        .unwrap();
        for c in cue.range() {
            let g = &bank.commands[c * 8..c * 8 + 8];
            let word = bank.order.u32(g, 0);
            let arg = bank.order.u32(g, 4);
            let op = (word >> 24) as u8;
            let operand = word & 0xff_ffff;
            write!(
                line,
                "  cmd {c:>3} op {op:#04x} operand {operand:#08x} arg {arg:#010x}"
            )
            .unwrap();
            if let Some(s) = sounds.iter().find(|s| s.command == c) {
                let pcm = decode_adpcm(bank.waveform(s).unwrap());
                let rate = s.sample_rate();
                let secs = pcm.len() as f64 / f64::from(rate);
                write!(
                    line,
                    "  KEYON wf@{:#x} len {} {:.3}s desc vol {} note {} fine {}",
                    s.offset, s.length, secs, s.volume, s.centre_note, s.centre_fine
                )
                .unwrap();
                std::fs::write(format!("{out}/{}_cmd{c}.wav", name.name), wav(rate, &pcm)).unwrap();
            }
            if op == 5 || op == 8 {
                let at = (bank.parameter_offset + operand) as usize;
                let rec = &bank.block[at..at + 32];
                write!(
                    line,
                    "  CHILD vol {} idx {}",
                    bank.order.u32(rec, 0),
                    bank.order.u32(rec, 0x0c) as i32
                )
                .unwrap();
                write!(line, " rec[4..12]={:02x?}", &rec[4..12]).unwrap();
            }
            line.push('\n');
        }
        print!("{line}");
    }
}
