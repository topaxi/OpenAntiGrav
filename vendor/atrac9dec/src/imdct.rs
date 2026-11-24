//! IMDCT — literal port of LibAtrac9 imdct.c + decinit table generation.

use std::f64::consts::PI;
use std::sync::OnceLock;

const MAX_FRAME_SAMPLES: usize = 256;

fn bit_reverse32(mut value: u32, bit_count: u32) -> u32 {
    value = ((value & 0xaaaa_aaaa) >> 1) | ((value & 0x5555_5555) << 1);
    value = ((value & 0xcccc_cccc) >> 2) | ((value & 0x3333_3333) << 2);
    value = ((value & 0xf0f0_f0f0) >> 4) | ((value & 0x0f0f_0f0f) << 4);
    value = ((value & 0xff00_ff00) >> 8) | ((value & 0x00ff_00ff) << 8);
    value = (value >> 16) | (value << 16);
    if bit_count == 0 { 0 } else { value >> (32 - bit_count) }
}

struct MdctTables {
    sin_tables: [[f64; 256]; 9],
    cos_tables: [[f64; 256]; 9],
    shuffle_tables: [[usize; 256]; 9],
    /// ImdctWindow[bits-6]
    imdct_window: [[f64; 256]; 3],
}

impl MdctTables {
    fn generate(frame_size_power: u8) -> Self {
        let mut t = Self {
            sin_tables: [[0.0; 256]; 9],
            cos_tables: [[0.0; 256]; 9],
            shuffle_tables: [[0; 256]; 9],
            imdct_window: [[0.0; 256]; 3],
        };
        for i in 0..9 {
            let size = 1usize << i;
            for j in 0..size {
                let value = PI * (4 * j + 1) as f64 / (4 * size) as f64;
                t.sin_tables[i][j] = value.sin();
                t.cos_tables[i][j] = value.cos();
            }
            for j in 0..size {
                let gray = (j as u32) ^ ((j as u32) / 2);
                t.shuffle_tables[i][j] = bit_reverse32(gray, i as u32) as usize;
            }
        }
        let frame_size = 1usize << frame_size_power;
        let idx = (frame_size_power as usize) - 6;
        let mut mdct = [0.0f64; 256];
        for i in 0..frame_size {
            mdct[i] = (((i as f64 + 0.5) / frame_size as f64 - 0.5) * PI).sin() + 1.0;
            mdct[i] *= 0.5;
        }
        for i in 0..frame_size {
            let a = mdct[frame_size - 1 - i];
            let b = mdct[i];
            t.imdct_window[idx][i] = b / (a * a + b * b);
        }
        t
    }
}

static TABLES_256: OnceLock<MdctTables> = OnceLock::new(); // power 8 → 256 samples

fn tables_for(bits: u32) -> &'static MdctTables {
    // Sample uses power 8 (256). Support 6,7,8 via separate statics if needed.
    match bits {
        8 => TABLES_256.get_or_init(|| MdctTables::generate(8)),
        7 => {
            static T7: OnceLock<MdctTables> = OnceLock::new();
            T7.get_or_init(|| MdctTables::generate(7))
        }
        6 => {
            static T6: OnceLock<MdctTables> = OnceLock::new();
            T6.get_or_init(|| MdctTables::generate(6))
        }
        _ => TABLES_256.get_or_init(|| MdctTables::generate(8)),
    }
}

#[derive(Debug, Clone)]
pub struct Mdct {
    pub bits: u32,
    pub size: usize,
    pub previous: [f64; MAX_FRAME_SAMPLES],
}

impl Mdct {
    pub fn new(bits: u32) -> Self {
        let size = 1usize << bits;
        assert!(size <= MAX_FRAME_SAMPLES);
        let _ = tables_for(bits); // ensure init
        Self {
            bits,
            size,
            previous: [0.0; MAX_FRAME_SAMPLES],
        }
    }

