//! Prints the raw command list of named cues in one bank of an opened image.
//!
//! ```sh
//! cargo run -p oag-game --example hd_cue_commands_probe -- data/images/hdfury-ps3-eu-dec.iso 'Data\Sound\speech_zone.bnk' zone_5 c_CLEAR
//! ```
use oag_formats::sblk::Bank;
use oag_formats::sblk::timeline::WalkModel;

fn main() {
    let mut args = std::env::args().skip(1);
    let image = args.next().expect("image");
    let bank_path = args.next().expect("bank path in the archive");
    let names: Vec<String> = args.collect();
    let mut opened = oag_source::title::open_source(&image, Vec::new(), Vec::new()).expect("opens");
    let blob = opened.archives.read_name(&bank_path).expect("bank reads");
    let bank = Bank::parse(&blob).expect("parses");
    let all = bank.sound_names();
    for name in &all {
        if !names.is_empty() && !names.iter().any(|n| n == &name.name) {
            continue;
        }
        let Some(cue) = bank.cue(name.cue) else {
            continue;
        };
        println!(
            "cue {} {} vol {} flags {:#x} cmds {:?}",
            cue.index,
            name.name,
            cue.volume,
            cue.flags,
            cue.range()
        );
        for c in cue.range() {
            let g = &bank.commands[c * 8..c * 8 + 8];
            let word = bank.order.u32(g, 0);
            let arg = bank.order.u32(g, 4);
            println!(
                "  cmd {c:>4} op {:#04x} operand {:#08x} delay-word {:#010x} ({})",
                word >> 24,
                word & 0xff_ffff,
                arg,
                arg as u16 as i16
            );
        }
        let t = bank.cue_timeline_modelled(
            &cue,
            &[],
            WalkModel {
                goto_markers: true,
                ..WalkModel::default()
            },
        );
        println!(
            "  timeline: complete={} unread={:02x?} unresolved={} passed={:02x?}",
            t.is_complete(),
            t.unread,
            t.unresolved,
            t.passed
        );
        for g in &t.grains {
            println!(
                "    tick {:>4} ({:.4}s @240Hz) wf@{:#x} {} Hz angle {} vol cue {} snd {} scale {}",
                g.tick,
                f64::from(g.tick) / 240.0,
                g.sound.offset,
                g.sound.sample_rate(),
                g.angle,
                g.cue_volume,
                g.sound.volume,
                g.scale
            );
        }
    }
}
