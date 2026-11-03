//! A mod's own sounds (`[sounds]`, `play-mod-sound`): decoded by the race load, played
//! through the same mixer, limiter and spatial path as the title's cues, placed on the
//! craft that raised them. A loop follows its craft until stopped. An output: heard,
//! never hashed.

use std::sync::Arc;

use oag_audio::{Bus, Mixer, Play, Sound, VoiceId};
use oag_core::math::Vec3;
use oag_gameplay::cue::{ModSoundEvent, ModSoundMode};

use super::{Placed, Placement, place_on_craft};

/// Every mod's loaded sounds, by mod id, each in the mod's `[sounds]` order; a slot is
/// `None` where the file did not load.
#[derive(Debug, Clone, Default)]
pub struct ModSounds(pub Vec<(String, Vec<Option<Arc<Sound>>>)>);

impl ModSounds {
    /// The sound `index` of mod `mod_id`, if it loaded.
    #[must_use]
    pub fn get(&self, mod_id: &str, index: u16) -> Option<&Arc<Sound>> {
        self.0
            .iter()
            .find(|(id, _)| id == mod_id)?
            .1
            .get(usize::from(index))?
            .as_ref()
    }

    /// Whether no mod shipped a sound that loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0
            .iter()
            .all(|(_, sounds)| sounds.iter().all(Option::is_none))
    }
}

/// Where a mod sound is heard from: the craft's emitter, the player's own craft dry
/// (a title cue on the craft is placed the same way,
/// [`Placement::CraftUnlessPlayer`]).
const PLACEMENT: Placement = Placement::CraftUnlessPlayer;

/// The ears, every craft's pose and the player's grid slot.
pub(super) type Ears<'a> = (&'a oag_audio::Listener, &'a [Option<(Vec3, f32)>], usize);

/// The loops a race has running, by mod, sound and slot.
#[derive(Debug, Default)]
pub(super) struct Voices {
    held: Vec<(String, u16, u8, VoiceId)>,
}

impl Voices {
    /// Starts, stops and moves this tick's mod sounds. A finished race (`!running`) ends
    /// every loop: a module in the middle of one has stopped being called.
    pub(super) fn play(
        &mut self,
        events: &[ModSoundEvent],
        sounds: &ModSounds,
        mixer: &mut Mixer,
        (listener, craft, player_slot): Ears<'_>,
        running: bool,
    ) {
        for event in events {
            let slot = usize::from(event.slot);
            match event.mode {
                ModSoundMode::Stop => self.stop(&event.mod_id, event.index, event.slot, mixer),
                ModSoundMode::Once | ModSoundMode::Loop => {
                    let looping = event.mode == ModSoundMode::Loop;
                    if looping && self.holds(&event.mod_id, event.index, event.slot) {
                        continue;
                    }
                    let Some(sound) = sounds.get(&event.mod_id, event.index) else {
                        continue;
                    };
                    // A one-shot out of range is not started, as a title cue is not; a
                    // loop starts silent and is moved into range by `follow`.
                    let placed = place_on_craft(PLACEMENT, slot, listener, craft, player_slot);
                    let at = match placed {
                        Some(at) => at,
                        None if looping => SILENT,
                        None => continue,
                    };
                    let play = if looping {
                        Play::looping(Arc::clone(sound), Bus::Sfx)
                    } else {
                        Play::once(Arc::clone(sound), Bus::Sfx)
                    };
                    let id = mixer.play(Play {
                        gain: at.gain,
                        pan: at.pan,
                        front: at.front,
                        ..play
                    });
                    if let (true, Some(id)) = (looping, id) {
                        self.held
                            .push((event.mod_id.clone(), event.index, event.slot, id));
                    }
                }
            }
        }
        if !running {
            self.stop_all(mixer);
            return;
        }
        self.follow(mixer, listener, craft, player_slot);
    }

    fn holds(&self, mod_id: &str, index: u16, slot: u8) -> bool {
        self.held
            .iter()
            .any(|(m, i, s, _)| m == mod_id && *i == index && *s == slot)
    }

    fn stop(&mut self, mod_id: &str, index: u16, slot: u8, mixer: &mut Mixer) {
        self.held.retain(|(m, i, s, id)| {
            let this = m == mod_id && *i == index && *s == slot;
            if this {
                mixer.stop(*id);
            }
            !this
        });
    }

    /// Moves every loop with its craft, and ends one whose craft is gone or that the
    /// pool took back.
    fn follow(
        &mut self,
        mixer: &mut Mixer,
        listener: &oag_audio::Listener,
        craft: &[Option<(Vec3, f32)>],
        player_slot: usize,
    ) {
        self.held.retain(|&(_, _, slot, id)| {
            if !mixer.is_playing(id) {
                return false;
            }
            if craft.get(usize::from(slot)).is_none_or(Option::is_none) {
                mixer.stop(id);
                return false;
            }
            let at = place_on_craft(PLACEMENT, usize::from(slot), listener, craft, player_slot)
                .unwrap_or(SILENT);
            mixer.set_gain(id, at.gain);
            mixer.set_pan(id, at.pan);
            mixer.set_front(id, at.front);
            true
        });
    }

