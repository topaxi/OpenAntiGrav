//! The headless server needs no shared library beyond the C runtime.
//!
//! `oag-server` links `oag-raceplay`, which links wgpu, winit and cpal, and
//! never initialises any of them. The linker drops what the server never calls
//! (`--as-needed`), and wgpu and winit load their libraries at run time, so the
//! binary asks for libc and a few runtime libraries only. That is what lets a
//! plain Linux host with no ALSA, X11 or Vulkan run it. A change that makes the
//! server call into one of those libraries adds a `DT_NEEDED` entry here and
//! fails this test.
//!
//! Read straight from the ELF dynamic section rather than through `ldd`, which
//! executes the loader and is not on every machine.

#![cfg(all(target_os = "linux", target_pointer_width = "64"))]

/// The shared objects a headless host is assumed to have.
const ALLOWED: &[&str] = &[
    "libc.so.6",
    "libm.so.6",
    "libgcc_s.so.1",
    "libdl.so.2",
    "libpthread.so.0",
    "librt.so.1",
    "ld-linux-x86-64.so.2",
    "ld-linux-aarch64.so.1",
];

fn u16_at(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([b[at], b[at + 1]]))
}
fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}
fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

/// The `DT_NEEDED` names of a 64-bit little-endian ELF.
fn needed(elf: &[u8]) -> Vec<String> {
    assert_eq!(&elf[..4], b"\x7fELF");
    let phoff = u64_at(elf, 0x20) as usize;
    let phentsize = u16_at(elf, 0x36);
    let phnum = u16_at(elf, 0x38);
    let mut loads = Vec::new(); // (vaddr, offset, filesz)
    let mut dynamic = None;
    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        let offset = u64_at(elf, ph + 8) as usize;
        let (vaddr, filesz) = (u64_at(elf, ph + 16), u64_at(elf, ph + 32));
        match u32_at(elf, ph) {
            1 => loads.push((vaddr, offset, filesz)),
            2 => dynamic = Some((offset, filesz as usize)),
            _ => {}
        }
    }
    let to_offset = |vaddr: u64| {
        loads
            .iter()
            .find(|&&(v, _, size)| vaddr >= v && vaddr < v + size)
            .map(|&(v, o, _)| o + (vaddr - v) as usize)
            .expect("an address inside a PT_LOAD")
    };
    let Some((dyn_off, dyn_len)) = dynamic else {
        return Vec::new(); // statically linked
    };
    let (mut names, mut strtab) = (Vec::new(), None);
    for entry in elf[dyn_off..dyn_off + dyn_len].as_chunks::<16>().0 {
        let (tag, value) = (u64_at(&entry[..], 0), u64_at(&entry[..], 8));
        match tag {
            0 => break,
            1 => names.push(value as usize),
            5 => strtab = Some(to_offset(value)),
            _ => {}
        }
    }
    let strtab = strtab.expect("a string table");
    names
        .into_iter()
        .map(|n| {
            let rest = &elf[strtab + n..];
            let end = rest.iter().position(|&b| b == 0).unwrap();
            String::from_utf8(rest[..end].to_vec()).unwrap()
        })
        .collect()
}

#[test]
fn the_server_needs_no_shared_library_beyond_the_c_runtime() {
    let binary = std::fs::read(env!("CARGO_BIN_EXE_oag-server")).expect("the server binary");
    let libs = needed(&binary);
    assert!(
        libs.iter().any(|l| l.starts_with("libc.")),
        "the reader found no libc in {libs:?}: it is not reading the dynamic section"
    );
    let extra: Vec<&String> = libs
        .iter()
        .filter(|l| !ALLOWED.contains(&l.as_str()))
        .collect();
    assert!(
        extra.is_empty(),
        "oag-server now needs {extra:?}: a plain headless host will not have it. \
         Something on the server's path calls into a graphics, windowing or audio library."
    );
}
