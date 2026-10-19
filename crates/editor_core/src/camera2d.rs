//! The camera of a 2D map view.
//!
//! A 2D view looks down one of the coordinate axes, so it is fully described by which
//! axis that is, where the camera is centred in world space, and how many pixels one
//! world unit occupies.

use crate::i18n::translation_key;
use geometry_math::vec::{self, Vec2d, Vec3d};

/// The axis a 2D view looks down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPlane {
    /// Looking down -Z: the XY plane, the "top" view.
    Top,
    /// Looking down +Y: the XZ plane, the "front" view.
    Front,
    /// Looking down -X: the YZ plane, the "side" view.
    Side,
}

impl ViewPlane {
    /// Projects a world point onto this view plane.
    pub fn project(self, p: Vec3d) -> Vec2d {
        match self {
            // Y grows upward on screen in all three views, as map editors conventionally draw them.
            ViewPlane::Top => Vec2d::new(p.x(), p.y()),
            ViewPlane::Front => Vec2d::new(p.x(), p.z()),
            ViewPlane::Side => Vec2d::new(p.y(), p.z()),
        }
    }

    /// Lifts a point on this view plane back into world space, placing it at `depth`
    /// along the axis this view looks down.
    pub fn unproject(self, p: Vec2d, depth: f64) -> Vec3d {
        match self {
            ViewPlane::Top => Vec3d::new(p.x(), p.y(), depth),
            ViewPlane::Front => Vec3d::new(p.x(), depth, p.y()),
            ViewPlane::Side => Vec3d::new(depth, p.x(), p.y()),
        }
    }

    /// Returns the world axes this view's two screen axes correspond to.
    ///
    /// Top shows X and Y, Front shows X and Z, Side shows Y and Z. Two views share
    /// whichever axes appear in both lists, which is what makes them line up when linked.
    pub fn shown_axes(self) -> [usize; 2] {
        match self {
            ViewPlane::Top => [0, 1],
            ViewPlane::Front => [0, 2],
            ViewPlane::Side => [1, 2],
        }
    }

    /// Returns the index of the axis this view looks down, and so cannot show.
    ///
    /// A drag in a 2D view says nothing about this axis, so anything the drag creates has
    /// to get its extent along it from somewhere else.
    pub fn depth_axis(self) -> usize {
        match self {
            ViewPlane::Top => 2,
            ViewPlane::Front => 1,
            ViewPlane::Side => 0,
        }
    }

    /// Returns the direction this view looks in.
    ///
    /// The projections drop one axis; this is the direction along that axis that points
    /// away from the viewer, so a ray cast along it travels into the scene.
    pub fn view_direction(self) -> Vec3d {
        // The directions `MapView2D::initializeCamera` gives the three views. The Side
        // view looks along negative X, so a click there picks the brush nearest the +X
        // side, which is what standing to the right of the map and looking left means.
        match self {
            ViewPlane::Top => Vec3d::new(0.0, 0.0, -1.0),
            ViewPlane::Front => Vec3d::new(0.0, 1.0, 0.0),
            ViewPlane::Side => Vec3d::new(-1.0, 0.0, 0.0),
        }
    }

    /// Returns the name shown in the view's title bar.
    pub fn name(self) -> &'static str {
        match self {
            ViewPlane::Top => translation_key("Top"),
            ViewPlane::Front => translation_key("Front"),
            ViewPlane::Side => translation_key("Side"),
        }
    }
}

/// The camera of a 2D map view.
///
/// The position is a world point, as `gl::Camera`'s is: the two components the view shows
/// are what it is centred on, and the third is the depth it looks from. Keeping the whole
/// point rather than only the two visible components is what lets
/// [`update_cameras`] be one formula for every view rather than a restatement per plane.
#[derive(Debug, Clone, Copy)]
pub struct Camera2d {
    /// The view plane this camera looks at.
    pub plane: ViewPlane,
    /// Where the camera is, in world space.
    pub position: Vec3d,
    /// Pixels per world unit.
    pub zoom: f64,
}

/// The smallest zoom level, i.e. the most zoomed out.
pub const MIN_ZOOM: f64 = 0.02;
/// The largest zoom level, i.e. the most zoomed in.
pub const MAX_ZOOM: f64 = 64.0;

impl Camera2d {
    /// Creates a camera looking at the origin of the given plane, from the edge of the
    /// world.
    ///
    /// The depth is taken from the world bounds so that everything in the map is in front
    /// of the camera, which is where `MapView2D::initializeCamera` puts it.
    pub fn new(plane: ViewPlane, world_half_extent: f64) -> Self {
        // Standing at the far edge along the axis the view looks down, facing across the
        // world.
        let depth = -plane.view_direction()[plane.depth_axis()] * world_half_extent;
        let mut position = Vec3d::zero();
        position[plane.depth_axis()] = depth;

        Self {
            plane,
            position,
            zoom: 0.25,
        }
    }

