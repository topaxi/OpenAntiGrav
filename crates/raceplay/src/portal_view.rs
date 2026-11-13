//! The camera a mod's `portal-view` looks through: the frames of a portal pair off the road the
//! course runs on, whether the view being drawn has the portal in it, and the draw-side override
//! of [`Race::view`] while the scene draws the secondary view. No part of a transit: a craft
//! goes through a portal in `crate::transit`, which shares the road frame and nothing else.

use oag_core::math::frustum::Frustum;
use oag_core::math::{Mat4, Vec3, Vec4};
use oag_render::camera::portal::{self, Face};
use oag_render::pvs::UNPLACED;

use crate::Race;
use crate::extensions::PortalRequest;
use crate::transit::Exit;

/// A portal pair as the view being drawn sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PortalShot {
    /// What the surface reaches, which the main view's visible sets are asked about.
    pub(crate) reach: Reach,
    /// The view matrix of the camera through the pair.
    pub(crate) view: Mat4,
    /// The exit's plane in [`Self::view`]'s eye space.
    pub(crate) exit_plane: Vec4,
    /// What takes a world point at the entrance to `engine_uv`'s clip space: the secondary
    /// camera after [`portal::through`], so the surface reads the picture where it stands.
    pub(crate) engine_view_projection: Mat4,
}

/// The part of the track a portal's surface reaches: a sphere of the request's radius about the
/// entrance's centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Reach {
    /// The authored sections of the road inside the sphere, one bit each, and [`ALWAYS`] where
    /// one of them is not named: a section partition masks nothing it cannot place.
    pub(crate) sections: u64,
    pub(crate) centre: Vec3,
    pub(crate) radius: f32,
}

impl Race {
    /// A portal's frame at `exit`, on the road's own up: the spline sample's banking, not the
    /// course's tangent alone. A ring point faces along the ring; a route's exit faces along the
    /// route's own deck, which leaves the ring's direction at a fork.
    pub(crate) fn portal_frame(&self, exit: Exit, face: Face) -> Option<Mat4> {
        let course = self.sim.course.as_ref()?;
        let (_, up, centre, _) = self.road_frame(exit)?;
        let along = if exit.route == 0 {
            course.tangent(exit.to)?
        } else {
            Vec3::from_array(self.road_sample(exit)?.tangent)
        };
        portal::frame(centre, along, up, face)
    }

    /// `request` as the view being drawn sees it through `projection`; `None` when a progress
    /// is off the lap, or the entrance's face is not towards the camera or is outside the view,
    /// so nothing of it can be on the screen.
    pub(crate) fn portal_shot(
        &self,
        request: &PortalRequest,
        projection: Mat4,
    ) -> Option<PortalShot> {
        let course = self.sim.course.as_ref()?;
        let entrance = Exit::ring(course.index_at_progress(request.entrance)?);
        let entrance_section = self
            .road_sample(entrance)
            .map_or(UNPLACED, |sample| sample.section_id);
        let entrance = self.portal_frame(entrance, Face::Oncoming)?;
        let exit = self.portal_frame(request.exit?, Face::Departing)?;
        // A craft in a transit has flown on past its entrance; its pair is seen from its seat.
        let main = self.pair_view();
        let centre = entrance.w_axis.truncate();
        let eye = main.inverse().w_axis.truncate();
        let facing = (eye - centre).dot(entrance.z_axis.truncate()) > 0.0;
        let in_view = Frustum::from_view_projection(projection * main)
            .intersects_sphere(centre, request.radius);
        if !facing || !in_view {
            return None;
        }
        let reach = Reach {
            sections: crate::spline::section_bit(entrance_section)
                | self.sim.spline.sections_within(centre, request.radius),
            centre,
            radius: request.radius,
        };
        let through = portal::through(entrance, exit);
        let view = portal::view(main, through);
        Some(PortalShot {
            reach,
            view,
            exit_plane: portal::eye_plane(view, exit),
            engine_view_projection: projection * view * through,
        })
    }

    /// Draws the next views from `view` until it is put back with `None`. Draw-side state behind
    /// a shared reference, as [`Self::set_viewpoint`] is.
    pub(crate) fn set_portal_camera(&self, view: Option<Mat4>) {
        self.view.viewpoints.set_portal(view);
    }
}
