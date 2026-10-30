//! The track region as a guest reads it: the track, its pads, its triggers, its spline
//! paths and its routes, each with its box and the properties a track file authors
//! (`mod_*` in the Blender add-on). The engine gives no property a meaning; a mod reads one by
//! [`prop!`](crate::prop).

use oag_ext_abi::track::{EntityView, Kind, Region};

/// The race's track. A disc circuit has its pads and no properties; a race with no
/// pads at all reads as an empty track.
#[derive(Debug, Clone, Copy)]
pub struct Track<'a> {
    region: Region<'a>,
}

impl<'a> Track<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Self {
            region: Region::new(bytes),
        }
    }

    /// The track itself: its own properties (`[props]` in `.track.toml`).
    #[must_use]
    pub fn whole(&self) -> Entity<'a> {
        self.entity(Kind::Track, 0).unwrap_or(Entity {
            view: None,
            kind: Kind::Track,
        })
    }

    /// How many speed pads it has: a [`Event::SpeedPad`](crate::Event::SpeedPad)'s
    /// `pad` is below it.
    #[must_use]
    pub fn speed_pads(&self) -> usize {
        self.region.count(Kind::SpeedPad)
    }

    /// How many weapon pads it has.
    #[must_use]
    pub fn weapon_pads(&self) -> usize {
        self.region.count(Kind::WeaponPad)
    }

    /// How many triggers it has.
    #[must_use]
    pub fn triggers(&self) -> usize {
        self.region.count(Kind::Trigger)
    }

    /// How many paths its spline graph has: one for a plain loop.
    #[must_use]
    pub fn paths(&self) -> usize {
        self.region.count(Kind::Path)
    }

    /// How many routes it has: ways from a fork on the lap back to it, none for a plain
    /// loop. A [`Reply::prefer_route`](crate::Reply::prefer_route)'s route is below it.
    #[must_use]
    pub fn routes(&self) -> usize {
        self.region.count(Kind::Route)
    }

    /// The `pad`-th speed pad.
    #[must_use]
    pub fn speed_pad(&self, pad: u16) -> Option<Entity<'a>> {
        self.entity(Kind::SpeedPad, usize::from(pad))
    }

    /// The `pad`-th weapon pad.
    #[must_use]
    pub fn weapon_pad(&self, pad: u16) -> Option<Entity<'a>> {
        self.entity(Kind::WeaponPad, usize::from(pad))
    }

    /// The `trigger`-th trigger.
    #[must_use]
    pub fn trigger(&self, trigger: u16) -> Option<Entity<'a>> {
        self.entity(Kind::Trigger, usize::from(trigger))
    }

    /// The `path`-th spline path: its properties (`mod_*` on the Blender spline) and
    /// the bounds of its points.
    #[must_use]
    pub fn path(&self, path: u16) -> Option<Entity<'a>> {
        self.entity(Kind::Path, usize::from(path))
    }

    /// The `route`-th route: the properties of the paths it drives (the first path's
    /// value where two set one name) and the bounds of its samples.
    #[must_use]
    pub fn route(&self, route: u16) -> Option<Entity<'a>> {
        self.entity(Kind::Route, usize::from(route))
    }

    fn entity(&self, kind: Kind, index: usize) -> Option<Entity<'a>> {
        self.region.entity(kind, index).map(|view| Entity {
            view: Some(view),
            kind,
        })
    }
}

/// One box of the track, or the track itself.
#[derive(Debug, Clone, Copy)]
pub struct Entity<'a> {
    view: Option<EntityView<'a>>,
    kind: Kind,
}

impl Entity<'_> {
    /// What it is.
    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The property `key` names ([`prop!`](crate::prop)), if the author set it. A bool
    /// reads as `1.0` or `0.0`.
    #[must_use]
    pub fn prop(&self, key: u32) -> Option<f32> {
        self.view?.prop(key)
    }

    /// Local to world, row-major, translation in row 3 (the identity for the track).
    #[must_use]
    pub fn to_world(&self) -> [f32; 16] {
        self.view
            .map_or(oag_ext_abi::track::IDENTITY, |view| view.to_world())
    }

    /// The box, local space: minimum and maximum. Empty for the track.
    #[must_use]
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        self.view
            .map_or(([0.0; 3], [0.0; 3]), |view| (view.min(), view.max()))
    }
}

/// A property's key, hashed at compile time: `prop!("boost")` is the key of a track
/// file's `mod_boost`.
#[macro_export]
macro_rules! prop {
    ($name:literal) => {{
        const KEY: u32 = $crate::prop_key($name);
        KEY
    }};
}
