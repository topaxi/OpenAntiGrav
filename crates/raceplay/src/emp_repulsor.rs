//! The Repulsor's opening borrowed by a Detonator EMP, for a title with no EMP of its own:
//! the blue screen flash, `WO_REPULSER_BLAST` riding the firer and the field ring
//! (`pulse_repulsorwave.vex`) shrinking in and widening out around it, as a fired Repulsor
//! draws them (`docs/gameplay/detonator-pulse.md`, "The EMP").

use oag_title::Trigger;
use oag_weapons::detonator::EventKind;

use super::*;

/// How long the ring and the blast stay, the Repulsor's own life.
const LIFE_SECONDS: f32 = 1.6;

/// When the ring stops fading in and widens away: the share of the Repulsor's life its
/// charge takes, shortened because the EMP's wave leaves at once.
const WIDEN_SECONDS: f32 = 0.2;

/// One launch's borrowed opening: view state only.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EmpRepulsor {
    pub(crate) age: f32,
    pub(crate) field: crate::repulser_field::Field,
    blast: Option<psys::Playing>,
}

impl RaceView {
    /// Starts the opening on the tick the EMP launches, then steps and ends it.
    pub(crate) fn advance_emp_repulsor(&mut self, core: &SimRace) {
        if !self.detonator_emp_repulsor {
            return;
        }
        let slot = core.sim.world.primary_slot();
        let firer = core.sim.world.ships[slot].physics.body.position;
        let launched = core
            .sim
            .detonator_events
            .iter()
            .any(|event| event.kind == EventKind::EmpLaunch);
        if launched {
            if let Some(flash) = &mut self.screen_flash {
                flash.start(oag_fx::flash::REPULSER, firer);
            }
            self.release_emp_repulsor();
            let blast = self
                .handles
                .get(Trigger::RepulserBlast)
                .cloned()
                .and_then(|effect| self.stage.attach(&effect, firer, 1.0));
            if let Some(playing) = blast {
                self.stage.set_rides_frame(playing, true);
            }
            self.emp_repulsor = Some(EmpRepulsor {
                age: 0.0,
                field: crate::repulser_field::Field::new(),
                blast,
            });
        }
        let Some(mut opening) = self.emp_repulsor else {
            return;
        };
        opening.age += core.sim.dt;
        if opening.age >= LIFE_SECONDS {
            self.release_emp_repulsor();
            return;
        }
        opening
            .field
            .step(opening.age, opening.age >= WIDEN_SECONDS);
        if let (Some(playing), Some((_, sample, _))) =
            (opening.blast, core.sim.spline.nearest(firer))
        {
            let (side, up, _) = crate::repulser_field::field_basis(
                Vec3::from_array(sample.lateral),
                Vec3::from_array(sample.down),
            );
            self.stage.follow(playing, firer);
            self.stage.orient(playing, up);
            let spun = oag_core::math::Quat::from_axis_angle(up, opening.field.spin) * side;
            self.stage.stretch(playing, 1.0, spun);
        }
        self.emp_repulsor = Some(opening);
    }

    fn release_emp_repulsor(&mut self) {
        if let Some(playing) = self.emp_repulsor.take().and_then(|opening| opening.blast) {
            self.stage.release(playing);
        }
    }
}

impl Race {
    /// The borrowed ring's matrix, alpha and age, in the form the Repulsor pool's own draws
    /// take. Drawn through the pool's first slot, which a Detonator race (no pickups, so no
    /// Repulsor) never fills.
    pub(crate) fn emp_repulsor_draw(&self) -> Option<(Mat4, f32, f32)> {
        let opening = self.view.emp_repulsor?;
        let slot = self.core.sim.world.primary_slot();
        let firer = self.core.sim.world.ships[slot].physics.body.position;
        let (_, sample, _) = self.core.sim.spline.nearest(firer)?;
        let matrix = crate::repulser_field::field_matrix(
            firer,
            Vec3::from_array(sample.lateral),
            Vec3::from_array(sample.down),
            opening.field.scale(),
        );
        Some((matrix, opening.field.drawn_alpha(), opening.age))
    }
}
