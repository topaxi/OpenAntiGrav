//! A mod pickup's look and the ABI's weapon vocabulary: what `pickup-look` and a
//! manifest's `[pickup]` name, as the tables' own types the HUD reads
//! (`oag_tables::pickup_look`).
//!
//! The look is an output: resolved per tick for the slot's HUD, never hashed. The title's
//! fade plays on whatever it is.

use oag_ext_abi::{Colour as AbiColour, Command, Icon as AbiIcon, Pickup};
use oag_tables::pickup_look::{Colour, Icon, PickupLook};

/// The ABI's weapon vocabulary and the tables' weapons, both ways. The tables' own
/// `Extension` is the mod's pickup, which the ABI names [`Pickup::Ours`].
pub(crate) fn pickup(weapon: oag_tables::weapons::Weapon) -> Pickup {
    use oag_ext_abi::Weapon as Abi;
    use oag_tables::weapons::Weapon;
    Pickup::Weapon(match weapon {
        Weapon::Rocket => Abi::Rocket,
        Weapon::Missile => Abi::Missile,
        Weapon::Quake => Abi::Quake,
        Weapon::Cannon => Abi::Cannon,
        Weapon::Turbo => Abi::Turbo,
        Weapon::Shield => Abi::Shield,
        Weapon::Autopilot => Abi::Autopilot,
        Weapon::Plasma => Abi::Plasma,
        Weapon::Bomb => Abi::Bomb,
        Weapon::Mine => Abi::Mine,
        Weapon::LeachBeam => Abi::LeachBeam,
        Weapon::Repulser => Abi::Repulser,
        Weapon::Shuriken => Abi::Shuriken,
        Weapon::Disruptor => Abi::Disruptor,
        Weapon::Extension => return Pickup::Ours,
    })
}

pub(crate) fn weapon(weapon: oag_ext_abi::Weapon) -> oag_tables::weapons::Weapon {
    use oag_ext_abi::Weapon as Abi;
    use oag_tables::weapons::Weapon;
    match weapon {
        Abi::Rocket => Weapon::Rocket,
        Abi::Missile => Weapon::Missile,
        Abi::Quake => Weapon::Quake,
        Abi::Cannon => Weapon::Cannon,
        Abi::Turbo => Weapon::Turbo,
        Abi::Shield => Weapon::Shield,
        Abi::Autopilot => Weapon::Autopilot,
        Abi::Plasma => Weapon::Plasma,
        Abi::Bomb => Weapon::Bomb,
        Abi::Mine => Weapon::Mine,
        Abi::LeachBeam => Weapon::LeachBeam,
        Abi::Repulser => Weapon::Repulser,
        Abi::Shuriken => Weapon::Shuriken,
        Abi::Disruptor => Weapon::Disruptor,
    }
}

/// The look a `pickup-look` command names; a mod's own icon is keyed into the HUD sheet
/// for `mod_id` ([`crate::extensions::mod_icon_src`]).
pub(crate) fn from_command(command: &Command, mod_id: &str) -> Option<PickupLook> {
    let Command::PickupLook { icon, colour, .. } = command else {
        return None;
    };
    Some(PickupLook {
        icon: match icon {
            AbiIcon::Title(w) => Icon::Title(weapon(*w)),
            AbiIcon::Mod(name) => Icon::Mod(crate::extensions::mod_icon_src(mod_id, name.as_str())),
        },
        colour: match colour {
            AbiColour::Title(w) => Colour::Title(weapon(*w)),
            AbiColour::Rgba(rgba) => Colour::Rgba([rgba.r, rgba.g, rgba.b, rgba.a]),
        },
    })
}

/// The look of `slot`'s mod pickup this tick: the last `pickup-look` among `hud` named for
/// it, else the `[pickup]` of the module that owns the mod pickup (`owner`, its index plus
/// one). `None` in a race without one.
pub(crate) fn resolve<'a>(
    hud: &[(usize, Command)],
    slot: usize,
    owner: u8,
    pickups: &[Option<PickupLook>],
    mod_id: impl Fn(usize) -> &'a str,
) -> Option<PickupLook> {
    let sent = hud
        .iter()
        .rev()
        .filter(|(_, command)| command.slot().map(|s| s.index()) == Some(slot))
        .find_map(|(module, command)| from_command(command, mod_id(*module)));
    sent.or_else(|| {
        let owner = usize::from(owner).checked_sub(1)?;
        Some(from_manifest(pickups.get(owner)?.as_ref()?, mod_id(owner)))
    })
}

/// A manifest's `[pickup]` with its `[icons]` key turned into the HUD sheet's key.
pub(crate) fn from_manifest(look: &PickupLook, mod_id: &str) -> PickupLook {
    PickupLook {
        icon: match &look.icon {
            Icon::Mod(name) => Icon::Mod(crate::extensions::mod_icon_src(mod_id, name)),
            title => title.clone(),
        },
        colour: look.colour,
    }
}

/// A look for a test module that hands a pickup out: the Turbo's icon on a red backdrop.
#[cfg(test)]
pub(crate) fn a_look() -> PickupLook {
    PickupLook {
        icon: Icon::Title(oag_tables::weapons::Weapon::Turbo),
        colour: Colour::Rgba([0xE0, 0x20, 0x20, 0x80]),
    }
}
