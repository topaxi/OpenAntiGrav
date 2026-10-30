//! The spike's guest: "+5 % top speed per speed pad crossed", the shape of a
//! craft-specialty mod. Built for `wasm32-unknown-unknown`, `no_std`, no WASI.
//!
//! ABI (core WebAssembly, the subset a browser's `WebAssembly` API also runs):
//! the host writes an observation into the 256-byte `IO` buffer, calls one
//! export, and reads the command and the state blob back out. The guest keeps
//! **no state of its own between calls**: everything that must persist lives in
//! the 64-byte blob at the start of `IO`, which the host owns, snapshots and
//! hashes. `on_tick_leaky` breaks that rule on purpose to show what goes wrong.
use core::ptr::{addr_of_mut, read_volatile, write_volatile};

#[link(wasm_import_module = "oag")]
unsafe extern "C" {
    /// The host's seeded generator (`oag_core::Rng`); the guest has none of its own.
    fn rng_next() -> u32;
}

static mut IO: [u8; 256] = [0; 256];
static mut LEAK: u32 = 0;

const BLOB_PADS: usize = 0;
const BLOB_SCALE: usize = 4;
const BLOB_ACCUM: usize = 8;
const OBS_SPEED: usize = 68;
const OBS_BASE_TOP: usize = 76;
const OUT_SCALE: usize = 96;
const SPIN_COUNTER: usize = 128;

fn io() -> *mut u8 {
    addr_of_mut!(IO).cast()
}

fn get_f32(at: usize) -> f32 {
    f32::from_bits(get_u32(at))
}

fn get_u32(at: usize) -> u32 {
    let mut bytes = [0u8; 4];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = unsafe { read_volatile(io().add(at + i)) };
    }
    u32::from_le_bytes(bytes)
}

fn put_u32(at: usize, v: u32) {
    for (i, b) in v.to_le_bytes().iter().enumerate() {
        unsafe { write_volatile(io().add(at + i), *b) };
    }
}

fn put_f32(at: usize, v: f32) {
    put_u32(at, v.to_bits());
}

fn scale() -> f32 {
    let s = get_f32(BLOB_SCALE);
    if s == 0.0 { 1.0 } else { s }
}

#[unsafe(no_mangle)]
pub extern "C" fn oag_abi() -> u32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn oag_io() -> u32 {
    io() as u32
}

/// On a `pad-crossed` event: one more pad crossed, five percent more top speed, capped.
#[unsafe(no_mangle)]
pub extern "C" fn on_pad_enter() -> u32 {
    put_u32(BLOB_PADS, get_u32(BLOB_PADS) + 1);
    let next = scale() * 1.05;
    put_f32(BLOB_SCALE, if next > 1.5 { 1.5 } else { next });
    let jitter = (unsafe { rng_next() } >> 8) as f32 / 16_777_216.0;
    put_f32(BLOB_ACCUM, get_f32(BLOB_ACCUM) + jitter);
    0
}

fn tick_scale() -> f32 {
    let s = scale();
    let speed = get_f32(OBS_SPEED);
    let drag = speed / (get_f32(OBS_BASE_TOP) * s);
    let accum = get_f32(BLOB_ACCUM);
    put_f32(BLOB_ACCUM, accum * 0.999 + 0.0001 * speed);
    s * (1.0 - 0.01 * drag * drag) + accum.sqrt() * 0.001
}

/// The `tick` call: the top-speed scale this tick (a `scale-thrust` in miniature).
#[unsafe(no_mangle)]
pub extern "C" fn on_tick() -> u32 {
    put_f32(OUT_SCALE, tick_scale());
    0
}

/// The same, but keeping a counter in its own linear memory between calls: what
/// a rollback cannot restore unless the host owns every byte that persists.
#[unsafe(no_mangle)]
pub extern "C" fn on_tick_leaky() -> u32 {
    let calls = unsafe {
        let p = addr_of_mut!(LEAK);
        let n = read_volatile(p) + 1;
        write_volatile(p, n);
        n
    };
    put_f32(OUT_SCALE, tick_scale() + calls as f32 * 1.0e-6);
    0
}

/// A runaway loop: counts to `n`, writing the count each step, so the host can
/// read exactly where fuel ran out.
#[unsafe(no_mangle)]
pub extern "C" fn spin(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        put_u32(SPIN_COUNTER, i);
        i += 1;
    }
    i
}
