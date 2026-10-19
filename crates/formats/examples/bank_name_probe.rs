//! Which of a list of cue names a bank resolves, hashed or plain.
//!
//! `cargo run -p oag-formats --example bank_name_probe -- <bank.bnk> name,name,...`

use oag_formats::sblk::Bank;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("bank path");
    let names = args.next().expect("comma separated names");
    let blob = std::fs::read(&path).expect("bank");
    let bank = Bank::parse(&blob).expect("parse");
    println!("{path}: hashed {}", bank.is_hashed());
    for name in names.split(',') {
        match bank.cue_named_or_hashed(name) {
            Some(cue) => {
                println!(
                    "  {name}: cue {} vol {} cmds {:?}",
                    cue.index,
                    cue.volume,
                    cue.range()
                );
                if std::env::var("PROBE_CMDS").is_ok() {
                    let sounds = bank.sounds();
                    for c in cue.range() {
                        let g = &bank.commands[c * 8..c * 8 + 8];
                        let word = bank.order.u32(g, 0);
                        let arg = bank.order.u32(g, 4);
                        let wf = sounds
                            .iter()
                            .find(|s| s.command == c)
                            .map(|s| format!("vol {} note {}", s.volume, s.centre_note))
                            .unwrap_or_default();
                        println!(
                            "    cmd {c} op {:#04x} operand {:#08x} arg {:#x} {wf}",
                            word >> 24,
                            word & 0xff_ffff,
                            arg
                        );
                        if matches!(word >> 24, 0x04 | 0x2e) {
                            let at = bank.parameter_offset as usize + (word & 0xff_ffff) as usize;
                            if let Some(record) = bank.block.get(at..at + 32) {
                                let hex: Vec<String> =
                                    record.iter().map(|b| format!("{b:02x}")).collect();
                                println!("      record {}", hex.join(" "));
                            }
                        }
                    }
                }
            }
            None => println!("  {name}: -"),
        }
    }
}