    /// Returns the world point the viewport is centred on, in the view's own plane.
    #[inline]
    pub fn center(&self) -> Vec2d {
        self.plane.project(self.position)
    }

    /// Moves the camera so the viewport is centred on the given point, leaving the depth
    /// it looks from alone.
    pub fn set_center(&mut self, center: Vec2d) {
        let shown = self.plane.shown_axes();
        self.position[shown[0]] = center.x();
        self.position[shown[1]] = center.y();
    }

    /// Returns the direction this camera looks in.
    #[inline]
    pub fn direction(&self) -> Vec3d {
        self.plane.view_direction()
    }

    /// Converts a world-space point to viewport pixel coordinates.
    ///
    /// The viewport's own Y axis grows downward, so the world Y is flipped.
    pub fn world_to_screen(&self, world: Vec3d, viewport: (f64, f64)) -> Vec2d {
        let p = self.plane.project(world);
        self.plane_to_screen(p, viewport)
    }

    /// Converts an already-projected point to viewport pixel coordinates.
    pub fn plane_to_screen(&self, p: Vec2d, viewport: (f64, f64)) -> Vec2d {
        let (w, h) = viewport;
        let center = self.center();
        Vec2d::new(
            w / 2.0 + (p.x() - center.x()) * self.zoom,
            h / 2.0 - (p.y() - center.y()) * self.zoom,
        )
    }

    /// Converts viewport pixel coordinates back to a point on the view plane.
    pub fn screen_to_plane(&self, screen: Vec2d, viewport: (f64, f64)) -> Vec2d {
        let (w, h) = viewport;
        let center = self.center();
        Vec2d::new(
            (screen.x() - w / 2.0) / self.zoom + center.x(),
            (h / 2.0 - screen.y()) / self.zoom + center.y(),
        )
    }

    /// Pans the camera by the given pixel delta.
    pub fn pan_by_pixels(&mut self, dx: f64, dy: f64) {
        let center = self.center();
        self.set_center(Vec2d::new(
            center.x() - dx / self.zoom,
            center.y() + dy / self.zoom,
        ));
    }

    /// Returns the ray a click at the given viewport position casts into the scene.
    ///
    /// A 2D view is orthographic, so the ray runs along the axis the view looks down,
    /// starting far enough back to be outside anything it might hit.
    pub fn ray_through(&self, screen: Vec2d, viewport: (f64, f64), reach: f64) -> geometry_math::ray::Ray3d {
        let on_plane = self.screen_to_plane(screen, viewport);
        let direction = self.plane.view_direction();
        // Starting a full reach behind the scene means everything in front of the viewer
        // is ahead of the ray, whatever its depth.
        let origin = self.plane.unproject(on_plane, 0.0) - direction * reach;
        geometry_math::ray::Ray3d::new(origin, direction)
    }

