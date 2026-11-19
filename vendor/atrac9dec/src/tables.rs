//! Static tables used by the ATRAC9 decoder.
//! Ported from LibAtrac9 / tables.c + tables.h

/// Channel configuration layouts.
/// Index is the 3-bit ChannelConfigIndex from the config data.
#[derive(Debug, Clone, Copy)]
pub struct ChannelConfig {
    pub block_count: u8,
    pub channel_count: u8,
    /// Block types: 0 = Mono, 1 = Stereo, 2 = LFE
    pub types: [u8; 5],
}

pub const CHANNEL_CONFIGS: [ChannelConfig; 6] = [
    // 0: mono
    ChannelConfig {
        block_count: 1,
        channel_count: 1,
        types: [0, 0, 0, 0, 0],
    },
    // 1: dual mono
    ChannelConfig {
        block_count: 2,
        channel_count: 2,
        types: [0, 0, 0, 0, 0],
    },
    // 2: stereo
    ChannelConfig {
        block_count: 1,
        channel_count: 2,
        types: [1, 0, 0, 0, 0],
    },
    // 3: 6-channel (Stereo, Mono, LFE, Stereo)
    ChannelConfig {
        block_count: 4,
        channel_count: 6,
        types: [1, 0, 2, 1, 0],
    },
    // 4: 8-channel
    ChannelConfig {
        block_count: 5,
        channel_count: 8,
        types: [1, 0, 2, 1, 1],
    },
    // 5: dual stereo
    ChannelConfig {
        block_count: 2,
        channel_count: 4,
        types: [1, 1, 0, 0, 0],
    },
];

/// Sample rates indexed by the 4-bit SampleRateIndex in config data.
pub const SAMPLE_RATES: [u32; 16] = [
    11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 44100, 48000, 64000, 88200, 96000,
    128000, 176400, 192000,
];

/// Frame sample count power-of-two for each sample-rate index.
/// frame_samples = 1 << FRAME_SAMPLES_POWER[sri]
pub const FRAME_SAMPLES_POWER: [u8; 16] = [6, 6, 7, 7, 7, 8, 8, 8, 6, 6, 7, 7, 7, 8, 8, 8]; // 64..256

/// Maps quantization unit → codebook set index.
pub const QUANT_UNIT_TO_CODEBOOK_INDEX: [u8; 30] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
];

/// Number of coefficients per quantization unit.
pub const QUANT_UNIT_TO_COEFF_COUNT: [u8; 30] = [
    2, 2, 2, 2, 2, 2, 2, 2, 4, 4, 4, 4, 8, 8, 8, 8, 8, 8, 8, 8, 16, 16, 16, 16, 16, 16, 16, 16, 16,
    16,
];

/// Starting coefficient index for each quantization unit.
pub const QUANT_UNIT_TO_COEFF_INDEX: [usize; 31] = [
    0, 2, 4, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32, 40, 48, 56, 64, 72, 80, 88, 96, 112, 128, 144,
    160, 176, 192, 208, 224, 240, 256,
];

/// Max band count for each sample-rate index (from LibAtrac9).
pub const MAX_BAND_COUNT: [u8; 16] = [8, 8, 12, 12, 12, 18, 18, 18, 8, 8, 12, 12, 12, 16, 16, 16];

/// Maps band count → quantization unit count.
pub const BAND_TO_QUANT_UNIT_COUNT: [u8; 19] = [
    0, 4, 8, 10, 12, 13, 14, 15, 16, 18, 20, 21, 22, 23, 24, 25, 26, 28, 30,
];

/// Spectrum scale factors (2^(sf-15) style).
pub const SPECTRUM_SCALE: [f64; 32] = [
    3.0517578125e-5,
    6.1035156250e-5,
    1.2207031250e-4,
    2.4414062500e-4,
    4.8828125000e-4,
    9.7656250000e-4,
    1.9531250000e-3,
    3.9062500000e-3,
    7.8125000000e-3,
    1.5625000000e-2,
    3.1250000000e-2,
    6.2500000000e-2,
    1.2500000000e-1,
    2.5000000000e-1,
    5.0000000000e-1,
    1.0000000000e+0,
    2.0000000000e+0,
    4.0000000000e+0,
    8.0000000000e+0,
    1.6000000000e+1,
    3.2000000000e+1,
    6.4000000000e+1,
    1.2800000000e+2,
    2.5600000000e+2,
    5.1200000000e+2,
    1.0240000000e+3,
    2.0480000000e+3,
    4.0960000000e+3,
    8.1920000000e+3,
    1.6384000000e+4,
    3.2768000000e+4,
    6.5536000000e+4,
];