    /// Ends every loop.
    pub(super) fn stop_all(&mut self, mixer: &mut Mixer) {
        for (_, _, _, id) in self.held.drain(..) {
            mixer.stop(id);
        }
    }

    /// How many loops are running.
    pub(super) fn count(&self) -> usize {
        self.held.len()
    }
}

/// A loop whose craft is out of range: running, heard by nobody.
const SILENT: Placed = Placed {
    gain: 0.0,
    pan: None,
    front: 1.0,
};

#[cfg(test)]
mod tests {
    use super::*;
    use oag_audio::mixer::CHANNELS;

    const SLOTS: usize = oag_gameplay::MAX_SHIPS;

    fn tone(frames: usize) -> Arc<Sound> {
        Arc::new(Sound::new(vec![8000; frames], 1, 48_000).unwrap())
    }

    fn sounds(sound: &Arc<Sound>) -> ModSounds {
        ModSounds(vec![(
            "test.sound".to_string(),
            vec![Some(Arc::clone(sound)), None],
        )])
    }

    fn event(index: u16, slot: u8, mode: ModSoundMode) -> ModSoundEvent {
        ModSoundEvent {
            mod_id: "test.sound".to_string(),
            index,
            slot,
            mode,
        }
    }

    /// Every craft's pose: the listener looks down -z, so +x is to its right.
    fn craft(poses: &[(usize, [f32; 3])]) -> [Option<(Vec3, f32)>; SLOTS] {
        let mut out = [None; SLOTS];
        for &(slot, at) in poses {
            out[slot] = Some((Vec3::new(at[0], at[1], at[2]), 0.0));
        }
        out
    }

    struct Rig {
        mixer: Mixer,
        voices: Voices,
        sounds: ModSounds,
        listener: oag_audio::Listener,
    }

    impl Rig {
        fn new(sound: &Arc<Sound>) -> Self {
            Self {
                mixer: Mixer::new(48_000),
                voices: Voices::default(),
                sounds: sounds(sound),
                listener: oag_audio::Listener::at_origin(),
            }
        }

        fn tick(
            &mut self,
            events: &[ModSoundEvent],
            craft: &[Option<(Vec3, f32)>; SLOTS],
            running: bool,
        ) -> (f32, f32) {
            self.voices.play(
                events,
                &self.sounds,
                &mut self.mixer,
                (&self.listener, craft, 0),
                running,
            );
            // Two milliseconds, the same length every time, summed per ear.
            let mut out = vec![0.0; 96 * CHANNELS];
            self.mixer.render(&mut out);
            out.as_chunks::<2>()
                .0
                .iter()
                .fold((0.0, 0.0), |(l, r), f| (l + f[0].abs(), r + f[1].abs()))
        }
    }

    #[test]
    fn a_one_shot_comes_from_the_craft_that_asked_and_the_players_own_is_dry() {
        let sound = tone(4800);
        let at = craft(&[(0, [0.0, 0.0, 0.0]), (1, [60.0, 0.0, -60.0])]);

        let mut rig = Rig::new(&sound);
        let (left, right) = rig.tick(&[event(0, 1, ModSoundMode::Once)], &at, true);
        assert!(
            right > left * 1.5 && left > 0.0,
            "a rival to the right: {left} {right}"
        );

        let mut rig = Rig::new(&sound);
        let (left, right) = rig.tick(&[event(0, 0, ModSoundMode::Once)], &at, true);
        assert!(
            left > 0.0 && (left - right).abs() < left * 0.01,
            "own craft is dry: {left} {right}"
        );

        let mut rig = Rig::new(&sound);
        let far = craft(&[(0, [0.0; 3]), (1, [60_000.0, 0.0, 0.0])]);
        let (left, right) = rig.tick(&[event(0, 1, ModSoundMode::Once)], &far, true);
        assert_eq!((left, right), (0.0, 0.0), "out of range is not started");
        assert_eq!(rig.mixer.active_voices(), 0);
    }

    #[test]
    fn a_sound_that_did_not_load_plays_nothing() {
        let mut rig = Rig::new(&tone(4800));
        let at = craft(&[(0, [0.0; 3]), (1, [60.0, 0.0, -60.0])]);
        // Index 1 is a `None` slot, index 9 does not exist, the mod is unknown.
        let mut unknown = event(0, 1, ModSoundMode::Once);
        unknown.mod_id = "other".to_string();
        let events = [
            event(1, 1, ModSoundMode::Once),
            event(9, 1, ModSoundMode::Once),
            unknown,
        ];
        assert_eq!(rig.tick(&events, &at, true), (0.0, 0.0));
        assert_eq!(rig.mixer.active_voices(), 0);
    }

