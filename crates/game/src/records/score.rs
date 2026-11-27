//! The score record a Detonator run is compared against and kept in
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-endrace.md`, "The record notice").
//!
//! The original keeps one table per circuit and direction (`CRC32("<NN_Track>_fury")`), with no
//! speed class in the key, so the best is the highest [`Record::best_score`] of every row of the
//! title, circuit and mode.

use oag_ui_screens::endrace::ScoreRecord;

use super::{Key, Record, Store};

impl Store {
    /// The standing best score of `key`'s title, circuit and mode, `0` before one has scored.
    #[must_use]
    pub fn best_score(&self, key: &Key) -> u32 {
        self.records
            .iter()
            .filter(|row| row.title == key.title && row.track == key.track && row.mode == key.mode)
            .filter_map(|row| row.best_score)
            .max()
            .unwrap_or(0)
    }

    /// Judges `score` against the standing best, then keeps it on `key`'s row. Called before the
    /// race is folded in. A run that is not `keep` (an unofficial one) sets no record: its notice
    /// is the standing best, so it never announces a record the next run does not find (chosen,
    /// not measured).
    pub fn judge_score(&mut self, key: &Key, score: u32, keep: bool) -> ScoreRecord {
        let best = self.best_score(key);
        if !keep {
            return ScoreRecord {
                new: false,
                value: best,
            };
        }
        let notice = ScoreRecord::judge(score, best);
        if notice.new {
            let at = match self.records.iter().position(|row| row.key() == *key) {
                Some(at) => at,
                None => {
                    self.records.push(Record {
                        title: key.title.clone(),
                        track: key.track.clone(),
                        mode: key.mode.clone(),
                        class: key.class.clone(),
                        ..Record::default()
                    });
                    self.records.sort_by_key(Record::key);
                    self.records
                        .iter()
                        .position(|row| row.key() == *key)
                        .expect("just pushed")
                }
            };
            self.records[at].best_score = Some(score);
        }
        notice
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(track: &str, class: &str) -> Key {
        Key::new("HD", Some(track), "detonator", class)
    }

    #[test]
    fn a_run_is_a_record_only_when_strictly_above_the_best() {
        let mut store = Store::default();
        // No record reads 0, and 0 does not beat it: the original's first-run frame.
        assert_eq!(
            store.judge_score(&key("17_Track", "venom"), 0, true),
            ScoreRecord {
                new: false,
                value: 0
            }
        );
        assert_eq!(
            store.judge_score(&key("17_Track", "venom"), 5600, true),
            ScoreRecord {
                new: true,
                value: 5600
            }
        );
        assert_eq!(
            store.judge_score(&key("17_Track", "venom"), 5600, true),
            ScoreRecord {
                new: false,
                value: 5600
            }
        );
        assert_eq!(
            store.judge_score(&key("17_Track", "venom"), 100, true),
            ScoreRecord {
                new: false,
                value: 5600
            }
        );
    }

    #[test]
    fn the_best_is_the_circuit_and_directions_across_classes() {
        let mut store = Store::default();
        store.judge_score(&key("17_Track", "venom"), 900, true);
        assert_eq!(store.best_score(&key("17_Track", "phantom")), 900);
        assert_eq!(store.best_score(&key("18_Track", "venom")), 0);
    }

    #[test]
    fn an_unofficial_run_sets_no_record_and_shows_the_standing_best() {
        let mut store = Store::default();
        store.judge_score(&key("17_Track", "venom"), 100, true);
        assert_eq!(
            store.judge_score(&key("17_Track", "venom"), 900, false),
            ScoreRecord {
                new: false,
                value: 100
            }
        );
        assert_eq!(store.best_score(&key("17_Track", "venom")), 100);
    }
}
