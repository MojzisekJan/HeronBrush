//! Pose handles in the views: in 3D and, through the core's `animation_drag2d`, in the
//! flat views. A drag edits entity properties in one transaction, opened only once the
//! pointer has travelled a few pixels, so a click never authors a key.
use crate::egui_canvas::EguiCanvas;
use editor_core::{
    animation_drag2d::{self as drag2d, Turn},
    animation_pose::Pose,
    camera2d::Camera2d,
    camera3d::Camera3d,
    document::Document,
    grid::Grid,
    tool::Tool,
    view_filters::ViewFilters,
};
use eframe::egui;
use geometry_math::vec::{Vec2d, Vec3d};
use std::{cell::RefCell, rc::Rc};

/// What the views tell the animation panel: a key picked on a path, and a time to seek to.
#[derive(Default)]
pub struct Link {
    /// A motion-path key was clicked at this time: select its keys and seek there.
    pub select_key: Option<f64>,
    /// A view press moved the playhead (frame, fps): stop playback and follow it.
    pub seek: Option<(u32, u32)>,
    /// A pose drag is in progress in some view, so Escape belongs to it.
    pub dragging: bool,
}

#[derive(Default)]
pub struct Gizmo {
    drag: Option<Drag>,
    link: Rc<RefCell<Link>>,
}
enum Kind {
    /// A 3D axis arrow: `vector` is the arrow on screen, `units` world units per pixel along it.
    Axis {
        channel: usize,
        vector: egui::Vec2,
        units: f64,
    },
    /// A 3D rotation ring.
    Ring {
        channel: usize,
        ring: render::rotation_gizmo::RingDrag,
        camera: Camera3d,
        rect: egui::Rect,
    },
    /// A motion-path key in 3D, moved in the screen plane.
    Path { axes: [Vec3d; 2] },
    /// A flat view: the body translated, turned about the centre, the pivot, or a path key.
    Flat {
        camera: Camera2d,
        turn: Option<Turn>,
        path_key: bool,
    },
}
struct Drag {
    document: Rc<RefCell<Document>>,
    pose: Pose,
    kind: Kind,
    pivot: bool,
    start: egui::Pos2,
    changed: bool,
    frame: (Option<u32>, u32),
    selection: editor_core::document::SelectionSnapshot,
}
const COLORS: [egui::Color32; 3] = [
    egui::Color32::from_rgb(255, 64, 64),
    egui::Color32::from_rgb(64, 255, 64),
    egui::Color32::from_rgb(77, 140, 255),
];
const PIVOT: egui::Color32 = egui::Color32::from_rgb(255, 204, 26);

impl Gizmo {
    /// A gizmo that reports path-key clicks and seeks through `link`.
    pub fn linked(link: &Rc<RefCell<Link>>) -> Self {
        Self {
            drag: None,
            link: Rc::clone(link),
        }
    }
    pub fn cancel(&mut self) {
        if let Some(drag) = self.drag.take() {
            if drag.changed {
                drag.document.borrow_mut().cancel_transaction();
            }
            self.link.borrow_mut().dragging = false;
        }
    }
    fn finish(&mut self) {
        if let Some(drag) = self.drag.take() {
            if drag.changed {
                drag.document.borrow_mut().commit_entity_transaction();
            }
            self.link.borrow_mut().dragging = false;
        }
    }
    /// Cancels a drag the frame, the selection or the window focus moved away from.
    fn check(&mut self, ui: &egui::Ui, document: &Rc<RefCell<Document>>, f: &ViewFilters) {
        if let Some(drag) = &self.drag {
            let cancel = ui.input(|i| {
                i.key_pressed(egui::Key::Escape)
                    || ((i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::Z))
                    || !i.focused
            }) || drag.frame != (f.animation_frame, f.animation_fps)
                || drag.selection != document.borrow().selection_snapshot();
            if cancel {
                self.cancel();
            }
        }
    }
    fn begin(
        &mut self,
        document: &Rc<RefCell<Document>>,
        pose: Pose,
        kind: Kind,
        pivot: bool,
        start: egui::Pos2,
        filters: &Rc<RefCell<ViewFilters>>,
    ) {
        let f = *filters.borrow();
        self.drag = Some(Drag {
            document: Rc::clone(document),
            pose,
            kind,
            pivot,
            start,
            changed: false,
            frame: (f.animation_frame, f.animation_fps),
            selection: document.borrow().selection_snapshot(),
        });
        self.link.borrow_mut().dragging = true;
    }
    /// A press on a motion-path key: the playhead jumps there and the panel selects it.
    fn path_key(&self, filters: &Rc<RefCell<ViewFilters>>, pose: &Pose, time: f64) {
        let mut f = filters.borrow_mut();
        f.animation_fps = pose.clip.fps;
        f.animation_frame = Some((time * pose.clip.fps as f64).round() as u32);
        self.link.borrow_mut().select_key = Some(time);
    }

