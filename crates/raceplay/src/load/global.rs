//! `Data\XML\HandlingStats.xml`: the engine-wide block, and the four numbers a
//! race takes out of it for its own speed class.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`. A move, with one behaviour change carried in
//! with it: the per-class lookup is keyed on the **name** the disc spells rather
//! than on `oag_physics::SpeedClass`'s discriminant, so a title whose ladder is
//! not Pulse's reaches its own block. See [`resolve`].

use super::*;

/// What a race takes out of the engine-wide `<Global>` block.
///
/// Every field has a stated fallback and every fallback is *reported*, because
/// each one is a feature quietly not happening rather than a crash - and a speed
/// pad that does nothing reads as a bug in the force law, which is the wrong
/// place to look for it.
pub(super) struct GlobalTunables {
    /// `<GlobalClass><SpeedupPads/></GlobalClass>` for this race's rung.
    pub(super) pad_tunables: handling::SpeedupPads,
    /// `<Special/>`, of which one attribute is read.
    pub(super) special: handling::Special,
    /// `<GlobalClass><GravityMul airborne/></GlobalClass>`, **identity** when
    /// absent rather than zero: a zero would leave a grounded craft weightless,
    /// which is a broken race and not a degraded one.
    pub(super) class_gravity_scale: f32,
    /// `<Zone/>`, on Zone mode only.
    pub(super) zone: Option<handling::Zone>,
    /// `<GlobalClass><WeaponPad refresh_time/></GlobalClass>`.
    pub(super) weapon_pad_refresh: f32,
    /// `<StartBoost/>`, or `None` where the file authors none (Pure) or cannot
    /// be read: no launch boost, rather than one on invented numbers.
    pub(super) start_boost: Option<oag_physics::launch::StartBoost>,
    /// `<GlobalClass><PilotAssist/><PilotAssistPenalty/></GlobalClass>` for this
    /// rung, `None` where the title authors none (Pulse, Pure).
    pub(super) pilot_assist: Option<oag_physics::pilot_assist::Params>,
}

