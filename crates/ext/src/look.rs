//! The title's own pickup looks, one constant per weapon: `icon::title::TURBO` is the
//! title's icon sprite for a Turbo, `colour::title::SHIELD` its backdrop row for a
//! Shield. A title with none for a weapon draws none; Pure has no backdrop widget at
//! all, so a `colour::title::*` draws nothing there.

/// A pickup's icon.
pub mod icon {
    use oag_ext_abi::{Icon, Weapon};

    /// The title's own icon for each weapon, as an [`Icon`].
    pub mod title {
        use super::{Icon, Weapon};

        macro_rules! icons {
            ($($name:ident = $weapon:ident),* $(,)?) => {
                $(
                    #[doc = concat!("The title's own `", stringify!($weapon), "` icon.")]
                    pub const $name: Icon = Icon::Title(Weapon::$weapon);
                )*
            };
        }
        icons! {
            ROCKET = Rocket, MISSILE = Missile, QUAKE = Quake, CANNON = Cannon,
            TURBO = Turbo, SHIELD = Shield, AUTOPILOT = Autopilot, PLASMA = Plasma,
            BOMB = Bomb, MINE = Mine, LEACH_BEAM = LeachBeam, REPULSER = Repulser,
            SHURIKEN = Shuriken, DISRUPTOR = Disruptor,
        }
    }
}

/// A pickup's backdrop colour.
pub mod colour {
    use oag_ext_abi::{Colour, Weapon};

    /// The title's own backdrop colour for each weapon, as a [`Colour`].
    pub mod title {
        use super::{Colour, Weapon};

        macro_rules! colours {
            ($($name:ident = $weapon:ident),* $(,)?) => {
                $(
                    #[doc = concat!("The title's own `", stringify!($weapon), "` backdrop.")]
                    pub const $name: Colour = Colour::Title(Weapon::$weapon);
                )*
            };
        }
        colours! {
            ROCKET = Rocket, MISSILE = Missile, QUAKE = Quake, CANNON = Cannon,
            TURBO = Turbo, SHIELD = Shield, AUTOPILOT = Autopilot, PLASMA = Plasma,
            BOMB = Bomb, MINE = Mine, LEACH_BEAM = LeachBeam, REPULSER = Repulser,
            SHURIKEN = Shuriken, DISRUPTOR = Disruptor,
        }
    }
}