    #[test]
    fn a_loop_outlives_its_sound_follows_its_craft_and_stops_when_told() {
        // 100 frames of sound, played for many times its length.
        let mut rig = Rig::new(&tone(100));
        let right = craft(&[(0, [0.0; 3]), (1, [60.0, 0.0, -60.0])]);
        let left = craft(&[(0, [0.0; 3]), (1, [-60.0, 0.0, -60.0])]);

        let start = [event(0, 1, ModSoundMode::Loop)];
        let (l, r) = rig.tick(&start, &right, true);
        assert!(r > l, "starts at the craft: {l} {r}");
        assert_eq!(rig.voices.count(), 1);
        for _ in 0..5 {
            // Asked again every tick, as a module that holds a sound does.
            let (l, r) = rig.tick(&start, &right, true);
            assert!(r > l && l > 0.0, "still sounding: {l} {r}");
        }
        assert_eq!(rig.voices.count(), 1, "a running loop is not started twice");
        assert_eq!(rig.mixer.active_voices(), 1);

        // The mixer ramps a voice's pan and gain, so give the move a few steps.
        let mut moved = (0.0, 0.0);
        for _ in 0..20 {
            moved = rig.tick(&[], &left, true);
        }
        let (l, r) = moved;
        assert!(l > r, "the loop moved with the craft: {l} {r}");

        rig.tick(&[event(0, 1, ModSoundMode::Stop)], &left, true);
        assert_eq!(rig.voices.count(), 0);
        let mut tail = (1.0, 1.0);
        for _ in 0..8 {
            tail = rig.tick(&[], &left, true);
        }
        assert_eq!(tail, (0.0, 0.0), "silent after the stop");
        assert_eq!(rig.mixer.active_voices(), 0);
    }

    #[test]
    fn a_loop_ends_with_its_craft_and_with_the_race() {
        let sound = tone(100);
        let both = craft(&[(0, [0.0; 3]), (1, [60.0, 0.0, -60.0])]);
        let alone = craft(&[(0, [0.0; 3])]);

        let mut rig = Rig::new(&sound);
        rig.tick(&[event(0, 1, ModSoundMode::Loop)], &both, true);
        assert_eq!(rig.voices.count(), 1);
        rig.tick(&[], &alone, true);
        assert_eq!(rig.voices.count(), 0, "the slot's craft is gone");

        let mut rig = Rig::new(&sound);
        rig.tick(&[event(0, 1, ModSoundMode::Loop)], &both, true);
        rig.tick(&[], &both, false);
        assert_eq!(rig.voices.count(), 0, "a finished race ends it");

        let mut rig = Rig::new(&sound);
        rig.tick(&[event(0, 1, ModSoundMode::Loop)], &both, true);
        rig.voices.stop_all(&mut rig.mixer);
        assert_eq!(rig.voices.count(), 0, "leaving the race ends it");
    }

    #[test]
    fn a_loop_out_of_range_starts_silent_and_is_heard_when_the_craft_comes_in() {
        let mut rig = Rig::new(&tone(100));
        let far = craft(&[(0, [0.0; 3]), (1, [60_000.0, 0.0, 0.0])]);
        let near = craft(&[(0, [0.0; 3]), (1, [60.0, 0.0, -60.0])]);
        assert_eq!(
            rig.tick(&[event(0, 1, ModSoundMode::Loop)], &far, true),
            (0.0, 0.0)
        );
        assert_eq!(rig.voices.count(), 1);
        let (l, r) = rig.tick(&[], &near, true);
        assert!(l + r > 0.0, "{l} {r}");
    }

    #[test]
    fn a_sound_at_its_own_rate_lasts_its_own_time_in_the_mixers() {
        // Half a second at 22,050 Hz in a 48,000 Hz mixer: 24,000 output frames, not
        // the 11,025 of an unconverted copy.
        let sound = Arc::new(Sound::new(vec![8000; 11_025], 1, 22_050).unwrap());
        let mut rig = Rig::new(&sound);
        let at = craft(&[(0, [0.0; 3])]);
        rig.tick(&[event(0, 0, ModSoundMode::Once)], &at, true);
        let mut ticks = 1;
        while rig.mixer.active_voices() > 0 && ticks < 1000 {
            rig.tick(&[], &at, true);
            ticks += 1;
        }
        let frames = ticks * 96;
        assert!(
            (23_000..=25_000).contains(&frames),
            "{frames} output frames"
        );
    }
}
