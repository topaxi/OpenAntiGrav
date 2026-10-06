//! Shannon entropy, used to tell compressed data from structured data.
//!
//! Rules of thumb for 8-bit symbols, on game-disc content:
//!
//! | Entropy (bits/byte) | Usually means |
//! | --- | --- |
//! | < 4.0 | Text, sparse tables, padding |
//! | 4.0 - 6.5 | Structured binary: geometry, index tables, headers |
//! | 6.5 - 7.5 | Mixed, or a container with compressed payloads |
//! | > 7.5 | Compressed or encrypted |
//!
//! These are heuristics for prioritising which files to open first, not
//! conclusions. A high-entropy file is a reason to look for a compression
//! header, not proof that one is there.

/// Shannon entropy of `data` in bits per byte, in `[0, 8]`.
///
/// Returns 0 for empty input.
#[must_use]
pub fn shannon(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut counts = [0u64; 256];
    for &b in data {
        counts[b as usize] += 1;
    }

    let len = data.len() as f64;
    counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
}

/// A coarse label for an entropy value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    /// Mostly text, padding or sparse tables.
    Sparse,
    Structured,
    /// Mixed content, or a container holding compressed payloads.
    Mixed,
    Packed,
}

impl std::fmt::Display for Density {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Sparse => "sparse",
            Self::Structured => "structured",
            Self::Mixed => "mixed",
            Self::Packed => "packed",
        })
    }
}

/// Buckets an entropy value.
#[must_use]
pub fn classify(entropy: f64) -> Density {
    if entropy < 4.0 {
        Density::Sparse
    } else if entropy < 6.5 {
        Density::Structured
    } else if entropy < 7.5 {
        Density::Mixed
    } else {
        Density::Packed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_has_no_entropy() {
        assert_eq!(shannon(&[]), 0.0);
    }

    #[test]
    fn a_single_repeated_byte_has_zero_entropy() {
        assert_eq!(shannon(&[0x41; 1024]), 0.0);
    }

    #[test]
    fn a_uniform_byte_distribution_saturates() {
        let data: Vec<u8> = (0..=255u8).cycle().take(256 * 16).collect();
        let e = shannon(&data);
        assert!((e - 8.0).abs() < 1e-9, "expected 8 bits/byte, got {e}");
    }

    #[test]
    fn two_equally_likely_bytes_give_one_bit() {
        let data: Vec<u8> = [0u8, 1].iter().copied().cycle().take(1024).collect();
        assert!((shannon(&data) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn classifies_the_boundaries() {
        assert_eq!(classify(0.0), Density::Sparse);
        assert_eq!(classify(3.9), Density::Sparse);
        assert_eq!(classify(4.0), Density::Structured);
        assert_eq!(classify(6.4), Density::Structured);
        assert_eq!(classify(6.5), Density::Mixed);
        assert_eq!(classify(7.9), Density::Packed);
        assert_eq!(classify(8.0), Density::Packed);
    }
}