    /// Moves and zooms the camera so the given box fills the viewport, with a margin.
    ///
    /// Returns without changing anything if the box has no extent in this view's plane,
    /// which happens for a map that is flat when seen down this axis.
    pub fn frame(&mut self, bounds: &geometry_math::bbox::Bbox3d, viewport: (f64, f64)) {
        let min = self.plane.project(bounds.min);
        let max = self.plane.project(bounds.max);
        // Projecting min and max can put them either way round on an axis, since the view
        // planes do not all keep the sign of the axes they drop.
        let lower = vec::min(min, max);
        let upper = vec::max(min, max);

        let size = upper - lower;
        if size.x() <= 0.0 || size.y() <= 0.0 {
            return;
        }

        const MARGIN: f64 = 0.9;
        let (w, h) = viewport;
        let zoom = (w / size.x()).min(h / size.y()) * MARGIN;

        self.set_center((lower + upper) / 2.0);
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Zooms by the given factor, keeping the world point under `anchor` fixed.
    pub fn zoom_at(&mut self, factor: f64, anchor: Vec2d, viewport: (f64, f64)) {
        let before = self.screen_to_plane(anchor, viewport);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.screen_to_plane(anchor, viewport);
        self.set_center(self.center() + before - after);
    }
}

/// Brings the other cameras into line with the one that moved.
///
/// Shares the master's position on axes visible in both views, retaining the other coordinates. The
/// zoom is shared on every axis.
pub fn update_cameras(master: &Camera2d, others: &mut [&mut Camera2d]) {
    let one = Vec3d::one();

    for camera in others {
        if std::ptr::eq(*camera as *const Camera2d, master as *const Camera2d) {
            continue;
        }

        camera.zoom = master.zoom;

        let old_position = camera.position;
        let factors = one - vec::abs(master.direction()) - vec::abs(camera.direction());
        camera.position = (one - factors) * old_position + factors * master.position;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_math::assert_approx_eq;

    const VIEWPORT: (f64, f64) = (800.0, 600.0);

    /// The half-extent the tests build their cameras against.
    const WORLD: f64 = 16384.0;

    fn camera(plane: ViewPlane) -> Camera2d {
        Camera2d::new(plane, WORLD)
    }

    #[test]
    fn the_view_center_maps_to_the_middle_of_the_viewport() {
        let camera = camera(ViewPlane::Top);
        assert_eq!(
            camera.world_to_screen(Vec3d::zero(), VIEWPORT),
            Vec2d::new(400.0, 300.0)
        );
    }

    #[test]
    fn world_y_grows_upward_on_screen() {
        let camera = camera(ViewPlane::Top);
        let above = camera.world_to_screen(Vec3d::new(0.0, 100.0, 0.0), VIEWPORT);
        assert!(above.y() < 300.0);
    }

    #[test]
    fn screen_to_plane_inverts_plane_to_screen() {
        let mut camera = camera(ViewPlane::Front);
        camera.set_center(Vec2d::new(-37.0, 91.0));
        camera.zoom = 1.75;

        let world = Vec2d::new(123.0, -45.0);
        let screen = camera.plane_to_screen(world, VIEWPORT);
        assert_approx_eq!(camera.screen_to_plane(screen, VIEWPORT), world);
    }

    #[test]
    fn zooming_keeps_the_anchored_world_point_under_the_cursor() {
        let mut camera = camera(ViewPlane::Top);
        let anchor = Vec2d::new(120.0, 480.0);
        let before = camera.screen_to_plane(anchor, VIEWPORT);

        camera.zoom_at(1.6, anchor, VIEWPORT);

        assert_approx_eq!(camera.screen_to_plane(anchor, VIEWPORT), before);
    }

    #[test]
    fn a_click_casts_a_ray_into_the_scene() {
        let camera = camera(ViewPlane::Top);
        // The middle of the viewport looks at the camera's centre, which is the origin.
        let ray = camera.ray_through(Vec2d::new(400.0, 300.0), VIEWPORT, 1000.0);

        assert_approx_eq!(ray.direction, Vec3d::new(0.0, 0.0, -1.0));
        // Starting above everything, so a point at any depth is ahead of it.
        assert_approx_eq!(ray.origin, Vec3d::new(0.0, 0.0, 1000.0));
    }

    #[test]
    fn a_ray_passes_through_the_world_point_that_was_clicked() {
        use geometry_math::abstract_line::AbstractLine;

        for plane in [ViewPlane::Top, ViewPlane::Front, ViewPlane::Side] {
            let camera = camera(plane);
            let screen = Vec2d::new(120.0, 480.0);
            let ray = camera.ray_through(screen, VIEWPORT, 1000.0);

            // Wherever the ray is along its length, it projects back onto the click.
            let on_ray = ray.point_at_distance(1000.0);
            assert_approx_eq!(
                camera.world_to_screen(on_ray, VIEWPORT),
                screen,
                1e-9
            );
        }
    }

    #[test]
    fn framing_a_box_centres_and_fits_it() {
        use geometry_math::bbox::Bbox3d;

        let mut camera = camera(ViewPlane::Top);
        let bounds = Bbox3d::new(
            Vec3d::new(-100.0, -50.0, -10.0),
            Vec3d::new(100.0, 50.0, 10.0),
        );
        camera.frame(&bounds, VIEWPORT);

        // The box's centre is now at the centre of the viewport.
        assert_approx_eq!(
            camera.world_to_screen(bounds.center(), VIEWPORT),
            Vec2d::new(400.0, 300.0)
        );
        // And its corners are on screen.
        for corner in bounds.vertices() {
            let screen = camera.world_to_screen(corner, VIEWPORT);
            assert!(screen.x() >= 0.0 && screen.x() <= VIEWPORT.0, "{screen:?}");
            assert!(screen.y() >= 0.0 && screen.y() <= VIEWPORT.1, "{screen:?}");
        }
    }

    #[test]
    fn framing_a_box_that_is_flat_in_this_view_changes_nothing() {
        use geometry_math::bbox::Bbox3d;

        let mut camera = camera(ViewPlane::Top);
        let before = camera.zoom;
        // A box with no extent in x or y, seen from the top.
        camera.frame(
            &Bbox3d::new(Vec3d::zero(), Vec3d::new(0.0, 0.0, 100.0)),
            VIEWPORT,
        );
        assert_eq!(camera.zoom, before);
    }

    #[test]
    fn zoom_is_clamped() {
        let mut camera = camera(ViewPlane::Top);
        let anchor = Vec2d::new(400.0, 300.0);

        camera.zoom_at(1e9, anchor, VIEWPORT);
        assert_eq!(camera.zoom, MAX_ZOOM);

        camera.zoom_at(1e-9, anchor, VIEWPORT);
        assert_eq!(camera.zoom, MIN_ZOOM);
    }

    #[test]
    fn linking_lines_up_the_axis_two_views_share() {
        // Top and Front both show X, so panning Top sideways must pan Front sideways too.
        let mut top = camera(ViewPlane::Top);
        top.set_center(Vec2d::new(100.0, 200.0));
        top.zoom = 2.0;

        let mut front = camera(ViewPlane::Front);
        front.set_center(Vec2d::new(0.0, 50.0));
        update_cameras(&top, &mut [&mut front]);

        // X came from Top; Z is Front's own and is left alone.
        assert_eq!(front.center(), Vec2d::new(100.0, 50.0));
        assert_eq!(front.zoom, 2.0);
    }

    #[test]
    fn linking_matches_axes_rather_than_screen_positions() {
        // Top's vertical axis is Y, which is Side's *horizontal* axis. Linking follows the
        // axis, not the position on screen.
        let mut top = camera(ViewPlane::Top);
        top.set_center(Vec2d::new(100.0, 200.0));

        let mut side = camera(ViewPlane::Side);
        side.set_center(Vec2d::new(0.0, 50.0));
        update_cameras(&top, &mut [&mut side]);

        assert_eq!(side.center(), Vec2d::new(200.0, 50.0));
    }

    #[test]
    fn front_and_side_share_the_vertical_axis() {
        let mut front = camera(ViewPlane::Front);
        front.set_center(Vec2d::new(10.0, 300.0));

        let mut side = camera(ViewPlane::Side);
        side.set_center(Vec2d::new(20.0, 0.0));
        update_cameras(&front, &mut [&mut side]);

        assert_eq!(side.center(), Vec2d::new(20.0, 300.0));
    }

    #[test]
    fn linking_leaves_the_depth_a_view_looks_from_alone() {
        // The master cannot speak for the axis it looks down, so the other keeps its own.
        let mut top = camera(ViewPlane::Top);
        top.set_center(Vec2d::new(50.0, 60.0));
        let depth_before = camera(ViewPlane::Front).position.y();

        let mut front = camera(ViewPlane::Front);
        update_cameras(&top, &mut [&mut front]);

        assert_eq!(front.position.y(), depth_before);
    }

    #[test]
    fn linking_all_three_leaves_every_shared_axis_agreeing() {
        let mut top = camera(ViewPlane::Top);
        top.set_center(Vec2d::new(-64.0, 128.0));
        top.zoom = 0.75;

        let mut front = camera(ViewPlane::Front);
        let mut side = camera(ViewPlane::Side);
        update_cameras(&top, &mut [&mut front, &mut side]);

        // Every axis two views both show now holds the same value.
        for (a, b) in [(&top, &front), (&top, &side), (&front, &side)] {
            for axis in 0..3 {
                let shows = |c: &Camera2d| c.plane.shown_axes().contains(&axis);
                if shows(a) && shows(b) {
                    assert_approx_eq!(a.position[axis], b.position[axis], 1e-9);
                }
            }
            assert_eq!(a.zoom, b.zoom);
        }
    }

    #[test]
    fn the_depth_axis_is_the_one_the_view_drops() {
        // Projecting loses exactly the depth axis, so changing only that component must
        // leave the projection unchanged.
        for plane in [ViewPlane::Top, ViewPlane::Front, ViewPlane::Side] {
            let mut moved = Vec3d::new(1.0, 2.0, 3.0);
            let base = plane.project(moved);
            moved[plane.depth_axis()] += 100.0;
            assert_eq!(plane.project(moved), base, "{plane:?}");
        }
    }

    #[test]
    fn each_view_plane_drops_the_axis_it_looks_down() {
        let p = Vec3d::new(1.0, 2.0, 3.0);
        assert_eq!(ViewPlane::Top.project(p), Vec2d::new(1.0, 2.0));
        assert_eq!(ViewPlane::Front.project(p), Vec2d::new(1.0, 3.0));
        assert_eq!(ViewPlane::Side.project(p), Vec2d::new(2.0, 3.0));
    }

    #[test]
    fn unproject_restores_the_axis_the_view_looks_down() {
        for plane in [ViewPlane::Top, ViewPlane::Front, ViewPlane::Side] {
            let p = Vec3d::new(1.0, 2.0, 3.0);
            let round_tripped = plane.unproject(plane.project(p), 9.0);
            assert_eq!(plane.project(round_tripped), plane.project(p));
        }
    }
}
