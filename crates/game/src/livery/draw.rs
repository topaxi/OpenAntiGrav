//! Which team flies each grid slot: Pulse's own AI roster draw.
//!
//! **Recovered off Pulse PSP, `FUN_08821bd4` (`0x08821bd4`)**, the function
//! the race-session constructor `FUN_08820d78` calls on every launch, and
//! confirmed live on seven Single Race launches with three different player
//! teams - see `docs/ghidra/functions/psp-pulse-usa/grid.md`, "Which team
//! flies which slot, recovered". Confidence 85 for what was seen live (the
//! player's team never among the AI, every other team on the grid, the order
//! different on every launch), lower for the parts read statically only (the
//! swap below, seven of more than eight with the DLC packs mounted).
//!
//! The original, in order:
//!
//! 1. Takes the team catalogue - `Type="Race"` entries only, which
//!    [`crate::catalogue::teams`] already filters - and drops the player's own
//!    team by exact name, any team whose name contains `Zone`, and any team
//!    not yet unlocked.
//! 2. Shuffles **every** eligible team with a naive swap,
//!    `for i in 0..n { swap(i, rand() % n) }` - not Fisher-Yates, and the bias
//!    that leaves is part of the law, so it is kept.
//! 3. Gives the first seven to the AI racers and the player the last racer
//!    index. This project's slot 0 is the player's, so the seven land on
//!    slots 1 to 7 in draw order.
//!
//! **What is this project's, not the original's:**
//!
//! - **The seed.** The original seeds `srand` from `sceKernelGetSystemTimeWide`,
//!   the wall clock, so a different grid every launch. The simulation may not
//!   read a clock (`docs/architecture/determinism.md`), so the draw takes the
//!   race seed through a generator of its own, salted by [`ROSTER_SALT`]
//!   (chosen, not measured). One seed, one grid: a replay rebuilds it and a
//!   Tournament's legs share it, which the original also does - its later
//!   legs keep the first leg's roster rather than drawing again.
//! - **The unlock filter is not applied.** The caller's list is what the front
//!   end offers, which is already what is unlocked.
//! - **A list shorter than the grid cycles.** The original loops forever
//!   collecting seven teams that do not exist; Pure and the PS2 set can be
//!   shorter than the grid, and a repeated livery beats a hang. Chosen, not
//!   measured, and the load report names the repeat.
//!
//! **Every title draws this way.** Pulse is the only one measured; Pure, HD and
//! the rest have no rule of their own on record, so they inherit Pulse's law
//! rather than an invented ordering.

/// Mixed into the race seed so the roster draw has a stream of its own and
/// moving it never shifts a value the world's generator hands out.
///
/// Chosen, not measured: the original's seed is the wall clock.
pub const ROSTER_SALT: u64 = 0x5245_4f52_4441_4d53;

/// Which team flies each grid slot, the player first.
///
/// `available` is the catalogue's own order, `seed` the race's own
/// ([`crate::race::Options::seed`] resolved, [`crate::race::SEED`] when unset). Deterministic in all three
/// inputs. An empty list, or one holding only the player, gives every slot
/// the player's team, which is what a source with no readable definition gets.
#[must_use]
pub fn teams_for_slots(player: &str, available: &[String], slots: usize, seed: u64) -> Vec<String> {
    let mut out = Vec::with_capacity(slots);
    out.push(player.to_string());
    let mut eligible: Vec<&String> = available
        .iter()
        .filter(|team| *team != player && !team.contains("Zone"))
        .collect();
    if eligible.is_empty() {
        out.resize(slots, player.to_string());
        return out;
    }
    shuffle(&mut eligible, seed);
    for slot in 1..slots {
        out.push(eligible[(slot - 1) % eligible.len()].clone());
    }
    out
}

