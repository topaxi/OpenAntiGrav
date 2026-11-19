//! Huffman codebook support for ATRAC9.

use crate::bit_reader::{sign_extend32, BitReader};
use crate::huffman_tables::*;

#[derive(Debug)]
pub struct HuffmanCodebook {
    pub bits: &'static [u8],
    pub codes: &'static [u16],
    pub lookup: Vec<u8>,
    pub length: usize,
    pub value_count: usize,
    pub value_count_power: u32,
    pub value_bits: u32,
    pub value_max: i32,
    pub max_bit_size: u32,
}

impl HuffmanCodebook {
    pub fn new(
        bits: &'static [u8],
        codes: &'static [u16],
        value_count: usize,
        value_count_power: u32,
        value_bits: u32,
        max_bit_size: u32,
    ) -> Self {
        let length = bits.len();
        let value_max = 1i32 << value_bits;
        let mut lookup = vec![0u8; 1usize << max_bit_size];
        for i in 0..length {
            let b = bits[i] as u32;
            if b == 0 { continue; }
            let unused = max_bit_size.saturating_sub(b);
            let start = (codes[i] as u32) << unused;
            let len = 1u32 << unused;
            for j in start..(start + len).min(lookup.len() as u32) {
                lookup[j as usize] = i as u8;
            }
        }
        Self { bits, codes, lookup, length, value_count, value_count_power, value_bits, value_max, max_bit_size }
    }

    pub fn empty() -> Self {
        Self { bits: &[], codes: &[], lookup: vec![], length: 0, value_count: 0, value_count_power: 0, value_bits: 0, value_max: 0, max_bit_size: 0 }
    }

    #[inline]
    pub fn read(&self, br: &mut BitReader, is_signed: bool) -> i32 {
        if self.length == 0 { return 0; }
        let code = br.peek(self.max_bit_size) as usize;
        let idx = *self.lookup.get(code).unwrap_or(&0) as usize;
        let bits = *self.bits.get(idx).unwrap_or(&0) as u32;
        br.skip(bits);
        let value = idx as i32;
        if is_signed { sign_extend32(value, self.value_bits as i32) } else { value }
    }
}

pub fn decode_huffman_values(spectrum: &mut [i32], index: usize, band_count: usize, huff: &HuffmanCodebook, values: &[i32]) {
    let value_count = band_count >> huff.value_count_power;
    let mask = (1 << huff.value_bits) - 1;
    let mut idx = index;
    for i in 0..value_count {
        let mut value = *values.get(i).unwrap_or(&0);
        for _ in 0..huff.value_count {
            if idx < spectrum.len() {
                spectrum[idx] = sign_extend32(value & mask, huff.value_bits as i32);
                idx += 1;
            }
            value >>= huff.value_bits;
        }
    }
}

pub struct ScaleFactorCodebooks {
    pub unsigned: [HuffmanCodebook; 7],
    pub signed: [HuffmanCodebook; 6],
}

impl ScaleFactorCodebooks {
    pub fn new() -> Self {
        let unsigned = [
            HuffmanCodebook::empty(),
            HuffmanCodebook::new(&SF_A1_BITS, &SF_A1_CODES, 1, 0, 1, 1),
            HuffmanCodebook::new(&SF_A2_BITS, &SF_A2_CODES, 1, 0, 2, 3),
            HuffmanCodebook::new(&SF_A3_BITS, &SF_A3_CODES, 1, 0, 3, 6),
            HuffmanCodebook::new(&SF_A4_BITS, &SF_A4_CODES, 1, 0, 4, 8),
            HuffmanCodebook::new(&SF_A5_BITS, &SF_A5_CODES, 1, 0, 5, 8),
            HuffmanCodebook::new(&SF_A6_BITS, &SF_A6_CODES, 1, 0, 6, 8),
        ];
        let signed = [
            HuffmanCodebook::empty(),
            HuffmanCodebook::empty(),
            HuffmanCodebook::new(&SF_B2_BITS, &SF_B2_CODES, 1, 0, 2, 2),
            HuffmanCodebook::new(&SF_B3_BITS, &SF_B3_CODES, 1, 0, 3, 6),
            HuffmanCodebook::new(&SF_B4_BITS, &SF_B4_CODES, 1, 0, 4, 8),
            HuffmanCodebook::new(&SF_B5_BITS, &SF_B5_CODES, 1, 0, 5, 8),
        ];
        Self { unsigned, signed }
    }
}