    /// Turns one frame of spectra into `output` samples, overlapping the half the last frame
    /// left in `previous`. Every sample is the same IEEE double operation in the same order as
    /// LibAtrac9's `Mdct::RunImdct`; the loops run over slices cut to their length first so the
    /// optimiser needs no index check inside them.
    pub fn run(&mut self, input: &[f64], output: &mut [f64]) {
        let size = self.size;
        let half = size / 2;
        let bits = self.bits as usize;
        let mut dct_out = [0.0f64; MAX_FRAME_SAMPLES];
        self.dct4(input, &mut dct_out[..size]);

        let t = tables_for(self.bits);
        let window = &t.imdct_window[bits - 6][..size];
        let (window_lo, window_hi) = window.split_at(half);
        let (dct_lo, dct_hi) = dct_out[..size].split_at(half);
        let (previous_lo, previous_hi) = self.previous[..size].split_at_mut(half);
        let (output_lo, output_hi) = output[..size].split_at_mut(half);

        for i in 0..half {
            let back = half - 1 - i;
            output_lo[i] = window_lo[i] * dct_hi[i] + previous_lo[i];
            output_hi[i] = window_hi[i] * (-dct_hi[back]) - previous_hi[i];
            previous_lo[i] = window_hi[back] * (-dct_lo[back]);
            previous_hi[i] = window_lo[back] * dct_lo[i];
        }
    }

