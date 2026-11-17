//! What an online room holds fixed beyond a launch, as the header's `options` table spells it,
//! so a recording of a network race rebuilds as that race and not as a local one.
//!
//! **One mechanism for writing and rebuilding**, as [`super::Launch`]: `oag-net` builds a
//! [`Room`] from the room's parameters, applies it to its race ([`Race::apply_room`]) and writes
//! [`Room::to_table`] into the recording's header; [`super::apply_recorded`] reads the table back
//! and applies it the same way. Without it the rebuild seats every person as local, and
//! `World::controllers` is hashed, so the tick-zero hash would refuse the file.

use std::collections::BTreeMap;

use oag_gameplay::Controller;
use oag_physics::pilot_assist::Level;

use super::UnknownOption;
use crate::Race;

/// The rules of an online room that change the race and are not in a launch request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Room {
    /// How many grid slots, from slot 0, are people at another machine.
    pub people: u8,
    /// The Pilot Assist level each of them flies, in seat order; a seat past the end flies Off.
    pub assists: Vec<Level>,
    /// Whether barrel rolls are allowed.
    pub barrel_rolls: bool,
    /// The zone a Zone race ends at; `None` for the original's own Zone.
    pub zone_target: Option<u16>,
}

impl Room {
    /// The keys a room adds to a header's `options` table.
    #[must_use]
    pub fn to_table(&self) -> BTreeMap<String, String> {
        let mut table = BTreeMap::new();
        table.insert("room_people".to_string(), self.people.to_string());
        let assists: Vec<&str> = self.assists.iter().map(|level| level.name()).collect();
        table.insert("room_assists".to_string(), assists.join(","));
        table.insert("room_barrel_rolls".to_string(), self.barrel_rolls.to_string());
        if let Some(zone) = self.zone_target {
            table.insert("room_zone_target".to_string(), zone.to_string());
        }
        table
    }

    /// Reads a header's `options` table: `None` for a race that was not an online room's.
    ///
    /// # Errors
    ///
    /// [`UnknownOption`] for a value this build does not know.
    pub fn from_table(table: &BTreeMap<String, String>) -> Result<Option<Self>, UnknownOption> {
        let unknown = |key: &'static str, value: &str| UnknownOption {
            key,
            value: value.to_string(),
        };
        let Some(people) = table.get("room_people") else {
            return Ok(None);
        };
        let people = people
            .parse()
            .map_err(|_| unknown("room_people", people))?;
        let mut assists = Vec::new();
        for name in table
            .get("room_assists")
            .map_or("", String::as_str)
            .split(',')
            .filter(|name| !name.is_empty())
        {
            assists.push(Level::from_name(name).ok_or_else(|| unknown("room_assists", name))?);
        }
        let barrel_rolls = match table.get("room_barrel_rolls").map(String::as_str) {
            Some("true") | None => true,
            Some("false") => false,
            Some(value) => return Err(unknown("room_barrel_rolls", value)),
        };
        let zone_target = match table.get("room_zone_target") {
            None => None,
            Some(value) => Some(
                value
                    .parse()
                    .map_err(|_| unknown("room_zone_target", value))?,
            ),
        };
        Ok(Some(Self {
            people,
            assists,
            barrel_rolls,
            zone_target,
        }))
    }
}

impl Race {
    /// Applies an online room's rules to a race just started: its people are remote, each flies
    /// the assist level they asked for, and the barrel-roll and zone rules are the room's. What
    /// the network race and the rebuild of its recording both call.
    pub fn apply_room(&mut self, room: &Room) {
        let seats = usize::from(room.people).min(oag_gameplay::MAX_PLAYERS);
        for controller in self.sim.world.controllers.iter_mut().take(seats) {
            *controller = Controller::Remote;
        }
        for slot in 0..seats {
            self.set_pilot_assist_seat(slot, room.assists.get(slot).copied().unwrap_or_default());
        }
        self.set_barrel_rolls(room.barrel_rolls);
        self.set_zone_target(room.zone_target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_room_round_trips_through_the_table_and_a_local_race_has_none() {
        for room in [
            Room {
                people: 2,
                assists: vec![Level::Off, Level::Extreme],
                barrel_rolls: false,
                zone_target: Some(12),
            },
            Room {
                people: 1,
                assists: Vec::new(),
                barrel_rolls: true,
                zone_target: None,
            },
        ] {
            assert_eq!(Room::from_table(&room.to_table()), Ok(Some(room)));
        }
        assert_eq!(Room::from_table(&BTreeMap::new()), Ok(None));
        let mut table = BTreeMap::new();
        table.insert("room_people".to_string(), "two".to_string());
        assert_eq!(Room::from_table(&table).unwrap_err().key, "room_people");
    }
}