impl Default for ScaleFactorCodebooks {
    fn default() -> Self { Self::new() }
}

pub struct SpectrumCodebooks {
    pub tables: [[[HuffmanCodebook; 4]; 8]; 2],
}

impl SpectrumCodebooks {
    pub fn new() -> Self {
        let mut tables: [[[HuffmanCodebook; 4]; 8]; 2] =
            std::array::from_fn(|_| std::array::from_fn(|_| std::array::from_fn(|_| HuffmanCodebook::empty())));

        tables[0][2][0] = HuffmanCodebook::new(&SPECTRUM_A21_BITS, &SPECTRUM_A21_CODES, 2, 1, 2, 3);
        tables[0][2][1] = HuffmanCodebook::new(&SPECTRUM_A22_BITS, &SPECTRUM_A22_CODES, 4, 2, 2, 8);
        tables[0][2][2] = HuffmanCodebook::new(&SPECTRUM_A23_BITS, &SPECTRUM_A23_CODES, 4, 2, 2, 9);
        tables[0][2][3] = HuffmanCodebook::new(&SPECTRUM_A24_BITS, &SPECTRUM_A24_CODES, 4, 2, 2, 10);
        tables[0][3][0] = HuffmanCodebook::new(&SPECTRUM_A31_BITS, &SPECTRUM_A31_CODES, 2, 1, 3, 7);
        tables[0][3][1] = HuffmanCodebook::new(&SPECTRUM_A32_BITS, &SPECTRUM_A32_CODES, 2, 1, 3, 7);
        tables[0][3][2] = HuffmanCodebook::new(&SPECTRUM_A33_BITS, &SPECTRUM_A33_CODES, 2, 1, 3, 8);
        tables[0][3][3] = HuffmanCodebook::new(&SPECTRUM_A34_BITS, &SPECTRUM_A34_CODES, 2, 1, 3, 10);
        tables[0][4][0] = HuffmanCodebook::new(&SPECTRUM_A41_BITS, &SPECTRUM_A41_CODES, 2, 1, 4, 9);
        tables[0][4][1] = HuffmanCodebook::new(&SPECTRUM_A42_BITS, &SPECTRUM_A42_CODES, 2, 1, 4, 10);
        tables[0][4][2] = HuffmanCodebook::new(&SPECTRUM_A43_BITS, &SPECTRUM_A43_CODES, 2, 1, 4, 10);
        tables[0][4][3] = HuffmanCodebook::new(&SPECTRUM_A44_BITS, &SPECTRUM_A44_CODES, 2, 1, 4, 10);
        tables[0][5][0] = HuffmanCodebook::new(&SPECTRUM_A51_BITS, &SPECTRUM_A51_CODES, 1, 0, 5, 6);
        tables[0][5][1] = HuffmanCodebook::new(&SPECTRUM_A52_BITS, &SPECTRUM_A52_CODES, 1, 0, 5, 6);
        tables[0][5][2] = HuffmanCodebook::new(&SPECTRUM_A53_BITS, &SPECTRUM_A53_CODES, 1, 0, 5, 7);
        tables[0][5][3] = HuffmanCodebook::new(&SPECTRUM_A54_BITS, &SPECTRUM_A54_CODES, 1, 0, 5, 8);
        tables[0][6][0] = HuffmanCodebook::new(&SPECTRUM_A61_BITS, &SPECTRUM_A61_CODES, 1, 0, 6, 7);
        tables[0][6][1] = HuffmanCodebook::new(&SPECTRUM_A62_BITS, &SPECTRUM_A62_CODES, 1, 0, 6, 7);
        tables[0][6][2] = HuffmanCodebook::new(&SPECTRUM_A63_BITS, &SPECTRUM_A63_CODES, 1, 0, 6, 8);
        tables[0][6][3] = HuffmanCodebook::new(&SPECTRUM_A64_BITS, &SPECTRUM_A64_CODES, 1, 0, 6, 9);
        tables[0][7][0] = HuffmanCodebook::new(&SPECTRUM_A71_BITS, &SPECTRUM_A71_CODES, 1, 0, 7, 8);
        tables[0][7][1] = HuffmanCodebook::new(&SPECTRUM_A72_BITS, &SPECTRUM_A72_CODES, 1, 0, 7, 8);
        tables[0][7][2] = HuffmanCodebook::new(&SPECTRUM_A73_BITS, &SPECTRUM_A73_CODES, 1, 0, 7, 9);
        tables[0][7][3] = HuffmanCodebook::new(&SPECTRUM_A74_BITS, &SPECTRUM_A74_CODES, 1, 0, 7, 10);
        tables[1][2][1] = HuffmanCodebook::new(&SPECTRUM_B22_BITS, &SPECTRUM_B22_CODES, 4, 2, 2, 10);
        tables[1][2][2] = HuffmanCodebook::new(&SPECTRUM_B23_BITS, &SPECTRUM_B23_CODES, 4, 2, 2, 10);
        tables[1][2][3] = HuffmanCodebook::new(&SPECTRUM_B24_BITS, &SPECTRUM_B24_CODES, 4, 2, 2, 10);
        tables[1][3][1] = HuffmanCodebook::new(&SPECTRUM_B32_BITS, &SPECTRUM_B32_CODES, 2, 1, 3, 9);
        tables[1][3][2] = HuffmanCodebook::new(&SPECTRUM_B33_BITS, &SPECTRUM_B33_CODES, 2, 1, 3, 10);
        tables[1][3][3] = HuffmanCodebook::new(&SPECTRUM_B34_BITS, &SPECTRUM_B34_CODES, 2, 1, 3, 10);
        tables[1][4][1] = HuffmanCodebook::new(&SPECTRUM_B42_BITS, &SPECTRUM_B42_CODES, 2, 1, 4, 10);
        tables[1][4][2] = HuffmanCodebook::new(&SPECTRUM_B43_BITS, &SPECTRUM_B43_CODES, 2, 1, 4, 10);
        tables[1][4][3] = HuffmanCodebook::new(&SPECTRUM_B44_BITS, &SPECTRUM_B44_CODES, 2, 1, 4, 10);
        tables[1][5][1] = HuffmanCodebook::new(&SPECTRUM_B52_BITS, &SPECTRUM_B52_CODES, 1, 0, 5, 7);
        tables[1][5][2] = HuffmanCodebook::new(&SPECTRUM_B53_BITS, &SPECTRUM_B53_CODES, 1, 0, 5, 8);
        tables[1][5][3] = HuffmanCodebook::new(&SPECTRUM_B54_BITS, &SPECTRUM_B54_CODES, 1, 0, 5, 9);
        tables[1][6][1] = HuffmanCodebook::new(&SPECTRUM_B62_BITS, &SPECTRUM_B62_CODES, 1, 0, 6, 8);
        tables[1][6][2] = HuffmanCodebook::new(&SPECTRUM_B63_BITS, &SPECTRUM_B63_CODES, 1, 0, 6, 9);
        tables[1][6][3] = HuffmanCodebook::new(&SPECTRUM_B64_BITS, &SPECTRUM_B64_CODES, 1, 0, 6, 10);
        tables[1][7][1] = HuffmanCodebook::new(&SPECTRUM_B72_BITS, &SPECTRUM_B72_CODES, 1, 0, 7, 9);
        tables[1][7][2] = HuffmanCodebook::new(&SPECTRUM_B73_BITS, &SPECTRUM_B73_CODES, 1, 0, 7, 10);
        tables[1][7][3] = HuffmanCodebook::new(&SPECTRUM_B74_BITS, &SPECTRUM_B74_CODES, 1, 0, 7, 10);

        Self { tables }
    }

    pub fn get(&self, codebook_set: usize, precision: usize, quant_group: usize) -> &HuffmanCodebook {
        &self.tables[codebook_set.min(1)][precision.min(7)][quant_group.min(3)]
    }
}

impl Default for SpectrumCodebooks {
    fn default() -> Self {
        Self::new()
    }
}