/// Quantizer step sizes.
pub const QUANTIZER_STEP_SIZE: [f64; 16] = [
    2.0000000000000000e+0,
    6.6666666666666663e-1,
    2.8571428571428570e-1,
    1.3333333333333333e-1,
    6.4516129032258063e-2,
    3.1746031746031744e-2,
    1.5748031496062992e-2,
    7.8431372549019607e-3,
    3.9138943248532287e-3,
    1.9550342130987292e-3,
    9.7703957010258913e-4,
    4.8840048840048840e-4,
    2.4417043096081065e-4,
    1.2207776353537203e-4,
    6.1037018951994385e-5,
    3.0518043793392844e-5,
];

/// Fine quantizer step sizes (LibAtrac9 `QuantizerFineStepSize`).
pub const QUANTIZER_FINE_STEP_SIZE: [f64; 16] = [
    3.0518043793392844e-05,
    1.0172681264464281e-05,
    4.3597205419132631e-06,
    2.0345362528928561e-06,
    9.8445302559331759e-07,
    4.8441339354591809e-07,
    2.4029955742829012e-07,
    1.1967860311134448e-07,
    5.9722199204291275e-08,
    2.9831909866464167e-08,
    1.4908668194134265e-08,
    7.4525137468602791e-09,
    3.7258019525568114e-09,
    1.8627872668859698e-09,
    9.3136520869755679e-10,
    4.6567549848772173e-10,
];

/// Inverse quantizer step sizes.
pub const QUANTIZER_INVERSE_STEP_SIZE: [f64; 16] = [
    0.5, 1.5, 3.5, 7.5, 15.5, 31.5, 63.5, 127.5, 255.5, 511.5, 1023.5, 2047.5, 4095.5, 8191.5,
    16383.5, 32767.5,
];

/// Scale-factor weight tables (used in gradient / intensity coding).
pub const SCALE_FACTOR_WEIGHTS: [[u8; 32]; 8] = [
    [
        0, 0, 0, 1, 1, 2, 2, 2, 2, 2, 2, 3, 2, 3, 3, 4, 4, 4, 4, 4, 4, 5, 5, 6, 6, 7, 7, 8, 10, 12,
        12, 12,
    ],
    [
        3, 2, 2, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1, 1, 2, 3, 3, 4, 5, 7, 10,
        10, 10,
    ],
    [
        0, 2, 4, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 8, 9, 12,
        12, 12,
    ],
    [
        0, 1, 1, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 6, 7, 8, 8, 10, 11, 11, 12, 13,
        13, 13, 13,
    ],
    [
        0, 2, 2, 3, 3, 4, 4, 5, 4, 5, 5, 5, 5, 6, 7, 8, 8, 8, 8, 9, 9, 9, 10, 10, 11, 12, 12, 13,
        13, 14, 14, 14,
    ],
    [
        1, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 4, 4, 5, 6, 7, 7, 9, 11,
        11, 11,
    ],
    [
        0, 5, 8, 10, 11, 11, 12, 12, 12, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
        13, 12, 12, 12, 12, 13, 15, 15, 15,
    ],
    [
        0, 2, 3, 4, 5, 6, 6, 7, 7, 8, 8, 8, 9, 9, 10, 10, 10, 11, 11, 11, 11, 11, 11, 12, 12, 12,
        12, 13, 13, 15, 15, 15,
    ],
];

/// Min band count: [low_rate, high_rate]
pub const MIN_BAND_COUNT: [u8; 2] = [3, 1];
/// Max extension band: [low_rate, high_rate]
pub const MAX_EXTENSION_BAND: [u8; 2] = [18, 16];