/// The original's swap: each index in turn trades places with `rand() % n`.
fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut rng = oag_core::rng::Rng::new(seed ^ ROSTER_SALT);
    let n = items.len();
    let Ok(n32) = u32::try_from(n) else {
        return;
    };
    for i in 0..n {
        let j = (rng.next_u32() % n32) as usize;
        items.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    fn pulse() -> Vec<String> {
        ids(&[
            "AG_Systems",
            "Assegai",
            "EGX",
            "Feisar",
            "Goteki",
            "Piranha",
            "Qirex",
            "Triakis",
        ])
    }

    /// Measured live on all seven launches: the player's team never flies an
    /// AI slot, and the other seven all do, exactly once.
    #[test]
    fn the_ai_are_every_other_team_and_never_the_players() {
        for seed in 0..64 {
            for player in pulse() {
                let slots = teams_for_slots(&player, &pulse(), 8, seed);
                assert_eq!(slots[0], player);
                let mut ai = slots[1..].to_vec();
                assert!(!ai.contains(&player), "seed {seed}: {slots:?}");
                ai.sort();
                let mut others: Vec<String> =
                    pulse().into_iter().filter(|t| *t != player).collect();
                others.sort();
                assert_eq!(ai, others, "seed {seed}");
            }
        }
    }

    /// The law that dropping the draw would lose: the order varies. A
    /// file-order fill puts the same team on slot 1 for every seed.
    #[test]
    fn every_other_team_reaches_the_first_ai_slot_across_seeds() {
        let mut seen: Vec<String> = (0..256)
            .map(|seed| teams_for_slots("Assegai", &pulse(), 8, seed)[1].clone())
            .collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 7, "{seen:?}");
    }

    #[test]
    fn one_seed_is_one_grid() {
        for seed in [0, 1, 7, u64::MAX] {
            assert_eq!(
                teams_for_slots("Feisar", &pulse(), 8, seed),
                teams_for_slots("Feisar", &pulse(), 8, seed)
            );
        }
    }

    /// The DLC case: twelve teams, eleven eligible, seven race - and which
    /// seven is the draw's, so a team past the first seven in file order
    /// reaches the grid for some seed. Truncating in file order never would.
    #[test]
    fn with_more_teams_than_slots_a_late_team_can_race() {
        let mut twelve = pulse();
        twelve.extend(ids(&["Harimau", "Icaras", "Mirage", "Tigron"]));
        let late = ["Harimau", "Icaras", "Mirage", "Tigron"];
        let raced = (0..64).any(|seed| {
            teams_for_slots("AG_Systems", &twelve, 8, seed)
                .iter()
                .any(|team| late.contains(&team.as_str()))
        });
        assert!(raced);
        for seed in 0..64 {
            let slots = teams_for_slots("AG_Systems", &twelve, 8, seed);
            let mut unique = slots.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(
                unique.len(),
                8,
                "no repeat while eleven are eligible: {slots:?}"
            );
        }
    }

    /// `FUN_08821dd4` drops any name containing `Zone`, case-sensitively.
    #[test]
    fn a_zone_named_team_never_races() {
        let mut with_zone = pulse();
        with_zone.push("Zone".to_string());
        for seed in 0..32 {
            let slots = teams_for_slots("Feisar", &with_zone, 8, seed);
            assert!(!slots.iter().any(|t| t == "Zone"), "{slots:?}");
        }
    }

    /// The Pure and PS2 case, chosen rather than measured: a repeated livery
    /// beats the original's hang.
    #[test]
    fn a_short_list_cycles_through_every_other_team() {
        let slots = teams_for_slots("a", &ids(&["a", "b", "c"]), 6, 3);
        assert_eq!(slots[0], "a");
        assert!(slots[1..].iter().all(|t| t == "b" || t == "c"), "{slots:?}");
        assert_ne!(slots[1], slots[2], "both others before either repeats");
    }

    #[test]
    fn no_other_teams_means_the_whole_grid_wears_the_players_hull() {
        assert_eq!(teams_for_slots("a", &[], 4, 1), ids(&["a", "a", "a", "a"]));
        assert_eq!(
            teams_for_slots("a", &ids(&["a"]), 4, 1),
            ids(&["a", "a", "a", "a"])
        );
    }

    #[test]
    fn one_slot_is_just_the_player() {
        assert_eq!(teams_for_slots("a", &ids(&["a", "b"]), 1, 1), ids(&["a"]));
    }
}