/// Reads the file and resolves the rung `options.class` names.
///
/// # The per-class lookup is by name
///
/// `Global::class_named` answers out of the four arrays for Pulse's rungs and
/// out of `Global::extra` for anything else, so Wipeout Pure's fifth
/// `<GlobalClass name="VECTOR">` is reachable and no title that authors four
/// grows a fifth entry. A rung this file does not author comes back `None` and
/// is **reported** - never filled in from a neighbouring rung, which would be
/// racing on borrowed numbers.
///
/// Every measured disc authors a `VECTOR` block here, Pulse's included, so the
/// presence of one says nothing about whether a title offers the rung. That is
/// decided by the per-team files, via `oag_title::SpeedClasses`.
pub(super) fn resolve(
    archives: &mut oag_assets::Archives,
    options: &Options,
    report: &mut Vec<String>,
) -> GlobalTunables {
    // Three distinct failures, reported as three distinct lines. The decoder was
    // deliberately tightened so an incomplete `<Global>` names the element,
    // attribute or class that is missing rather than returning a zero, and
    // collapsing that into one "unreadable" would throw the whole point away -
    // especially now that `<Zone>` and `<SpeedupPads>` share one result, so a
    // malformed pad block would otherwise be reported as a missing Zone law.
    let global = match archives.read_name(handling::GLOBAL_ENTRY) {
        Err(e) => {
            report.push(format!("{}: {e}", handling::GLOBAL_ENTRY));
            None
        }
        Ok(blob) => match handling::global_from_blob(&blob) {
            Err(e) => {
                report.push(format!("{}: {e}", handling::GLOBAL_ENTRY));
                None
            }
            Ok(None) => {
                report.push(format!(
                    "{} carries no <Global> block",
                    handling::GLOBAL_ENTRY
                ));
                None
            }
            Ok(some) => some,
        },
    };

    let class = global
        .as_ref()
        .and_then(|global| global.class_named(&options.class));
    if global.is_some() && class.is_none() {
        report.push(format!(
            "{}: no <GlobalClass name=\"{}\">, so no speed pads, unscaled gravity \
             and no weapon-pad debounce",
            handling::GLOBAL_ENTRY,
            options.class
        ));
    }

    // An absent or unreadable file means no boost and no auto-speed rather than
    // invented numbers. Said out loud, because a speed pad that quietly does
    // nothing reads as a physics bug and gets looked for in the force law.
    let pad_tunables = match class {
        Some((pads, _, _)) => pads,
        None => {
            report.push("speed pads apply no boost this run".to_string());
            handling::SpeedupPads::default()
        }
    };

    // `<Special speedpad_jump>`, out of the same file and with the same fallback
    // reasoning: a zero means the boost simply does not tilt, which is a missing
    // feature rather than a broken race. Reported for the same reason the pad
    // tunables are: a tilt that silently reads zero is indistinguishable from a
    // tilt that is not implemented, and this is the only place the number's
    // journey off the disc is observable.
    let special = global
        .as_ref()
        .map(|global| global.special)
        .unwrap_or_default();
    report.push(format!(
        "<Special speedpad_jump>: {} - the boost tilts {:.2} degrees toward the \
         hull's up while the pitch axis is held up",
        special.speedpad_jump,
        special.speedpad_jump.atan().to_degrees()
    ));

    // Named `airborne` in the XML and applied to the *grounded* term; see
    // `oag_tables::handling::GravityMul`, which reads the VFPU pair chain out.
    let class_gravity_scale = match class {
        Some((_, mul, _)) => {
            let scale = mul.airborne;
            report.push(format!(
                "<GravityMul>: grounded gravity scaled by {scale} for the {} class",
                options.class
            ));
            scale
        }
        None => {
            report.push("gravity is unscaled this run".to_string());
            1.0
        }
    };

    let zone = if options.mode == Mode::Zone {
        match global.as_ref() {
            Some(global) => report.push(format!(
                "<Zone>: start {}, increment {} per zone, recharge {}",
                global.zone.start, global.zone.increment, global.zone.recharge
            )),
            None => report.push("this run has no auto-speed".to_string()),
        }
        global.as_ref().map(|g| g.zone)
    } else {
        None
    };

    // The weapon-pad debounce, out of the same `<GlobalClass>` block as the two
    // above and with the same "absent means the feature is off" fallback. Zero
    // is a real degradation rather than a neutral value - it would let one
    // crossing grant a pickup on every tick the hull is inside the volume - so
    // the trigger treats a zero as "grant once and never again on this pad"
    // rather than trusting it; see `Race::test_weapon_pads`.
    //
    // **Eliminator reads a different figure, from 2026-09-08.**
    // `<WeaponPad elimination_refresh_time>` is an order of magnitude shorter
    // than the ordinary `refresh_time` on both shipped PSP discs (`0.05`
    // against `0.55`) - `oag_tables::handling::global::WeaponPad`'s own doc
    // comment already named this mode by name before anything read it. A
    // title with no second figure (Pure, which has no Eliminator) falls back
    // to the ordinary one.
    let weapon_pad_refresh = match class {
        Some((_, _, pad)) if options.mode == Mode::Eliminator => {
            pad.elimination_refresh_time.unwrap_or(pad.refresh_time)
        }
        Some((_, _, pad)) => pad.refresh_time,
        None => 0.0,
    };

    let start_boost = global
        .as_ref()
        .and_then(|global| global.start_boost)
        .map(|b| oag_physics::launch::StartBoost {
            window_start: b.window_start,
            window_end: b.window_end,
            stall_end: b.stall_end,
            overall_duration: b.overall_duration,
            stall_mul: b.stall_mul,
            normal_mul: b.normal_mul,
            boost_mul: b.boost_mul,
        });
    report.push(match start_boost {
        Some(b) => format!(
            "<StartBoost>: a launch boost for {} s after GO, graded by when thrust first lands",
            b.overall_duration
        ),
        None => "<StartBoost>: absent, so no launch boost this run".to_string(),
    });

    let pilot_assist = global
        .as_ref()
        .zip(handling::SpeedClass::from_name(&options.class))
        .and_then(|(global, class)| global.pilot_assist(class))
        .map(|p| oag_physics::pilot_assist::Params {
            la_dist_const: p.la_dist_const,
            la_dist_vel_mul: p.la_dist_vel_mul,
            la_dist_max: p.la_dist_max,
            spring_mul: p.spring_mul,
            torque_mul: p.torque_mul,
            max_torque: p.max_torque,
            max_ang_vel: p.max_ang_vel,
            general_thrust_percent: p.general_thrust_percent,
            thrust_percent_on_use: p.thrust_percent_on_use,
            penalty_duration: p.penalty_duration,
        });
    report.push(match pilot_assist {
        Some(_) => format!("<PilotAssist>: authored for the {} class", options.class),
        None => "<PilotAssist>: not authored, so no Pilot Assist this run".to_string(),
    });

    GlobalTunables {
        pilot_assist,
        pad_tunables,
        special,
        class_gravity_scale,
        start_boost,
        zone,
        weapon_pad_refresh,
    }
}
