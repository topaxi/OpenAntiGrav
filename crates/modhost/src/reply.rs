//! Reading a guest's reply: the record checks are `oag_ext_abi`'s, the ones that need
//! the observation are here. A broken rule is a [`Fault::Reply`], never a clamp.

use oag_ext_abi::{Command, IO_LEN, Pickup, Slot};

use crate::{Fault, Observation};

pub(crate) fn read(
    io: &[u8; IO_LEN],
    observation: &Observation<'_>,
) -> Result<Vec<Command>, Fault> {
    let reply = |e: oag_ext_abi::Malformed| Fault::Reply(e.to_string());
    let count = oag_ext_abi::reply_count(io).map_err(reply)?;
    let craft = |slot: Slot| observation.crafts.get(slot.index());
    let mut thrust_seen = 0_u32;
    let mut shield_seen = 0_u32;
    let mut slot_pads_seen = 0_u32;
    let mut shield_cmd_seen = 0_u32;
    let mut pads_seen: Vec<u16> = Vec::new();
    let mut hud_items = [0_usize; oag_ext_abi::MAX_CRAFTS];
    let mut commands = Vec::with_capacity(count);
    for i in 0..count {
        let command = oag_ext_abi::Command::decode_in(
            &oag_ext_abi::command_record(io, i),
            oag_ext_abi::hud::pool(io),
        )
        .map_err(|e| Fault::Reply(format!("command {i}: {e}")))?;
        if let Some(slot) = command.slot()
            && observation.active & slot.bit() == 0
        {
            return Err(Fault::Reply(format!(
                "command {i} names slot {}, which holds no craft",
                slot.index()
            )));
        }
        match command {
            Command::ScaleThrust { slot, .. } => {
                if thrust_seen & slot.bit() != 0 {
                    return Err(Fault::Reply(format!(
                        "two scale-thrust commands for slot {}",
                        slot.index()
                    )));
                }
                thrust_seen |= slot.bit();
            }
            Command::ShieldDelta { slot, amount } => {
                let max = craft(slot).map_or(0.0, |craft| craft.shield_max);
                if amount.abs() > max {
                    return Err(Fault::Reply(format!(
                        "shield-delta {amount} for slot {} is not within its maximum {max}",
                        slot.index()
                    )));
                }
                if shield_seen & slot.bit() != 0 {
                    return Err(Fault::Reply(format!(
                        "two shield-delta commands for slot {}",
                        slot.index()
                    )));
                }
                shield_seen |= slot.bit();
            }
            Command::GivePickup { slot } => {
                let pickup = craft(slot).map_or(Pickup::None, |craft| craft.pickup);
                if pickup != Pickup::None {
                    return Err(Fault::Reply(format!(
                        "give-pickup for slot {}, which already holds {pickup:?}",
                        slot.index()
                    )));
                }
            }
            Command::ClearPickup { slot } => {
                if craft(slot).map_or(Pickup::None, |craft| craft.pickup) != Pickup::Ours {
                    return Err(Fault::Reply(format!(
                        "clear-pickup for slot {}, which holds no mod pickup",
                        slot.index()
                    )));
                }
            }
            Command::ScalePad { pad, .. } => {
                if usize::from(pad) >= observation.speed_pads {
                    return Err(Fault::Reply(format!(
                        "scale-pad for speed pad {pad}, the track has {}",
                        observation.speed_pads
                    )));
                }
                if pads_seen.contains(&pad) {
                    return Err(Fault::Reply(format!(
                        "two scale-pad commands for pad {pad}"
                    )));
                }
                pads_seen.push(pad);
            }
            Command::ScaleSlotPads { slot, .. } => {
                if slot_pads_seen & slot.bit() != 0 {
                    return Err(Fault::Reply(format!(
                        "two scale-pad commands for slot {}",
                        slot.index()
                    )));
                }
                slot_pads_seen |= slot.bit();
            }
            Command::GiveShield { slot }
            | Command::HoldShield { slot }
            | Command::ClearShield { slot } => {
                if shield_cmd_seen & slot.bit() != 0 {
                    return Err(Fault::Reply(format!(
                        "two shield commands (give, hold or clear) for slot {}",
                        slot.index()
                    )));
                }
                shield_cmd_seen |= slot.bit();
            }
            Command::HudText { slot, .. } | Command::HudIcon { slot, .. } => {
                hud_items[slot.index()] += 1;
                if hud_items[slot.index()] > oag_ext_abi::MAX_HUD_ITEMS {
                    return Err(Fault::Reply(format!(
                        "more than {} hud-text and hud-icon items for slot {}",
                        oag_ext_abi::MAX_HUD_ITEMS,
                        slot.index()
                    )));
                }
            }
            Command::PickupRoll { .. }
            | Command::PlayCue { .. }
            | Command::FireEffect { .. }
            | Command::FireModEffect { .. }
            | Command::HudTint { .. }
            | Command::HudShake { .. }
            | Command::HudBarExtra { .. }
            | Command::SetWeather { .. }
            | Command::SetSun { .. }
            | Command::GradeSky { .. } => {}
        }
        commands.push(command);
    }
    Ok(commands)
}