    fn dct4(&self, input: &[f64], output: &mut [f64]) {
        let mdct_bits = self.bits as usize;
        let mdct_size = self.size;
        let t = tables_for(self.bits);
        let mut dct_temp = [0.0f64; MAX_FRAME_SAMPLES];
        let temp = &mut dct_temp[..mdct_size];
        let half_size = mdct_size / 2;

        {
            let input = &input[..mdct_size];
            let sin_table = &t.sin_tables[mdct_bits][..half_size];
            let cos_table = &t.cos_tables[mdct_bits][..half_size];
            for i in 0..half_size {
                let a = input[i * 2];
                let b = input[mdct_size - 1 - i * 2];
                temp[i * 2] = a * cos_table[i] + b * sin_table[i];
                temp[i * 2 + 1] = a * sin_table[i] - b * cos_table[i];
            }
        }

        let stage_count = mdct_bits - 1;
        for stage in 0..stage_count {
            let block_size_bits = stage_count - stage;
            let block_half_size_bits = block_size_bits - 1;
            let block_size = 1usize << block_size_bits;
            let block_half_size = 1usize << block_half_size_bits;
            let sin_table = &t.sin_tables[block_half_size_bits][..block_half_size];
            let cos_table = &t.cos_tables[block_half_size_bits][..block_half_size];

            // A block spans `2 * block_size` entries: the front half holds `block_half_size`
            // pairs, and each pair meets the pair `block_size` entries behind it.
            let mut start = 0;
            if block_half_size == 1 {
                // The last stage: one pair meets one pair, on the table's only entry.
                let (sin, cos) = (sin_table[0], cos_table[0]);
                while start < mdct_size {
                    let quad = &mut temp[start..start + 4];
                    let (f0, f1, b0, b1) = (quad[0], quad[1], quad[2], quad[3]);
                    let a = f0 - b0;
                    let b = f1 - b1;
                    quad[0] = f0 + b0;
                    quad[1] = f1 + b1;
                    quad[2] = a * cos + b * sin;
                    quad[3] = a * sin - b * cos;
                    start += 4;
                }
                continue;
            }
            while start < mdct_size {
                let (front, back) = temp[start..start + 2 * block_size].split_at_mut(block_size);
                for i in 0..block_half_size {
                    let (f0, f1) = (front[i * 2], front[i * 2 + 1]);
                    let (b0, b1) = (back[i * 2], back[i * 2 + 1]);
                    let (sin, cos) = (sin_table[i], cos_table[i]);
                    let a = f0 - b0;
                    let b = f1 - b1;
                    front[i * 2] = f0 + b0;
                    front[i * 2 + 1] = f1 + b1;
                    back[i * 2] = a * cos + b * sin;
                    back[i * 2 + 1] = a * sin - b * cos;
                }
                start += 2 * block_size;
            }
        }

        let shuffle_table = &t.shuffle_tables[mdct_bits][..mdct_size];
        let output = &mut output[..mdct_size];
        for i in 0..mdct_size {
            output[i] = temp[shuffle_table[i]];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The transform as LibAtrac9 writes it, index by index: the specification the slice
    /// version has to equal bit for bit.
    fn reference_dct4(bits: usize, input: &[f64], output: &mut [f64]) {
        let size = 1usize << bits;
        let t = tables_for(bits as u32);
        let mut dct_temp = [0.0f64; MAX_FRAME_SAMPLES];
        let last_index = size - 1;
        let half_size = size / 2;
        for i in 0..half_size {
            let i2 = i * 2;
            let a = input[i2];
            let b = input[last_index - i2];
            let sin = t.sin_tables[bits][i];
            let cos = t.cos_tables[bits][i];
            dct_temp[i2] = a * cos + b * sin;
            dct_temp[i2 + 1] = a * sin - b * cos;
        }
        let stage_count = bits - 1;
        for stage in 0..stage_count {
            let block_count = 1 << stage;
            let block_size_bits = stage_count - stage;
            let block_half_size_bits = block_size_bits - 1;
            let block_size = 1 << block_size_bits;
            let block_half_size = 1 << block_half_size_bits;
            let sin_table = &t.sin_tables[block_half_size_bits];
            let cos_table = &t.cos_tables[block_half_size_bits];
            for block in 0..block_count {
                for i in 0..block_half_size {
                    let front_pos = (block * block_size + i) * 2;
                    let back_pos = front_pos + block_size;
                    let a = dct_temp[front_pos] - dct_temp[back_pos];
                    let b = dct_temp[front_pos + 1] - dct_temp[back_pos + 1];
                    let sin = sin_table[i];
                    let cos = cos_table[i];
                    dct_temp[front_pos] += dct_temp[back_pos];
                    dct_temp[front_pos + 1] += dct_temp[back_pos + 1];
                    dct_temp[back_pos] = a * cos + b * sin;
                    dct_temp[back_pos + 1] = a * sin - b * cos;
                }
            }
        }
        let shuffle_table = &t.shuffle_tables[bits];
        for i in 0..size {
            output[i] = dct_temp[shuffle_table[i]];
        }
    }

    fn reference_run(
        bits: usize,
        previous: &mut [f64; MAX_FRAME_SAMPLES],
        input: &[f64],
        output: &mut [f64],
    ) {
        let size = 1usize << bits;
        let half = size / 2;
        let mut dct_out = [0.0f64; MAX_FRAME_SAMPLES];
        reference_dct4(bits, input, &mut dct_out[..size]);
        let window = &tables_for(bits as u32).imdct_window[bits - 6];
        for i in 0..half {
            output[i] = window[i] * dct_out[i + half] + previous[i];
            output[i + half] = window[i + half] * (-dct_out[size - 1 - i]) - previous[i + half];
            previous[i] = window[size - 1 - i] * (-dct_out[half - i - 1]);
            previous[i + half] = window[half - i - 1] * dct_out[i];
        }
    }

    #[test]
    fn the_slice_transform_equals_the_indexed_one_bit_for_bit_across_frames() {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for bits in 6..=8u32 {
            let size = 1usize << bits;
            let mut mdct = Mdct::new(bits);
            let mut previous = [0.0f64; MAX_FRAME_SAMPLES];
            for frame in 0..64 {
                let mut input = [0.0f64; MAX_FRAME_SAMPLES];
                for value in &mut input[..size] {
                    let r = next();
                    *value = match r % 7 {
                        0 => 0.0,
                        1 => f64::from_bits(r),
                        _ => (r >> 11) as f64 / (1u64 << 40) as f64 - 4096.0,
                    };
                    if !value.is_finite() {
                        *value = 1.0e300;
                    }
                }
                let mut got = [0.0f64; MAX_FRAME_SAMPLES];
                let mut want = [0.0f64; MAX_FRAME_SAMPLES];
                mdct.run(&input[..size], &mut got[..size]);
                reference_run(bits as usize, &mut previous, &input[..size], &mut want[..size]);
                for i in 0..size {
                    assert_eq!(
                        got[i].to_bits(),
                        want[i].to_bits(),
                        "bits {bits} frame {frame} sample {i}"
                    );
                    assert_eq!(
                        mdct.previous[i].to_bits(),
                        previous[i].to_bits(),
                        "bits {bits} frame {frame} overlap {i}"
                    );
                }
            }
        }
    }
}