    /// The 3D view: mode badge, motion path, and with Auto key the axis or ring handles.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        document: &Rc<RefCell<Document>>,
        filters: &Rc<RefCell<ViewFilters>>,
        grid: &Rc<RefCell<Grid>>,
        camera: Camera3d,
    ) {
        let f = *filters.borrow();
        self.check(ui, document, &f);
        let Some(frame) = f.animation_frame else {
            return;
        };
        let (w, h) = (rect.width() as f64, rect.height() as f64);
        let project = |p: Vec3d| camera.project(p, w, h);
        let screen = |p: Vec2d| rect.min + egui::vec2(p.x() as f32, p.y() as f32);
        let time = frame as f64 / f.animation_fps.max(1) as f64;
        let painter = ui.painter_at(rect);
        editor_core::animation_mode::draw(&mut EguiCanvas::new(&painter, rect), &f, h);
        let Ok(pose) = Pose::selected(&document.borrow(), time) else {
            return;
        };
        let path = document.borrow().animation_path(&pose);
        if let Some(path) = path.as_ref() {
            path.draw(
                &mut EguiCanvas::new(&painter, rect),
                time,
                f.animation_fps,
                project,
            );
        }
        if !f.animation_auto_key {
            return;
        }
        let centre = pose.centre();
        let radius = camera.world_units_per_pixel(centre, h) * 72.0;
        let Some(start) = project(centre) else {
            return;
        };
        let rotate = f.animation_rotate && !f.animation_edit_pivot;
        let axis = |i: usize| {
            let mut v = Vec3d::zero();
            v[i] = 1.0;
            v
        };
        for i in 0..3 {
            let points: Vec<egui::Pos2> = if rotate {
                let (u, v) = ((i + 1) % 3, (i + 2) % 3);
                (0..=96)
                    .filter_map(|n| {
                        let a = n as f64 * std::f64::consts::TAU / 96.0;
                        project(centre + (axis(u) * a.cos() + axis(v) * a.sin()) * radius)
                            .map(screen)
                    })
                    .collect()
            } else {
                project(centre + axis(i) * radius)
                    .map(|end| vec![screen(start), screen(end)])
                    .unwrap_or_default()
            };
            if points.len() < 2 {
                continue;
            }
            painter.add(egui::Shape::line(
                points.clone(),
                egui::Stroke::new(6.0, egui::Color32::from_black_alpha(204)),
            ));
            painter.add(egui::Shape::line(
                points.clone(),
                egui::Stroke::new(2.5, COLORS[i]),
            ));
            if !rotate {
                painter.circle_filled(*points.last().unwrap(), 5.0, COLORS[i]);
            }
        }
        if f.animation_edit_pivot {
            painter.rect_filled(
                egui::Rect::from_center_size(screen(start), egui::vec2(10.0, 10.0)),
                0.0,
                PIVOT,
            );
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        if self.drag.is_none()
            && ui.input(|i| i.pointer.primary_pressed())
            && ui.rect_contains_pointer(rect)
        {
            if let Some(at) = ui.input(|i| i.pointer.press_origin()).or(pointer) {
                let local = Vec2d::new((at.x - rect.left()) as f64, (at.y - rect.top()) as f64);
                let path_time = if f.animation_edit_pivot {
                    None
                } else {
                    path.as_ref()
                        .as_ref()
                        .and_then(|path| path.pick(local, time, project))
                };
                if let Some(key_time) = path_time {
                    let mut pose = pose.clone();
                    pose.time = key_time;
                    let units = camera.world_units_per_pixel(pose.centre(), h);
                    self.path_key(filters, &pose, key_time);
                    self.begin(
                        document,
                        pose,
                        Kind::Path {
                            axes: [camera.right() * units, camera.up() * -units],
                        },
                        false,
                        at,
                        filters,
                    );
                } else if rotate {
                    let ray = camera.ray_through(local.x(), local.y(), w, h);
                    if let Some(ring) = render::rotation_gizmo::RingDrag::pick(
                        &camera, h, &ray, centre, radius, 8.0,
                    ) {
                        let channel = 3
                            + (0..3)
                                .max_by(|&a, &b| ring.axis[a].abs().total_cmp(&ring.axis[b].abs()))
                                .unwrap();
                        self.begin(
                            document,
                            pose.clone(),
                            Kind::Ring {
                                channel,
                                ring,
                                camera,
                                rect,
                            },
                            false,
                            at,
                            filters,
                        );
                    }
                } else {
                    let picked = (0..3)
                        .filter_map(|i| {
                            let end = project(centre + axis(i) * radius)?;
                            let v = end - start;
                            let len2 = v.x() * v.x() + v.y() * v.y();
                            if len2 < 100.0 {
                                return None;
                            }
                            let rel = local - start;
                            let t = ((rel.x() * v.x() + rel.y() * v.y()) / len2).clamp(0.15, 1.0);
                            let p = start + v * t;
                            let distance =
                                ((local - p).x().powi(2) + (local - p).y().powi(2)).sqrt();
                            (distance < 9.0).then_some((distance, i, v, radius / len2))
                        })
                        .min_by(|a, b| a.0.total_cmp(&b.0));
                    if let Some((_, channel, v, units)) = picked {
                        let kind = Kind::Axis {
                            channel,
                            vector: egui::vec2(v.x() as f32, v.y() as f32),
                            units,
                        };
                        self.begin(
                            document,
                            pose.clone(),
                            kind,
                            f.animation_edit_pivot,
                            at,
                            filters,
                        );
                    }
                }
            }
        }
        self.update(ui, grid, pointer);
    }

    /// A flat view: drag the animated body to move it, turn it about its centre with the
    /// Rotate tool or Rotate pose mode, drag the pivot from the centre, or drag a
    /// motion-path key. The view itself draws the badge, path and pivot marker.
    #[allow(clippy::too_many_arguments)]
    pub fn show_2d(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        document: &Rc<RefCell<Document>>,
        filters: &Rc<RefCell<ViewFilters>>,
        grid: &Rc<RefCell<Grid>>,
        tool: Tool,
        camera: Camera2d,
    ) {
        let f = *filters.borrow();
        self.check(ui, document, &f);
        let Some(frame) = f.animation_frame else {
            return;
        };
        let time = frame as f64 / f.animation_fps.max(1) as f64;
        let viewport = (rect.width() as f64, rect.height() as f64);
        let pointer = ui.input(|i| i.pointer.interact_pos());
        if self.drag.is_none()
            && ui.input(|i| i.pointer.primary_pressed())
            && ui.rect_contains_pointer(rect)
            && !document.borrow().in_transaction()
        {
            if let Some(at) = ui.input(|i| i.pointer.press_origin()).or(pointer) {
                let cursor = Vec2d::new((at.x - rect.left()) as f64, (at.y - rect.top()) as f64);
                let d = document.borrow();
                if let Ok(mut pose) = Pose::selected(&d, time) {
                    let p = camera.screen_to_plane(cursor, viewport);
                    let edit_pivot = f.animation_edit_pivot;
                    let path_time = if edit_pivot {
                        None
                    } else {
                        d.animation_path(&pose).as_ref().as_ref().and_then(|path| {
                            path.pick(cursor, time, |p| Some(camera.world_to_screen(p, viewport)))
                        })
                    };
                    let grabbed = if edit_pivot {
                        let c = camera.world_to_screen(pose.centre(), viewport);
                        (c.x() - cursor.x()).powi(2) + (c.y() - cursor.y()).powi(2)
                            <= drag2d::PIVOT_GRAB.powi(2)
                    } else if let Some(key_time) = path_time {
                        pose.time = key_time;
                        true
                    } else {
                        drag2d::hit(&d, &pose, camera, p)
                    };
                    let turn = if grabbed
                        && !edit_pivot
                        && path_time.is_none()
                        && (tool == Tool::Rotate || f.animation_rotate)
                    {
                        let start = p - camera.plane.project(pose.centre());
                        if start.x() * start.x() + start.y() * start.y()
                            < (2.0 / camera.zoom).powi(2)
                        {
                            None
                        } else {
                            Some(Some(Turn::new(start)))
                        }
                    } else {
                        Some(None)
                    };
                    drop(d);
                    if let (true, Some(turn)) = (grabbed, turn) {
                        if let Some(key_time) = path_time {
                            self.path_key(filters, &pose, key_time);
                        } else {
                            self.link.borrow_mut().seek = Some((frame, f.animation_fps));
                        }
                        let kind = Kind::Flat {
                            camera,
                            turn,
                            path_key: path_time.is_some(),
                        };
                        self.begin(document, pose, kind, edit_pivot, at, filters);
                    }
                }
            }
        }
        self.update(ui, grid, pointer);
    }

    fn update(&mut self, ui: &egui::Ui, grid: &Rc<RefCell<Grid>>, pointer: Option<egui::Pos2>) {
        let released = ui.input(|i| i.pointer.primary_released() || !i.pointer.primary_down());
        if let (Some(drag), Some(at)) = (&mut self.drag, pointer) {
            let delta = at - drag.start;
            if drag.changed || delta.length() as f64 >= drag2d::DRAG_THRESHOLD {
                apply(drag, delta, at, &grid.borrow());
                ui.ctx().request_repaint();
            }
        }
        if self.drag.is_some() && released {
            self.finish();
        }
    }
}

