//! The nearest of a track's sample positions, for every craft every tick.
//!
//! [`Positions::nearest`] answers exactly what a scan of every sample answers (the lowest
//! index among the shortest lengths), reading about a tenth of them: a sphere round each
//! run of samples proves most runs hold no answer. Rounding is covered by [`SLACK`], which
//! dwarfs the few parts in ten million a computed length can be off.

use oag_core::math::Vec3;

/// Samples per [`Block`].
const BLOCK: usize = 32;

/// How far a bound is widened, as a fraction of itself.
const SLACK: f32 = 1e-4;

/// A sphere round one run of [`BLOCK`] samples.
#[derive(Debug, Clone, PartialEq)]
struct Block {
    centre: Vec3,
    radius: f32,
}

/// Sample positions in table order, with their [`Block`]s.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Positions {
    at: Vec<Vec3>,
    blocks: Vec<Block>,
}

impl Positions {
    pub(crate) fn new(at: Vec<Vec3>) -> Self {
        let blocks = at
            .chunks(BLOCK)
            .map(|run| {
                let centre = run[run.len() / 2];
                let radius = run
                    .iter()
                    .fold(0.0f32, |widest, point| widest.max(point.distance(centre)));
                Block { centre, radius }
            })
            .collect();
        Self { at, blocks }
    }

    /// The nearest position's index and its length; a tie keeps the earlier one.
    ///
    /// `reach` is a length some sample cannot be beyond, so a block wholly past it holds no
    /// shortest length. A `NaN` position bounds nothing and reads every sample.
    pub(crate) fn nearest(&self, position: Vec3) -> Option<(usize, f32)> {
        let reach = self.blocks.iter().fold(f32::INFINITY, |reach, block| {
            reach.min(block.centre.distance(position) + block.radius)
        }) * (1.0 + SLACK);

        // A length is the root of its square and a root never falls, so a square no
        // smaller than the best's cannot be strictly nearer: only a smaller one pays
        // for the root, and the comparison that decides stays on the lengths.
        let mut best: Option<(usize, f32, f32)> = None;
        for (block, (run, points)) in self.blocks.iter().zip(self.at.chunks(BLOCK).enumerate()) {
            let closest =
                block.centre.distance(position) * (1.0 - SLACK) - block.radius * (1.0 + SLACK);
            if closest > reach {
                continue;
            }
            for (offset, point) in points.iter().enumerate() {
                let squared = (*point - position).length_squared();
                if best.is_some_and(|(_, _, previous)| squared >= previous) {
                    continue;
                }
                let distance = squared.sqrt();
                if best.is_none_or(|(_, previous, _)| distance < previous) {
                    best = Some((run * BLOCK + offset, distance, squared));
                }
            }
        }
        best.map(|(index, distance, _)| (index, distance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scan this stands in for.
    fn scan(at: &[Vec3], position: Vec3) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (index, point) in at.iter().enumerate() {
            let distance = (*point - position).length();
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best
    }

    fn agree(table: &Positions, position: Vec3) {
        let bits = |found: Option<(usize, f32)>| found.map(|(i, d)| (i, d.to_bits()));
        assert_eq!(
            bits(table.nearest(position)),
            bits(scan(&table.at, position)),
            "at {position}"
        );
    }

    /// Paths that double back and jump across the table, repeated samples for ties, far
    /// and near queries, and large coordinates.
    #[test]
    fn the_answer_is_the_scans_to_the_bit() {
        let mut rng = oag_core::rng::Rng::new(5);
        let mut at = Vec::new();
        for path in 0..5 {
            let mut point = Vec3::new(
                rng.next_f32() * 9000.0 - 4500.0,
                rng.next_f32() * 300.0,
                rng.next_f32() * 9000.0 - 4500.0,
            );
            for _ in 0..700 {
                let step = Vec3::new(rng.next_f32() - 0.5, rng.next_f32() - 0.5, 1.0);
                point += step * if path % 2 == 0 { 2.5 } else { -2.5 };
                at.push(point);
                if at.len() % 97 == 0 {
                    at.push(point);
                }
            }
        }
        let table = Positions::new(at.clone());
        for index in (0..at.len()).step_by(7) {
            agree(&table, at[index]);
            let aside = Vec3::new(rng.next_f32(), rng.next_f32(), rng.next_f32()) * 40.0;
            agree(&table, at[index] + aside);
        }
        for _ in 0..500 {
            let far = Vec3::new(rng.next_f32(), rng.next_f32(), rng.next_f32()) * 40000.0;
            agree(&table, far - Vec3::splat(20000.0));
        }
    }

    #[test]
    fn a_tie_keeps_the_earlier_sample_and_nothing_finds_nothing() {
        assert_eq!(Positions::new(Vec::new()).nearest(Vec3::ZERO), None);
        let same = Positions::new(vec![Vec3::ONE; 100]);
        assert_eq!(same.nearest(Vec3::ONE), Some((0, 0.0)));
        agree(&same, Vec3::splat(f32::NAN));
    }
}