/// Authors the pose for a pointer `delta` pixels from the press.
fn apply(drag: &mut Drag, delta: egui::Vec2, at: egui::Pos2, grid: &Grid) {
    let (dx, dy) = (delta.x as f64, delta.y as f64);
    enum Edit {
        Clip(editor_core::animation_curves::Clip),
        Pivot(Vec3d),
    }
    let edit = match &mut drag.kind {
        Kind::Path { axes } => {
            let centre = drag.pose.centre();
            let mut point = centre + axes[0] * dx + axes[1] * dy;
            for axis in 0..3 {
                point[axis] = grid.snap_scalar(point[axis]);
            }
            let values = std::array::from_fn(|axis| {
                drag.pose.clip.sample(axis, drag.pose.time) + point[axis] - centre[axis]
            });
            let mut clip = drag.pose.clip.clone();
            if clip.insert_position(drag.pose.time, values).is_err() {
                return;
            }
            Edit::Clip(clip)
        }
        Kind::Axis {
            channel,
            vector,
            units,
        } => {
            let amount = grid.snap_scalar((dx * vector.x as f64 + dy * vector.y as f64) * *units);
            if !drag.changed && amount.abs() < 1e-6 {
                return;
            }
            if drag.pivot {
                match drag.pose.moved_pivot(*channel, amount) {
                    Ok(pivot) => Edit::Pivot(pivot),
                    Err(_) => return,
                }
            } else {
                match drag.pose.keyed(*channel, amount) {
                    Ok(clip) => Edit::Clip(clip),
                    Err(_) => return,
                }
            }
        }
        Kind::Ring {
            channel,
            ring,
            camera,
            rect,
        } => {
            let ray = camera.ray_through(
                (at.x - rect.left()) as f64,
                (at.y - rect.top()) as f64,
                rect.width() as f64,
                rect.height() as f64,
            );
            let Some(angle) = ring.angle(&ray) else {
                return;
            };
            let angle = grid.snap_angle(angle).to_degrees();
            if !drag.changed && angle.abs() < 1e-6 {
                return;
            }
            match drag.pose.keyed(*channel, angle) {
                Ok(clip) => Edit::Clip(clip),
                Err(_) => return,
            }
        }
        Kind::Flat {
            camera,
            turn,
            path_key,
        } => {
            let screen_delta = Vec2d::new(dx / camera.zoom, -dy / camera.zoom);
            let plane = camera.plane;
            if drag.pivot {
                let pivot = drag2d::snapped_pivot(&drag.pose, plane, screen_delta, grid);
                if Pose::validate_pivot(pivot).is_err()
                    || (!drag.changed && pivot == drag.pose.pivot)
                {
                    return;
                }
                Edit::Pivot(pivot)
            } else if let Some(turn) = turn {
                let Some(angle) = turn.update(screen_delta) else {
                    return;
                };
                let angle = grid.snap_angle(angle).to_degrees() * drag2d::rotation_sign(plane);
                if !drag.changed && angle.abs() < 1e-10 {
                    return;
                }
                match drag.pose.keyed(plane.depth_axis() + 3, angle) {
                    Ok(clip) => Edit::Clip(clip),
                    Err(_) => return,
                }
            } else {
                let delta = if *path_key {
                    drag2d::snapped_position(drag.pose.centre(), plane, screen_delta, grid)
                        - drag.pose.centre()
                } else {
                    let d = plane.unproject(screen_delta, 0.0);
                    Vec3d::new(
                        grid.snap_scalar(d.x()),
                        grid.snap_scalar(d.y()),
                        grid.snap_scalar(d.z()),
                    )
                };
                if !drag.changed && delta == Vec3d::zero() {
                    return;
                }
                match drag2d::translated(&drag.pose, delta) {
                    Ok(clip) => Edit::Clip(clip),
                    Err(_) => return,
                }
            }
        }
    };
    let mut d = drag.document.borrow_mut();
    if !drag.changed {
        if !d.begin_entity_transaction(drag.pose.entity) {
            return;
        }
        drag.changed = true;
    }
    match edit {
        Edit::Clip(clip) => drag.pose.write(&mut d, &clip),
        Edit::Pivot(pivot) => drag.pose.write_pivot(&mut d, pivot),
    }
}

impl Drop for Gizmo {
    fn drop(&mut self) {
        self.cancel();
    }
}
